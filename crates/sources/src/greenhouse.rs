//! Greenhouse public job board API (no auth). Reference: career-ops `providers/greenhouse.mjs`.

use serde::Deserialize;

use crate::http::Http;
use crate::text::{html_to_text, looks_remote, parse_date};
use crate::{SourceError, slug_ok};
use waypoint_core::JobDraft;

pub const HOSTS: &[&str] = &["boards-api.greenhouse.io"];

#[derive(Deserialize)]
struct Board {
    #[serde(default)]
    jobs: Vec<Job>,
}

#[derive(Deserialize)]
struct Job {
    id: serde_json::Value,
    title: Option<String>,
    absolute_url: Option<String>,
    location: Option<Named>,
    first_published: Option<String>,
    updated_at: Option<String>,
    content: Option<String>,
    #[serde(default)]
    departments: Vec<Named>,
}

#[derive(Deserialize)]
struct Named {
    name: Option<String>,
}

pub async fn fetch(http: &Http, slug: &str, company: &str) -> Result<Vec<JobDraft>, SourceError> {
    slug_ok(slug)?;
    let url = format!("https://boards-api.greenhouse.io/v1/boards/{slug}/jobs?content=true");
    let board: Board = http.get_json(&url, HOSTS).await?;
    Ok(board.jobs.into_iter().filter_map(|j| map(j, slug, company)).collect())
}

fn map(j: Job, slug: &str, company: &str) -> Option<JobDraft> {
    let url = j.absolute_url.filter(|u| !u.is_empty())?;
    let location = j.location.and_then(|l| l.name).unwrap_or_default();
    Some(JobDraft {
        source: "greenhouse".into(),
        board: Some(slug.into()),
        external_id: Some(j.id.to_string().trim_matches('"').to_string()),
        remote: looks_remote(&location),
        location,
        url,
        company: company.into(),
        title: j.title.unwrap_or_default(),
        department: j.departments.into_iter().find_map(|d| d.name),
        description: j.content.as_deref().map(html_to_text).unwrap_or_default(),
        posted_at: j.first_published.or(j.updated_at).as_deref().and_then(parse_date),
        ..Default::default()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_fixture() {
        let raw = r#"{"jobs":[
            {"id":4011,"title":"Staff Engineer","absolute_url":"https://job-boards.greenhouse.io/acme/jobs/4011",
             "location":{"name":"Remote (US)"},"first_published":"2026-09-01T12:00:00-04:00",
             "content":"&lt;p&gt;Build things&lt;/p&gt;","departments":[{"name":"Engineering"}]},
            {"id":4012,"title":"No URL","absolute_url":null}
        ]}"#;
        let board: Board = serde_json::from_str(raw).unwrap();
        let jobs: Vec<_> = board.jobs.into_iter().filter_map(|j| map(j, "acme", "Acme")).collect();
        assert_eq!(jobs.len(), 1);
        let j = &jobs[0];
        assert_eq!((j.external_id.as_deref(), j.remote, j.description.as_str()), (Some("4011"), true, "Build things"));
        assert_eq!(j.department.as_deref(), Some("Engineering"));
        assert!(j.posted_at.is_some());
    }
}
