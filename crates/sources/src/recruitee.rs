//! Recruitee public offers API (no auth), served from `{company}.recruitee.com`.
//! Reference: career-ops `providers/recruitee.mjs`.

use serde::Deserialize;

use crate::http::Http;
use crate::text::{html_to_text, looks_remote, parse_date};
use crate::{SourceError, slug_ok};
use waypoint_core::{JobDraft, Salary};

pub const HOSTS: &[&str] = &["*.recruitee.com"];

#[derive(Deserialize)]
struct Offers {
    #[serde(default)]
    offers: Vec<Offer>,
}

#[derive(Deserialize)]
struct Offer {
    id: Option<i64>,
    title: Option<String>,
    location: Option<String>,
    #[serde(default)]
    remote: bool,
    careers_url: Option<String>,
    department: Option<String>,
    employment_type_code: Option<String>,
    description: Option<String>,
    requirements: Option<String>,
    published_at: Option<String>,
    salary: Option<RSalary>,
    status: Option<String>,
}

#[derive(Deserialize)]
struct RSalary {
    min: Option<serde_json::Value>,
    max: Option<serde_json::Value>,
    period: Option<String>,
    currency: Option<String>,
}

fn num(v: Option<serde_json::Value>) -> Option<f64> {
    match v? {
        serde_json::Value::Number(n) => n.as_f64(),
        serde_json::Value::String(s) => s.parse().ok(),
        _ => None,
    }
}

pub async fn fetch(http: &Http, slug: &str, company: &str) -> Result<Vec<JobDraft>, SourceError> {
    slug_ok(slug)?;
    if slug.contains('.') {
        return Err(SourceError::BadSlug(slug.into()));
    }
    let o: Offers = http.get_json(&format!("https://{slug}.recruitee.com/api/offers/"), HOSTS).await?;
    Ok(o.offers.into_iter().filter_map(|j| map(j, slug, company)).collect())
}

fn map(j: Offer, slug: &str, company: &str) -> Option<JobDraft> {
    if j.status.as_deref().is_some_and(|s| s != "published") {
        return None;
    }
    let title = j.title.filter(|t| !t.trim().is_empty() && !t.to_lowercase().contains("(sample)"))?;
    let url = j.careers_url.filter(|u| u.starts_with("https://"))?;
    let mut location = j.location.unwrap_or_default();
    if j.remote && !looks_remote(&location) {
        location = if location.is_empty() { "Remote".into() } else { format!("{location} · Remote") };
    }
    let body = [j.description, j.requirements].into_iter().flatten().map(|h| html_to_text(&h)).filter(|t| !t.is_empty()).collect::<Vec<_>>().join("\n\n");
    let salary = j.salary.and_then(|s| {
        let (min, max) = (num(s.min), num(s.max));
        (min.is_some() || max.is_some()).then_some(Salary { min, max, currency: s.currency, period: s.period })
    });
    Some(JobDraft {
        source: "recruitee".into(),
        board: Some(slug.into()),
        external_id: j.id.map(|i| i.to_string()),
        url,
        company: company.into(),
        title,
        remote: j.remote || looks_remote(&location),
        location,
        employment_type: j.employment_type_code.map(|c| c.replace('_', " ")),
        department: j.department,
        description: body,
        salary,
        posted_at: j.published_at.as_deref().and_then(parse_date),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_offers_and_skips_samples() {
        let raw = r#"{"offers": [
          {"id": 1, "title": "Head of Product", "location": "Amsterdam, Netherlands", "remote": false, "careers_url": "https://careers.bunq.com/o/head-of-product",
           "description": "<p>Lead</p>", "requirements": "<ul><li>Experience</li></ul>", "published_at": "2026-10-15 09:31:36 UTC", "status": "published",
           "salary": {"min": null, "max": null, "period": null, "currency": null}},
          {"id": 2, "title": "Designer (sample)", "careers_url": "https://x.recruitee.com/o/d", "status": "published"}
        ]}"#;
        let o: Offers = serde_json::from_str(raw).unwrap();
        let jobs: Vec<_> = o.offers.into_iter().filter_map(|j| map(j, "bunq", "bunq")).collect();
        assert_eq!(jobs.len(), 1);
        assert_eq!(jobs[0].description, "Lead\n\n• Experience");
        assert!(jobs[0].salary.is_none());
        assert!(jobs[0].posted_at.is_some());
    }
}
