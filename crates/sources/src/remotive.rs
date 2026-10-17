//! Remotive public remote-jobs API. Their terms ask for attribution (link back to the
//! posting on remotive.com, which `url` does) and few requests per day, so scans of this
//! source are rate limited by the scheduler, not just the per-host throttle.

use serde::Deserialize;

use crate::SourceError;
use crate::http::Http;
use crate::text::{html_to_text, parse_date};
use waypoint_core::JobDraft;

pub const HOSTS: &[&str] = &["remotive.com"];

#[derive(Deserialize)]
struct Feed {
    #[serde(default)]
    jobs: Vec<Job>,
}

#[derive(Deserialize)]
struct Job {
    id: i64,
    url: String,
    title: String,
    company_name: String,
    category: Option<String>,
    job_type: Option<String>,
    publication_date: Option<String>,
    candidate_required_location: Option<String>,
    description: Option<String>,
}

/// `query` is a free-text search ("rust", "product designer").
pub async fn fetch(http: &Http, query: &str) -> Result<Vec<JobDraft>, SourceError> {
    let mut url = url::Url::parse("https://remotive.com/api/remote-jobs").expect("static url");
    url.query_pairs_mut().append_pair("search", query).append_pair("limit", "100");
    let feed: Feed = http.get_json(url.as_str(), HOSTS).await?;
    Ok(feed.jobs.into_iter().map(|j| map(j, query)).collect())
}

fn map(j: Job, query: &str) -> JobDraft {
    JobDraft {
        source: "remotive".into(),
        board: Some(query.into()),
        external_id: Some(j.id.to_string()),
        url: j.url,
        company: j.company_name,
        title: j.title,
        location: j.candidate_required_location.map(|l| format!("Remote · {l}")).unwrap_or_else(|| "Remote".into()),
        remote: true,
        employment_type: j.job_type.map(|t| t.replace('_', " ")),
        department: j.category,
        description: j.description.as_deref().map(html_to_text).unwrap_or_default(),
        salary: None,
        posted_at: j.publication_date.as_deref().and_then(parse_date),
    }
}
