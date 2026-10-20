//! Remote OK's public feed (no auth). Their API terms require a followed link back to the
//! posting on remoteok.com and naming Remote OK as the source, which the job URL and the
//! source label do. The feed has no search, so the keyword is matched here.
//! Reference: career-ops `providers/remoteok.mjs`.

use serde::Deserialize;

use crate::http::Http;
use crate::text::{html_to_text, parse_date};
use crate::{SourceError, matches_query, query_ok};
use waypoint_core::{JobDraft, Salary};

pub const HOSTS: &[&str] = &["remoteok.com"];

#[derive(Deserialize)]
struct Job {
    id: Option<serde_json::Value>,
    url: Option<String>,
    position: Option<String>,
    company: Option<String>,
    location: Option<String>,
    description: Option<String>,
    date: Option<String>,
    #[serde(default)]
    tags: Vec<String>,
    salary_min: Option<f64>,
    salary_max: Option<f64>,
}

pub async fn fetch(http: &Http, query: &str) -> Result<Vec<JobDraft>, SourceError> {
    query_ok(query)?;
    // The first element is the legal notice, not a job.
    let rows: Vec<serde_json::Value> = http.get_json("https://remoteok.com/api", HOSTS).await?;
    Ok(rows.into_iter().filter_map(|r| serde_json::from_value::<Job>(r).ok()).filter_map(|j| map(j, query)).collect())
}

fn map(j: Job, query: &str) -> Option<JobDraft> {
    let url = j.url.filter(|u| u.to_lowercase().starts_with("https://remoteok.com/"))?;
    let title = j.position.filter(|t| !t.trim().is_empty())?;
    let description = j.description.as_deref().map(html_to_text).unwrap_or_default();
    if !matches_query(&format!("{title} {} {description}", j.tags.join(" ")), query) {
        return None;
    }
    let salary = match (j.salary_min.filter(|v| *v > 0.0), j.salary_max.filter(|v| *v > 0.0)) {
        (None, None) => None,
        (min, max) => Some(Salary { min, max, currency: Some("USD".into()), period: Some("year".into()) }),
    };
    let location = j.location.filter(|l| !l.trim().is_empty()).map(|l| format!("Remote · {l}")).unwrap_or_else(|| "Remote".into());
    Some(JobDraft {
        source: "remoteok".into(),
        board: Some(query.into()),
        external_id: j.id.map(|v| v.to_string().trim_matches('"').to_string()),
        url,
        company: j.company.unwrap_or_default(),
        title,
        location,
        remote: true,
        employment_type: None,
        department: None,
        description,
        salary,
        posted_at: j.date.as_deref().and_then(parse_date),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filters_by_keyword_and_skips_the_notice() {
        let rows: Vec<serde_json::Value> = serde_json::from_str(r#"[
          {"legal": "API Terms of Service"},
          {"id": "1", "url": "https://remoteOK.com/remote-jobs/rust-1", "position": "Rust Engineer", "company": "Acme", "tags": ["rust"], "date": "2026-10-17T08:00:50+00:00", "salary_min": 0, "salary_max": 0},
          {"id": "2", "url": "https://remoteOK.com/remote-jobs/planner-2", "position": "Transport Planner", "company": "Curtins", "tags": ["consulting"]}
        ]"#).unwrap();
        let jobs: Vec<_> = rows.into_iter().filter_map(|r| serde_json::from_value::<Job>(r).ok()).filter_map(|j| map(j, "rust")).collect();
        assert_eq!(jobs.len(), 1);
        assert!(jobs[0].salary.is_none());
        assert_eq!(jobs[0].location, "Remote");
    }
}
