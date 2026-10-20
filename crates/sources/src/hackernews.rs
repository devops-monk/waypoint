//! Hacker News "Ask HN: Who is hiring?", through the Algolia HN API (no auth). Reads the
//! newest monthly thread and keeps top-level comments that match the keyword. Each job
//! links to its comment. Reference: career-ops `providers/hackernews.mjs`.

use serde::Deserialize;

use crate::http::Http;
use crate::text::{html_to_text, looks_remote, parse_date};
use crate::{SourceError, matches_query, query_ok};
use waypoint_core::JobDraft;

pub const HOSTS: &[&str] = &["hn.algolia.com"];

#[derive(Deserialize)]
struct Search {
    #[serde(default)]
    hits: Vec<Hit>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Hit {
    #[serde(rename = "objectID")]
    object_id: String,
    title: Option<String>,
}

#[derive(Deserialize)]
struct Item {
    id: Option<i64>,
    text: Option<String>,
    created_at: Option<String>,
    #[serde(default)]
    children: Vec<Item>,
}

pub async fn fetch(http: &Http, query: &str) -> Result<Vec<JobDraft>, SourceError> {
    query_ok(query)?;
    let s: Search = http.get_json("https://hn.algolia.com/api/v1/search_by_date?tags=story,author_whoishiring&hitsPerPage=10", HOSTS).await?;
    let Some(thread) = s.hits.into_iter().find(|h| h.title.as_deref().is_some_and(|t| t.contains("Who is hiring")) && h.object_id.chars().all(|c| c.is_ascii_digit())) else {
        return Ok(vec![]);
    };
    let item: Item = http.get_json(&format!("https://hn.algolia.com/api/v1/items/{}", thread.object_id), HOSTS).await?;
    Ok(item.children.into_iter().filter_map(|c| map(c, query)).collect())
}

/// Posts follow "Company | Role | Location | Remote | …" on the first line.
fn map(c: Item, query: &str) -> Option<JobDraft> {
    let id = c.id?;
    let text = html_to_text(c.text.as_deref()?);
    if !matches_query(&text, query) {
        return None;
    }
    let first = text.lines().next()?.trim();
    let parts: Vec<&str> = first.split('|').map(str::trim).filter(|p| !p.is_empty()).collect();
    if parts.len() < 2 {
        return None;
    }
    let strip_url = |s: &str| s.split_whitespace().filter(|w| !w.starts_with("http")).collect::<Vec<_>>().join(" ");
    let company = strip_url(parts[0]);
    let title = strip_url(parts[1]);
    let location = parts.iter().skip(2).map(|p| strip_url(p)).filter(|p| !p.is_empty() && p.len() < 60).take(2).collect::<Vec<_>>().join(" · ");
    if company.is_empty() || title.is_empty() {
        return None;
    }
    Some(JobDraft {
        source: "hackernews".into(),
        board: Some(query.into()),
        external_id: Some(id.to_string()),
        url: format!("https://news.ycombinator.com/item?id={id}"),
        company,
        title,
        remote: looks_remote(first),
        location,
        employment_type: None,
        department: None,
        description: text,
        salary: None,
        posted_at: c.created_at.as_deref().and_then(parse_date),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_hiring_comments() {
        let item: Item = serde_json::from_str(r#"{"id": 1, "children": [
          {"id": 11, "created_at": "2026-10-01T15:00:00Z", "text": "Acme | Senior Rust Engineer | Berlin | REMOTE (EU) | <a href=\"https://acme.test\">acme.test</a><p>We build databases in Rust."},
          {"id": 12, "text": "Globex | Designer | NYC<p>Figma all day."},
          {"id": 13, "text": "Just a reply about rust"}
        ]}"#).unwrap();
        let jobs: Vec<_> = item.children.into_iter().filter_map(|c| map(c, "rust")).collect();
        assert_eq!(jobs.len(), 1);
        assert_eq!(jobs[0].company, "Acme");
        assert_eq!(jobs[0].title, "Senior Rust Engineer");
        assert_eq!(jobs[0].location, "Berlin · REMOTE (EU)");
        assert!(jobs[0].remote);
        assert_eq!(jobs[0].url, "https://news.ycombinator.com/item?id=11");
    }
}
