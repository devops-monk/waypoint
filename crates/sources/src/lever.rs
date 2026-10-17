//! Lever public postings API (no auth). Reference: career-ops `providers/lever.mjs`.
//! Multi-location postings expose the full set in `categories.allLocations`; reading only
//! `location` would hide every other eligible city.

use serde::Deserialize;

use crate::http::Http;
use crate::text::{from_millis, html_to_text, looks_remote};
use crate::{SourceError, slug_ok};
use waypoint_core::{JobDraft, Salary};

pub const HOSTS: &[&str] = &["api.lever.co", "api.eu.lever.co"];

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Posting {
    id: String,
    text: Option<String>,
    hosted_url: Option<String>,
    #[serde(default)]
    categories: Categories,
    workplace_type: Option<String>,
    description_plain: Option<String>,
    description: Option<String>,
    #[serde(default)]
    lists: Vec<List>,
    additional_plain: Option<String>,
    created_at: Option<i64>,
    salary_range: Option<SalaryRange>,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct Categories {
    location: Option<String>,
    #[serde(default)]
    all_locations: Vec<String>,
    commitment: Option<String>,
    team: Option<String>,
    department: Option<String>,
}

#[derive(Deserialize)]
struct List {
    text: Option<String>,
    content: Option<String>,
}

#[derive(Deserialize)]
struct SalaryRange {
    min: Option<f64>,
    max: Option<f64>,
    currency: Option<String>,
    interval: Option<String>,
}

/// `eu` selects Lever's EU data-center API.
pub async fn fetch(http: &Http, slug: &str, company: &str, eu: bool) -> Result<Vec<JobDraft>, SourceError> {
    slug_ok(slug)?;
    let host = if eu { "api.eu.lever.co" } else { "api.lever.co" };
    let postings: Vec<Posting> = http.get_json(&format!("https://{host}/v0/postings/{slug}?mode=json"), HOSTS).await?;
    Ok(postings.into_iter().filter_map(|p| map(p, slug, company)).collect())
}

fn location_of(c: &Categories) -> String {
    let mut all: Vec<String> = c.location.iter().cloned().collect();
    for l in &c.all_locations {
        if !all.iter().any(|x| x.eq_ignore_ascii_case(l)) {
            all.push(l.clone());
        }
    }
    all.join(" · ")
}

fn map(p: Posting, slug: &str, company: &str) -> Option<JobDraft> {
    let url = p.hosted_url.filter(|u| !u.is_empty())?;
    let location = location_of(&p.categories);
    let mut description = p.description_plain.filter(|s| !s.trim().is_empty()).unwrap_or_else(|| p.description.as_deref().map(html_to_text).unwrap_or_default());
    for list in &p.lists {
        let body = list.content.as_deref().map(html_to_text).unwrap_or_default();
        if !body.is_empty() {
            description.push_str(&format!("\n\n{}\n{}", list.text.as_deref().unwrap_or(""), body));
        }
    }
    if let Some(extra) = p.additional_plain.filter(|s| !s.trim().is_empty()) {
        description.push_str("\n\n");
        description.push_str(extra.trim());
    }
    let remote = p.workplace_type.as_deref() == Some("remote") || looks_remote(&location);
    let salary = p.salary_range.map(|s| Salary {
        min: s.min,
        max: s.max,
        currency: s.currency,
        period: s.interval.map(|i| match i.as_str() {
            "per-year-salary" => "year".into(),
            "per-month-salary" => "month".into(),
            "per-hour-wage" => "hour".into(),
            other => other.into(),
        }),
    });
    Some(JobDraft {
        source: "lever".into(),
        board: Some(slug.into()),
        external_id: Some(p.id),
        url,
        company: company.into(),
        title: p.text.unwrap_or_default(),
        location,
        remote,
        employment_type: p.categories.commitment,
        department: p.categories.team.or(p.categories.department),
        description: description.trim().to_string(),
        salary,
        posted_at: p.created_at.and_then(from_millis),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_fixture_with_all_locations_and_salary() {
        let raw = r#"[{"id":"abc","text":"Backend Engineer","hostedUrl":"https://jobs.lever.co/acme/abc",
            "categories":{"location":"Barcelona","allLocations":["Barcelona","Montevideo"],"commitment":"Full-time","team":"Platform"},
            "workplaceType":"hybrid","descriptionPlain":"Intro","lists":[{"text":"You have","content":"<li>Go</li>"}],
            "createdAt":1767225600000,"salaryRange":{"min":60000,"max":80000,"currency":"EUR","interval":"per-year-salary"}}]"#;
        let ps: Vec<Posting> = serde_json::from_str(raw).unwrap();
        let j = map(ps.into_iter().next().unwrap(), "acme", "Acme").unwrap();
        assert_eq!(j.location, "Barcelona · Montevideo");
        assert!(!j.remote);
        assert_eq!(j.description, "Intro\n\nYou have\n• Go");
        assert_eq!(j.salary.unwrap().period.as_deref(), Some("year"));
    }
}
