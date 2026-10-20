//! Sweden's Platsbanken (Arbetsförmedlingen) through the open JobTech JobSearch API, no key
//! needed. Reference: career-ops `providers/platsbanken.mjs`. Query: "keywords".

use serde::Deserialize;

use crate::http::Http;
use crate::text::parse_date;
use crate::{SourceError, query_ok, split_query};
use waypoint_core::JobDraft;

pub const HOSTS: &[&str] = &["jobsearch.api.jobtechdev.se"];

#[derive(Deserialize)]
struct Results {
    #[serde(default)]
    hits: Vec<Ad>,
}

#[derive(Deserialize)]
struct Ad {
    id: Option<String>,
    headline: Option<String>,
    webpage_url: Option<String>,
    description: Option<Text>,
    employer: Option<Employer>,
    workplace_address: Option<Address>,
    employment_type: Option<Label>,
    workplace_model: Option<Label>,
    occupation_field: Option<Label>,
    publication_date: Option<String>,
}

#[derive(Deserialize)]
struct Text {
    text: Option<String>,
}

#[derive(Deserialize)]
struct Employer {
    name: Option<String>,
    workplace: Option<String>,
}

#[derive(Deserialize)]
struct Address {
    municipality: Option<String>,
    region: Option<String>,
    country: Option<String>,
}

#[derive(Deserialize)]
struct Label {
    label: Option<String>,
}

pub async fn fetch(http: &Http, query: &str) -> Result<Vec<JobDraft>, SourceError> {
    query_ok(query)?;
    let (what, place) = split_query(query);
    let q = match place {
        Some(p) => format!("{what} {p}"),
        None => what.to_string(),
    };
    let mut url = url::Url::parse("https://jobsearch.api.jobtechdev.se/search").expect("static url");
    url.query_pairs_mut().append_pair("q", &q).append_pair("limit", "100").append_pair("sort", "pubdate-desc");
    let r: Results = http.get_json(url.as_str(), HOSTS).await?;
    Ok(r.hits.into_iter().filter_map(|a| map(a, query)).collect())
}

fn map(a: Ad, query: &str) -> Option<JobDraft> {
    let id = a.id.filter(|i| i.chars().all(|c| c.is_ascii_digit()))?;
    let title = a.headline.filter(|t| !t.trim().is_empty())?;
    let model = a.workplace_model.and_then(|m| m.label).unwrap_or_default();
    let remote = model.to_lowercase().contains("distans");
    let mut location = a.workplace_address.map(|w| [w.municipality.or(w.region), w.country.filter(|c| c != "Sverige")].into_iter().flatten().collect::<Vec<_>>().join(", ")).unwrap_or_default();
    if remote {
        location = if location.is_empty() { "Distans".into() } else { format!("{location} · Distans") };
    }
    let employer = a.employer.and_then(|e| e.name.or(e.workplace)).unwrap_or_default();
    Some(JobDraft {
        source: "platsbanken".into(),
        board: Some(query.into()),
        url: a.webpage_url.filter(|u| u.starts_with("https://arbetsformedlingen.se/")).unwrap_or_else(|| format!("https://arbetsformedlingen.se/platsbanken/annonser/{id}")),
        external_id: Some(id),
        company: employer,
        title,
        location,
        remote,
        employment_type: a.employment_type.and_then(|e| e.label),
        department: a.occupation_field.and_then(|o| o.label),
        description: a.description.and_then(|d| d.text).unwrap_or_default().trim().to_string(),
        salary: None,
        posted_at: a.publication_date.as_deref().and_then(parse_date),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_ads() {
        let raw = r#"{"hits": [{"id": "31581608", "headline": "Backendutvecklare", "webpage_url": "https://arbetsformedlingen.se/platsbanken/annonser/31581608",
            "description": {"text": "Vi söker en utvecklare."}, "employer": {"name": "Acme AB"}, "workplace_address": {"municipality": "Stockholm", "country": "Sverige"},
            "workplace_model": {"label": "Distansarbete"}, "publication_date": "2026-10-12T06:38:05"}]}"#;
        let r: Results = serde_json::from_str(raw).unwrap();
        let j = map(r.hits.into_iter().next().unwrap(), "utvecklare").unwrap();
        assert_eq!(j.location, "Stockholm · Distans");
        assert!(j.remote);
        assert_eq!(j.company, "Acme AB");
    }
}
