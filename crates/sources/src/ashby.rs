//! Ashby public job board API (no auth). Reference: career-ops `providers/ashby.mjs`.
//! Secondary locations are folded in so multi-city postings aren't filtered out.

use serde::Deserialize;

use crate::http::Http;
use crate::text::{html_to_text, looks_remote, parse_date};
use crate::{SourceError, slug_ok};
use waypoint_core::{JobDraft, Salary};

pub const HOSTS: &[&str] = &["api.ashbyhq.com"];

#[derive(Deserialize)]
struct Board {
    #[serde(default)]
    jobs: Vec<Job>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Job {
    id: Option<String>,
    title: Option<String>,
    location: Option<String>,
    #[serde(default)]
    secondary_locations: Vec<Secondary>,
    department: Option<String>,
    team: Option<String>,
    is_remote: Option<bool>,
    workplace_type: Option<String>,
    employment_type: Option<String>,
    job_url: Option<String>,
    description_plain: Option<String>,
    description_html: Option<String>,
    published_at: Option<String>,
    is_listed: Option<bool>,
    compensation: Option<Compensation>,
}

#[derive(Deserialize)]
struct Secondary {
    location: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Compensation {
    #[serde(default)]
    summary_components: Vec<Component>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Component {
    compensation_type: Option<String>,
    interval: Option<String>,
    currency_code: Option<String>,
    min_value: Option<f64>,
    max_value: Option<f64>,
}

pub async fn fetch(http: &Http, slug: &str, company: &str) -> Result<Vec<JobDraft>, SourceError> {
    slug_ok(slug)?;
    let url = format!("https://api.ashbyhq.com/posting-api/job-board/{slug}?includeCompensation=true");
    let board: Board = http.get_json(&url, HOSTS).await?;
    Ok(board.jobs.into_iter().filter_map(|j| map(j, slug, company)).collect())
}

fn map(j: Job, slug: &str, company: &str) -> Option<JobDraft> {
    if j.is_listed == Some(false) {
        return None;
    }
    let url = j.job_url.filter(|u| !u.is_empty())?;
    let mut locs: Vec<String> = j.location.into_iter().collect();
    for s in j.secondary_locations.into_iter().filter_map(|s| s.location) {
        if !locs.contains(&s) {
            locs.push(s);
        }
    }
    let location = locs.join(" · ");
    let salary = j.compensation.and_then(|c| c.summary_components.into_iter().find(|s| s.compensation_type.as_deref() == Some("Salary"))).map(|s| Salary {
        min: s.min_value,
        max: s.max_value,
        currency: s.currency_code,
        period: s.interval.map(|i| if i.contains("YEAR") { "year".into() } else if i.contains("MONTH") { "month".into() } else if i.contains("HOUR") { "hour".into() } else { i }),
    });
    Some(JobDraft {
        source: "ashby".into(),
        board: Some(slug.into()),
        external_id: j.id,
        remote: j.is_remote == Some(true) || j.workplace_type.as_deref() == Some("Remote") || looks_remote(&location),
        location,
        url,
        company: company.into(),
        title: j.title.unwrap_or_default(),
        employment_type: j.employment_type,
        department: j.department.or(j.team),
        description: j.description_plain.filter(|s| !s.trim().is_empty()).unwrap_or_else(|| j.description_html.as_deref().map(html_to_text).unwrap_or_default()),
        salary,
        posted_at: j.published_at.as_deref().and_then(parse_date),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_fixture_and_skips_unlisted() {
        let raw = r#"{"jobs":[
          {"id":"1","title":"ML Engineer","location":"London","secondaryLocations":[{"location":"Berlin"}],
           "isRemote":false,"workplaceType":"Hybrid","jobUrl":"https://jobs.ashbyhq.com/acme/1","descriptionPlain":"Do ML",
           "publishedAt":"2026-08-01T00:00:00.000+00:00","isListed":true,
           "compensation":{"summaryComponents":[{"compensationType":"Salary","interval":"1 YEAR","currencyCode":"GBP","minValue":90000,"maxValue":120000}]}},
          {"id":"2","title":"Hidden","jobUrl":"https://jobs.ashbyhq.com/acme/2","isListed":false}
        ]}"#;
        let board: Board = serde_json::from_str(raw).unwrap();
        let jobs: Vec<_> = board.jobs.into_iter().filter_map(|j| map(j, "acme", "Acme")).collect();
        assert_eq!(jobs.len(), 1);
        assert_eq!(jobs[0].location, "London · Berlin");
        assert_eq!(jobs[0].salary.as_ref().unwrap().currency.as_deref(), Some("GBP"));
    }
}
