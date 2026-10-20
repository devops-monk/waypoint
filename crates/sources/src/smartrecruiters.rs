//! SmartRecruiters public postings API (no auth). Reference: career-ops
//! `providers/smartrecruiters.mjs`. The listing has no descriptions; they are fetched per
//! job when you open it.

use serde::Deserialize;

use crate::http::Http;
use crate::text::{html_to_text, parse_date};
use crate::{SourceError, slug_ok};
use waypoint_core::JobDraft;

pub const HOSTS: &[&str] = &["api.smartrecruiters.com"];
const PAGE: usize = 100;
/// 500 newest postings per scan; some employers list thousands.
const MAX_PAGES: usize = 5;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Page {
    #[serde(default)]
    total_found: usize,
    #[serde(default)]
    content: Vec<Posting>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Posting {
    id: Option<String>,
    name: Option<String>,
    ref_number: Option<String>,
    released_date: Option<String>,
    location: Option<Location>,
    company: Option<Company>,
    department: Option<Named>,
    type_of_employment: Option<Named>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Location {
    full_location: Option<String>,
    city: Option<String>,
    country: Option<String>,
    #[serde(default)]
    remote: bool,
}

#[derive(Deserialize)]
struct Company {
    identifier: Option<String>,
}

#[derive(Deserialize)]
struct Named {
    label: Option<String>,
}

pub async fn fetch(http: &Http, slug: &str, company: &str) -> Result<Vec<JobDraft>, SourceError> {
    slug_ok(slug)?;
    let mut out = Vec::new();
    for page in 0..MAX_PAGES {
        let url = format!("https://api.smartrecruiters.com/v1/companies/{slug}/postings?limit={PAGE}&offset={}", page * PAGE);
        let p: Page = http.get_json(&url, HOSTS).await?;
        let n = p.content.len();
        out.extend(p.content.into_iter().filter_map(|j| map(j, slug, company)));
        if n < PAGE || (page + 1) * PAGE >= p.total_found {
            break;
        }
    }
    Ok(out)
}

fn url_slug(title: &str) -> String {
    let s: String = title.to_lowercase().chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '-' }).collect();
    s.split('-').filter(|p| !p.is_empty()).collect::<Vec<_>>().join("-")
}

fn map(j: Posting, slug: &str, company: &str) -> Option<JobDraft> {
    let id = j.id.filter(|i| i.chars().all(|c| c.is_ascii_alphanumeric()))?;
    let title = j.name.unwrap_or_default();
    let identifier = j.company.and_then(|c| c.identifier).filter(|i| slug_ok(i).is_ok()).unwrap_or_else(|| slug.into());
    let (location, remote) = match j.location {
        Some(l) => {
            let place = l.full_location.filter(|s| !s.is_empty()).unwrap_or_else(|| [l.city, l.country.map(|c| c.to_uppercase())].into_iter().flatten().collect::<Vec<_>>().join(", "));
            (if l.remote { if place.is_empty() { "Remote".into() } else { format!("{place} · Remote") } } else { place }, l.remote)
        }
        None => (String::new(), false),
    };
    Some(JobDraft {
        source: "smartrecruiters".into(),
        board: Some(slug.into()),
        external_id: j.ref_number.or(Some(id.clone())),
        url: format!("https://jobs.smartrecruiters.com/{identifier}/{id}-{}", url_slug(&title)),
        company: company.into(),
        title,
        location,
        remote,
        employment_type: j.type_of_employment.and_then(|t| t.label),
        department: j.department.and_then(|d| d.label),
        description: String::new(),
        salary: None,
        posted_at: j.released_date.as_deref().and_then(parse_date),
    })
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Detail {
    job_ad: Option<JobAd>,
}

#[derive(Deserialize)]
struct JobAd {
    sections: Option<serde_json::Value>,
}

pub async fn fetch_description(http: &Http, slug: &str, url: &str) -> Result<Option<String>, SourceError> {
    slug_ok(slug)?;
    // https://jobs.smartrecruiters.com/{identifier}/{id}-{title}
    let Some(id) = url.rsplit('/').next().and_then(|last| last.split('-').next()).filter(|id| !id.is_empty() && id.chars().all(|c| c.is_ascii_alphanumeric())) else {
        return Ok(None);
    };
    let d: Detail = http.get_json(&format!("https://api.smartrecruiters.com/v1/companies/{slug}/postings/{id}"), HOSTS).await?;
    let Some(sections) = d.job_ad.and_then(|a| a.sections) else { return Ok(None) };
    let parts: Vec<String> = ["companyDescription", "jobDescription", "qualifications", "additionalInformation"]
        .iter()
        .filter_map(|k| sections[k]["text"].as_str())
        .map(html_to_text)
        .filter(|t| !t.is_empty())
        .collect();
    Ok(Some(parts.join("\n\n")).filter(|t| !t.is_empty()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_postings_to_public_urls() {
        let raw = r#"{"totalFound": 2, "content": [
          {"id": "744000156179239", "name": "Application Engineer (DCCS)", "refNumber": "REF1", "releasedDate": "2026-10-18T07:28:14.846Z",
           "company": {"identifier": "BoschGroup"}, "location": {"city": "Shanghai", "country": "cn", "fullLocation": "Shanghai, China", "remote": false},
           "typeOfEmployment": {"label": "Full-time"}},
          {"id": "../bad", "name": "x"}
        ]}"#;
        let p: Page = serde_json::from_str(raw).unwrap();
        let jobs: Vec<_> = p.content.into_iter().filter_map(|j| map(j, "boschgroup", "Bosch")).collect();
        assert_eq!(jobs.len(), 1);
        assert_eq!(jobs[0].url, "https://jobs.smartrecruiters.com/BoschGroup/744000156179239-application-engineer-dccs");
        assert_eq!(jobs[0].location, "Shanghai, China");
        assert!(jobs[0].posted_at.is_some());
    }
}
