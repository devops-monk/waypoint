//! Workable's public careers widget API (no auth). Reference: career-ops
//! `providers/workable.mjs`. A role listed in several cities arrives once per city, so
//! rows with the same link are merged.

use serde::Deserialize;

use crate::http::Http;
use crate::text::{html_to_text, parse_date};
use crate::{SourceError, slug_ok};
use waypoint_core::JobDraft;

pub const HOSTS: &[&str] = &["apply.workable.com"];

#[derive(Deserialize)]
struct Widget {
    #[serde(default)]
    jobs: Vec<Job>,
}

#[derive(Deserialize)]
struct Job {
    title: Option<String>,
    shortcode: Option<String>,
    shortlink: Option<String>,
    url: Option<String>,
    employment_type: Option<String>,
    #[serde(default)]
    telecommuting: bool,
    department: Option<String>,
    city: Option<String>,
    country: Option<String>,
    description: Option<String>,
    published_on: Option<String>,
    created_at: Option<String>,
}

pub async fn fetch(http: &Http, slug: &str, company: &str) -> Result<Vec<JobDraft>, SourceError> {
    slug_ok(slug)?;
    let w: Widget = http.get_json(&format!("https://apply.workable.com/api/v1/widget/accounts/{slug}?details=true"), HOSTS).await?;
    Ok(merge(w.jobs, slug, company))
}

fn merge(jobs: Vec<Job>, slug: &str, company: &str) -> Vec<JobDraft> {
    let mut out: Vec<JobDraft> = Vec::new();
    for j in jobs {
        let Some(url) = j.shortlink.or(j.url).filter(|u| u.starts_with("https://apply.workable.com/")) else { continue };
        let place = [j.city, j.country].into_iter().flatten().filter(|s| !s.trim().is_empty()).collect::<Vec<_>>().join(", ");
        if let Some(existing) = out.iter_mut().find(|d| d.url == url) {
            if !place.is_empty() && !existing.location.contains(&place) {
                existing.location = format!("{} · {place}", existing.location).trim_start_matches(" · ").to_string();
            }
            continue;
        }
        let location = match (place.is_empty(), j.telecommuting) {
            (true, true) => "Remote".to_string(),
            (false, true) => format!("{place} · Remote"),
            _ => place,
        };
        out.push(JobDraft {
            source: "workable".into(),
            board: Some(slug.into()),
            external_id: j.shortcode,
            url,
            company: company.into(),
            title: j.title.unwrap_or_default(),
            location,
            remote: j.telecommuting,
            employment_type: j.employment_type.filter(|s| !s.is_empty()),
            department: j.department.filter(|s| !s.is_empty()),
            description: j.description.as_deref().map(html_to_text).unwrap_or_default(),
            salary: None,
            posted_at: j.published_on.or(j.created_at).as_deref().and_then(parse_date),
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merges_cities_of_one_role() {
        let raw = r#"{"jobs": [
          {"title": "ML Engineer", "shortcode": "A1", "shortlink": "https://apply.workable.com/j/A1", "telecommuting": true, "city": "Paris", "country": "France", "description": "<p>Build</p>", "published_on": "2026-05-29"},
          {"title": "ML Engineer", "shortcode": "A1", "shortlink": "https://apply.workable.com/j/A1", "telecommuting": true, "city": "Berlin", "country": "Germany"},
          {"title": "Phish", "shortlink": "https://evil.test/j/X"}
        ]}"#;
        let w: Widget = serde_json::from_str(raw).unwrap();
        let jobs = merge(w.jobs, "huggingface", "Hugging Face");
        assert_eq!(jobs.len(), 1);
        assert_eq!(jobs[0].location, "Paris, France · Remote · Berlin, Germany");
        assert_eq!(jobs[0].description, "Build");
    }
}
