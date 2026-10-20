//! Denmark's Jobnet.dk (the public employment service), through the same search endpoint
//! its website uses. Reference: ai-job-search `.agents/skills/jobnet-search`.
//! Query: "keywords". Listings carry a short description; the full ad is on the linked page.

use serde::Deserialize;

use crate::http::Http;
use crate::text::{html_to_text, looks_remote, parse_date};
use crate::{SourceError, query_ok, split_query};
use waypoint_core::JobDraft;

pub const HOSTS: &[&str] = &["jobnet.dk"];

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Results {
    #[serde(default)]
    job_ads: Vec<Ad>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Ad {
    job_ad_id: Option<String>,
    title: Option<String>,
    hiring_org_name: Option<String>,
    occupation: Option<String>,
    municipality: Option<String>,
    postal_district_name: Option<String>,
    country: Option<String>,
    #[serde(default)]
    work_hour_part_time: bool,
    job_ad_url: Option<String>,
    description: Option<String>,
    publication_date: Option<String>,
}

pub async fn fetch(http: &Http, query: &str) -> Result<Vec<JobDraft>, SourceError> {
    query_ok(query)?;
    let (what, place) = split_query(query);
    let q = match place {
        Some(p) => format!("{what} {p}"),
        None => what.to_string(),
    };
    let mut url = url::Url::parse("https://jobnet.dk/bff/FindJob/Search").expect("static url");
    url.query_pairs_mut().append_pair("searchString", &q).append_pair("resultsPerPage", "100").append_pair("pageNumber", "1").append_pair("orderType", "PublicationDate");
    let r: Results = http.get_json_with(url.as_str(), HOSTS, &[("x-csrf", "1")]).await?;
    Ok(r.job_ads.into_iter().filter_map(|a| map(a, query)).collect())
}

fn map(a: Ad, query: &str) -> Option<JobDraft> {
    let id = a.job_ad_id.filter(|i| i.chars().all(|c| c.is_ascii_hexdigit() || c == '-'))?;
    let title = a.title.map(|t| t.trim().to_string()).filter(|t| !t.is_empty())?;
    let place = a.postal_district_name.or(a.municipality).unwrap_or_default();
    let location = match a.country.filter(|c| c != "Danmark") {
        Some(c) if !place.is_empty() => format!("{place}, {c}"),
        Some(c) => c,
        None => place,
    };
    let external = a.job_ad_url.filter(|u| u.starts_with("https://"));
    Some(JobDraft {
        source: "jobnet".into(),
        board: Some(query.into()),
        url: external.unwrap_or_else(|| format!("https://jobnet.dk/find-job/{id}")),
        external_id: Some(id),
        company: a.hiring_org_name.unwrap_or_default(),
        title,
        remote: looks_remote(&location),
        location,
        employment_type: Some(if a.work_hour_part_time { "Deltid" } else { "Fuldtid" }.into()),
        department: a.occupation,
        description: a.description.as_deref().map(html_to_text).unwrap_or_default(),
        salary: None,
        posted_at: a.publication_date.as_deref().and_then(parse_date),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_ads() {
        let raw = r#"{"jobAds": [{"jobAdId": "dafaccad-e5dc-42e4-a471-becf7e4f03f5", "title": "BLIV PROGRAMMØR ", "hiringOrgName": "RIKLAS ApS",
            "occupation": "Programmør og systemudvikler", "municipality": "Frederiksberg", "postalDistrictName": "Frederiksberg", "country": "Danmark",
            "workHourPartTime": false, "jobAdUrl": "", "description": "<p><strong>Vil du bygge vores CRM?</strong></p>", "publicationDate": "2026-10-18T00:00:00+02:00"}]}"#;
        let r: Results = serde_json::from_str(raw).unwrap();
        let j = map(r.job_ads.into_iter().next().unwrap(), "udvikler").unwrap();
        assert_eq!(j.url, "https://jobnet.dk/find-job/dafaccad-e5dc-42e4-a471-becf7e4f03f5");
        assert_eq!(j.title, "BLIV PROGRAMMØR");
        assert_eq!(j.location, "Frederiksberg");
        assert_eq!(j.description, "Vil du bygge vores CRM?");
    }
}
