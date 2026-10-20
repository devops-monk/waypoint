//! Adzuna job search API: one API covering the US, UK, India and most of Europe. It needs
//! your own free app ID and key from developer.adzuna.com, stored in the OS keychain.
//! Query: "cc:keywords" or "cc:keywords @ place", where cc is the country, e.g.
//! "in:rust developer @ Bengaluru". Descriptions are short snippets; the link opens the
//! full ad on Adzuna.

use serde::Deserialize;

use crate::http::Http;
use crate::text::{looks_remote, parse_date};
use crate::{SourceError, query_ok, split_query};
use waypoint_core::{JobDraft, Salary};

pub const HOSTS: &[&str] = &["api.adzuna.com"];

/// Countries Adzuna serves, with their currency.
pub const COUNTRIES: &[(&str, &str, &str)] = &[
    ("us", "United States", "USD"),
    ("gb", "United Kingdom", "GBP"),
    ("in", "India", "INR"),
    ("de", "Germany", "EUR"),
    ("nl", "Netherlands", "EUR"),
    ("fr", "France", "EUR"),
    ("at", "Austria", "EUR"),
    ("be", "Belgium", "EUR"),
    ("ch", "Switzerland", "CHF"),
    ("es", "Spain", "EUR"),
    ("it", "Italy", "EUR"),
    ("pl", "Poland", "PLN"),
    ("ca", "Canada", "CAD"),
    ("au", "Australia", "AUD"),
    ("nz", "New Zealand", "NZD"),
    ("sg", "Singapore", "SGD"),
    ("za", "South Africa", "ZAR"),
    ("br", "Brazil", "BRL"),
    ("mx", "Mexico", "MXN"),
];

#[derive(Debug, Clone)]
pub struct AdzunaKey {
    pub app_id: String,
    pub app_key: String,
}

#[derive(Deserialize)]
struct Results {
    #[serde(default)]
    results: Vec<Ad>,
}

#[derive(Deserialize)]
struct Ad {
    id: Option<String>,
    title: Option<String>,
    description: Option<String>,
    created: Option<String>,
    redirect_url: Option<String>,
    company: Option<Named>,
    location: Option<Named>,
    category: Option<Labelled>,
    contract_time: Option<String>,
    salary_min: Option<f64>,
    salary_max: Option<f64>,
    salary_is_predicted: Option<serde_json::Value>,
}

#[derive(Deserialize)]
struct Named {
    display_name: Option<String>,
}

#[derive(Deserialize)]
struct Labelled {
    label: Option<String>,
}

/// Split "in:rust developer" into the country and the rest.
pub fn parse_query(query: &str) -> Result<(&'static str, &'static str, &str), SourceError> {
    let (cc, rest) = query.split_once(':').ok_or_else(|| SourceError::BadSlug(query.into()))?;
    let (code, _, currency) = COUNTRIES.iter().find(|(c, _, _)| *c == cc.trim().to_lowercase()).ok_or_else(|| SourceError::BadSlug(query.into()))?;
    Ok((code, currency, rest.trim()))
}

pub async fn fetch(http: &Http, query: &str, key: Option<&AdzunaKey>) -> Result<Vec<JobDraft>, SourceError> {
    query_ok(query)?;
    let key = key.ok_or(SourceError::MissingKey("Adzuna"))?;
    let (country, currency, rest) = parse_query(query)?;
    let (what, place) = split_query(rest);
    let mut url = url::Url::parse(&format!("https://api.adzuna.com/v1/api/jobs/{country}/search/1")).expect("static url");
    {
        let mut q = url.query_pairs_mut();
        q.append_pair("app_id", &key.app_id).append_pair("app_key", &key.app_key).append_pair("what", what).append_pair("results_per_page", "50").append_pair("max_days_old", "30").append_pair("sort_by", "date");
        if let Some(p) = place {
            q.append_pair("where", p);
        }
    }
    let r: Results = http.get_json(url.as_str(), HOSTS).await?;
    Ok(r.results.into_iter().filter_map(|a| map(a, query, currency)).collect())
}

fn map(a: Ad, query: &str, currency: &str) -> Option<JobDraft> {
    let url = a.redirect_url.filter(|u| u.starts_with("https://www.adzuna."))?;
    let title = a.title.map(|t| crate::text::html_to_text(&t)).filter(|t| !t.is_empty())?;
    let location = a.location.and_then(|l| l.display_name).unwrap_or_default();
    // Adzuna estimates salaries it wasn't given; only show real ones.
    let predicted = matches!(a.salary_is_predicted, Some(serde_json::Value::String(ref s)) if s == "1") || matches!(a.salary_is_predicted, Some(serde_json::Value::Number(ref n)) if n.as_i64() == Some(1));
    let salary = (!predicted && (a.salary_min.is_some() || a.salary_max.is_some())).then(|| Salary { min: a.salary_min, max: a.salary_max, currency: Some(currency.into()), period: Some("year".into()) });
    Some(JobDraft {
        source: "adzuna".into(),
        board: Some(query.into()),
        external_id: a.id,
        url,
        company: a.company.and_then(|c| c.display_name).unwrap_or_default(),
        title,
        remote: looks_remote(&location),
        location,
        employment_type: a.contract_time.map(|c| c.replace('_', " ")),
        department: a.category.and_then(|c| c.label),
        description: a.description.as_deref().map(crate::text::html_to_text).unwrap_or_default(),
        salary,
        posted_at: a.created.as_deref().and_then(parse_date),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_queries_and_ads() {
        assert_eq!(parse_query("in:rust developer @ Bengaluru").unwrap(), ("in", "INR", "rust developer @ Bengaluru"));
        assert!(parse_query("xx:rust").is_err());
        assert!(parse_query("rust").is_err());
        let raw = r#"{"results": [
          {"id": "1", "title": "<strong>Rust</strong> Engineer", "description": "Build things", "created": "2026-10-10T10:00:00Z",
           "redirect_url": "https://www.adzuna.in/land/ad/1", "company": {"display_name": "Acme"}, "location": {"display_name": "Bengaluru, Karnataka"},
           "salary_min": 2000000, "salary_max": 3000000, "salary_is_predicted": "0"},
          {"id": "2", "title": "Predicted", "redirect_url": "https://www.adzuna.in/land/ad/2", "salary_min": 100, "salary_is_predicted": "1"}
        ]}"#;
        let r: Results = serde_json::from_str(raw).unwrap();
        let jobs: Vec<_> = r.results.into_iter().filter_map(|a| map(a, "in:rust", "INR")).collect();
        assert_eq!(jobs[0].title, "Rust Engineer");
        assert_eq!(jobs[0].salary.as_ref().unwrap().currency.as_deref(), Some("INR"));
        assert!(jobs[1].salary.is_none());
    }
}
