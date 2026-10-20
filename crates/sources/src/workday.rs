//! Workday career sites, through the same JSON endpoint the site's own search page calls.
//! Reference: career-ops `providers/workday.mjs`.
//!
//! The board slug is `tenant.instance.site`, e.g. `nvidia.wd5.NVIDIAExternalCareerSite`.
//! Large employers list thousands of roles, so a scan reads the newest pages only, and the
//! listing has no descriptions: those are fetched one job at a time when you open it.

use chrono::{Duration, Utc};
use serde::Deserialize;
use serde_json::json;

use crate::http::Http;
use crate::text::{html_to_text, looks_remote};
use crate::{SourceError, slug_ok};
use waypoint_core::JobDraft;

pub const HOSTS: &[&str] = &["*.*.myworkdayjobs.com"];
const PAGE: usize = 20;
/// 200 newest postings per scan.
const MAX_PAGES: usize = 10;

pub struct Site<'a> {
    tenant: &'a str,
    instance: &'a str,
    site: &'a str,
}

impl<'a> Site<'a> {
    pub fn parse(slug: &'a str) -> Result<Self, SourceError> {
        slug_ok(slug)?;
        let mut parts = slug.splitn(3, '.');
        match (parts.next(), parts.next(), parts.next()) {
            (Some(tenant), Some(instance), Some(site)) if instance.starts_with("wd") && !tenant.is_empty() && !site.is_empty() && !site.contains('.') => Ok(Site { tenant, instance, site }),
            _ => Err(SourceError::BadSlug(slug.into())),
        }
    }

    fn origin(&self) -> String {
        format!("https://{}.{}.myworkdayjobs.com", self.tenant, self.instance)
    }

    fn cxs(&self) -> String {
        format!("{}/wday/cxs/{}/{}", self.origin(), self.tenant, self.site)
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Page {
    #[serde(default)]
    total: usize,
    #[serde(default)]
    job_postings: Vec<Posting>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Posting {
    title: Option<String>,
    external_path: Option<String>,
    locations_text: Option<String>,
    posted_on: Option<String>,
    #[serde(default)]
    bullet_fields: Vec<String>,
}

pub async fn fetch(http: &Http, slug: &str, company: &str) -> Result<Vec<JobDraft>, SourceError> {
    let site = Site::parse(slug)?;
    let mut out = Vec::new();
    // Workday reports the total on the first page only; later pages say 0.
    let mut total = usize::MAX;
    for page in 0..MAX_PAGES {
        let body = json!({ "appliedFacets": {}, "limit": PAGE, "offset": page * PAGE, "searchText": "" });
        let p: Page = http.post_json(&format!("{}/jobs", site.cxs()), HOSTS, &body).await?;
        if page == 0 {
            total = p.total;
        }
        let n = p.job_postings.len();
        out.extend(p.job_postings.into_iter().filter_map(|j| map(j, &site, slug, company)));
        if n < PAGE || (page + 1) * PAGE >= total {
            break;
        }
    }
    Ok(out)
}

fn map(j: Posting, site: &Site, slug: &str, company: &str) -> Option<JobDraft> {
    let path = j.external_path.filter(|p| p.starts_with("/job/"))?;
    let location = j.locations_text.unwrap_or_default();
    Some(JobDraft {
        source: "workday".into(),
        board: Some(slug.into()),
        external_id: j.bullet_fields.into_iter().next(),
        url: format!("{}/{}{}", site.origin(), site.site, path),
        company: company.into(),
        title: j.title.unwrap_or_default(),
        remote: looks_remote(&location),
        location,
        employment_type: None,
        department: None,
        description: String::new(),
        salary: None,
        posted_at: j.posted_on.as_deref().and_then(posted_on),
    })
}

/// "Posted Today", "Posted Yesterday", "Posted 3 Days Ago", "Posted 30+ Days Ago".
fn posted_on(s: &str) -> Option<chrono::DateTime<Utc>> {
    let s = s.to_lowercase();
    let days = if s.contains("today") {
        0
    } else if s.contains("yesterday") {
        1
    } else {
        s.split_whitespace().find_map(|w| w.trim_end_matches('+').parse::<i64>().ok())?
    };
    Some(Utc::now() - Duration::days(days))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Detail {
    job_posting_info: DetailInfo,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct DetailInfo {
    job_description: Option<String>,
    time_type: Option<String>,
}

/// The posting's description, read from the job page's own JSON.
pub async fn fetch_description(http: &Http, slug: &str, url: &str) -> Result<Option<String>, SourceError> {
    let site = Site::parse(slug)?;
    let prefix = format!("{}/{}/job/", site.origin(), site.site);
    let Some(rest) = url.strip_prefix(&prefix) else { return Ok(None) };
    let d: Detail = http.get_json(&format!("{}/job/{rest}", site.cxs()), HOSTS).await?;
    let mut text = d.job_posting_info.job_description.as_deref().map(html_to_text).unwrap_or_default();
    if let Some(t) = d.job_posting_info.time_type.filter(|t| !t.is_empty()) {
        text = format!("{t}\n\n{text}");
    }
    Ok(Some(text).filter(|t| !t.trim().is_empty()))
}

/// `https://nvidia.wd5.myworkdayjobs.com/en-US/NVIDIAExternalCareerSite/…` and the
/// `/wday/cxs/{tenant}/{site}` API form both resolve to `nvidia.wd5.NVIDIAExternalCareerSite`.
pub fn detect(host: &str, segments: &[&str]) -> Option<String> {
    let labels: Vec<&str> = host.split('.').collect();
    let [tenant, instance, "myworkdayjobs", "com"] = labels.as_slice() else { return None };
    if !instance.starts_with("wd") {
        return None;
    }
    let site = match segments {
        ["wday", "cxs", _, site, ..] => *site,
        [locale, site, ..] if is_locale(locale) => *site,
        [site, ..] => *site,
        [] => return None,
    };
    let slug = format!("{tenant}.{instance}.{site}");
    let ok = Site::parse(&slug).is_ok();
    ok.then_some(slug)
}

fn is_locale(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() == 5 && b[2] == b'-' && b[..2].iter().all(u8::is_ascii_lowercase) && b[3..].iter().all(u8::is_ascii_uppercase)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_listing_and_parses_relative_dates() {
        let raw = r#"{"total": 2000, "jobPostings": [
            {"title": "Senior Engineer", "externalPath": "/job/US-CA-Santa-Clara/Senior-Engineer_JR1", "locationsText": "US, CA, Santa Clara", "postedOn": "Posted 3 Days Ago", "bulletFields": ["JR1"]},
            {"title": "No path"}
        ]}"#;
        let p: Page = serde_json::from_str(raw).unwrap();
        let site = Site::parse("nvidia.wd5.NVIDIAExternalCareerSite").unwrap();
        let jobs: Vec<_> = p.job_postings.into_iter().filter_map(|j| map(j, &site, "nvidia.wd5.NVIDIAExternalCareerSite", "NVIDIA")).collect();
        assert_eq!(jobs.len(), 1);
        assert_eq!(jobs[0].url, "https://nvidia.wd5.myworkdayjobs.com/NVIDIAExternalCareerSite/job/US-CA-Santa-Clara/Senior-Engineer_JR1");
        let age = Utc::now() - jobs[0].posted_at.unwrap();
        assert_eq!(age.num_days(), 3);
        assert!(posted_on("Posted 30+ Days Ago").is_some());
        assert!(posted_on("Posted Today").is_some());
    }

    #[test]
    fn detects_career_sites() {
        assert_eq!(detect("nvidia.wd5.myworkdayjobs.com", &["en-US", "NVIDIAExternalCareerSite", "job", "x"]).as_deref(), Some("nvidia.wd5.NVIDIAExternalCareerSite"));
        assert_eq!(detect("acme.wd1.myworkdayjobs.com", &["External"]).as_deref(), Some("acme.wd1.External"));
        assert_eq!(detect("acme.wd1.myworkdayjobs.com", &["wday", "cxs", "acme", "Careers", "jobs"]).as_deref(), Some("acme.wd1.Careers"));
        assert_eq!(detect("acme.myworkdayjobs.com", &["External"]), None);
        assert!(Site::parse("acme.wd1").is_err());
    }
}
