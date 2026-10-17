use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::status::Status;

/// A posting as produced by a source, before it has an id or lifecycle state.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct JobDraft {
    pub source: String,
    pub board: Option<String>,
    pub external_id: Option<String>,
    pub url: String,
    pub company: String,
    pub title: String,
    pub location: String,
    pub remote: bool,
    pub employment_type: Option<String>,
    pub department: Option<String>,
    pub description: String,
    pub salary: Option<Salary>,
    pub posted_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Salary {
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub currency: Option<String>,
    /// year | month | hour
    pub period: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    pub id: i64,
    pub dedup_key: String,
    pub url: String,
    pub source: String,
    pub board: Option<String>,
    pub company: String,
    pub title: String,
    pub location: String,
    pub remote: bool,
    pub employment_type: Option<String>,
    pub department: Option<String>,
    pub description: String,
    pub salary: Option<Salary>,
    pub posted_at: Option<DateTime<Utc>>,
    pub first_seen_at: DateTime<Utc>,
    pub last_seen_at: DateTime<Utc>,
    pub liveness: String,
    pub status: Status,
    pub status_at: DateTime<Utc>,
    pub starred: bool,
    pub notes: String,
    /// Latest evaluation headline, if any.
    pub score: Option<f64>,
    pub band: Option<String>,
}

/// Lightweight row for list views (no description body).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobSummary {
    pub id: i64,
    pub company: String,
    pub title: String,
    pub location: String,
    pub remote: bool,
    pub source: String,
    pub url: String,
    pub salary: Option<Salary>,
    pub posted_at: Option<DateTime<Utc>>,
    pub first_seen_at: DateTime<Utc>,
    pub status: Status,
    pub starred: bool,
    pub score: Option<f64>,
    pub band: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct JobFilter {
    pub statuses: Option<Vec<Status>>,
    pub query: Option<String>,
    pub remote_only: bool,
    pub starred_only: bool,
    pub min_score: Option<f64>,
    pub limit: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Board {
    pub id: i64,
    pub provider: String,
    pub slug: String,
    pub company: String,
    pub enabled: bool,
    pub last_run_at: Option<DateTime<Utc>>,
    pub last_error: Option<String>,
    pub last_count: Option<i64>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UpsertOutcome {
    pub inserted: usize,
    pub updated: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredEvaluation {
    pub id: i64,
    pub job_id: i64,
    pub score: Option<f64>,
    pub band: String,
    pub payload: serde_json::Value,
    pub model: String,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineCounts {
    pub status: Status,
    pub count: i64,
}
