//! Himalayas remote jobs search API (no auth). Reference: career-ops `providers/himalayas.mjs`.

use serde::Deserialize;

use crate::http::Http;
use crate::text::{html_to_text, parse_date};
use crate::{SourceError, query_ok};
use waypoint_core::{JobDraft, Salary};

pub const HOSTS: &[&str] = &["himalayas.app"];

#[derive(Deserialize)]
struct Feed {
    #[serde(default)]
    jobs: Vec<Job>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Job {
    title: Option<String>,
    company_name: Option<String>,
    employment_type: Option<String>,
    min_salary: Option<f64>,
    max_salary: Option<f64>,
    salary_period: Option<String>,
    currency: Option<String>,
    #[serde(default)]
    location_restrictions: Vec<String>,
    description: Option<String>,
    pub_date: Option<serde_json::Value>,
    application_link: Option<String>,
    guid: Option<String>,
}

pub async fn fetch(http: &Http, query: &str) -> Result<Vec<JobDraft>, SourceError> {
    query_ok(query)?;
    let mut url = url::Url::parse("https://himalayas.app/jobs/api/search").expect("static url");
    url.query_pairs_mut().append_pair("q", query).append_pair("limit", "100");
    let feed: Feed = http.get_json(url.as_str(), HOSTS).await?;
    Ok(feed.jobs.into_iter().filter_map(|j| map(j, query)).collect())
}

fn map(j: Job, query: &str) -> Option<JobDraft> {
    let url = j.guid.or(j.application_link).filter(|u| u.starts_with("https://himalayas.app/"))?;
    let posted = match j.pub_date {
        Some(serde_json::Value::Number(n)) => n.as_i64().and_then(|s| chrono::DateTime::from_timestamp(s, 0)),
        Some(serde_json::Value::String(s)) => parse_date(&s),
        _ => None,
    };
    let where_ = if j.location_restrictions.is_empty() { "Anywhere".to_string() } else { j.location_restrictions.join(", ") };
    Some(JobDraft {
        source: "himalayas".into(),
        board: Some(query.into()),
        external_id: None,
        url,
        company: j.company_name.unwrap_or_default(),
        title: j.title.unwrap_or_default(),
        location: format!("Remote · {where_}"),
        remote: true,
        employment_type: j.employment_type,
        department: None,
        description: j.description.as_deref().map(html_to_text).unwrap_or_default(),
        salary: (j.min_salary.is_some() || j.max_salary.is_some()).then(|| Salary {
            min: j.min_salary,
            max: j.max_salary,
            currency: j.currency,
            period: j.salary_period.map(|p| if p == "annual" { "year".into() } else { p }),
        }),
        posted_at: posted,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_jobs() {
        let raw = r#"{"jobs": [{"title": "Rust Developer", "companyName": "Acme", "employmentType": "Full Time", "minSalary": 90000, "maxSalary": null,
            "salaryPeriod": "annual", "currency": "USD", "locationRestrictions": ["United States"], "description": "<p>Rust</p>",
            "pubDate": 1791341968, "guid": "https://himalayas.app/companies/acme/jobs/rust-developer"}]}"#;
        let f: Feed = serde_json::from_str(raw).unwrap();
        let j = map(f.jobs.into_iter().next().unwrap(), "rust").unwrap();
        assert_eq!(j.location, "Remote · United States");
        assert_eq!(j.salary.unwrap().period.as_deref(), Some("year"));
        assert!(j.posted_at.is_some());
    }
}
