//! Job sources. Every source uses a public, documented-for-embedding endpoint (ATS job
//! board widgets, open job APIs). LinkedIn and Indeed are deliberately not scraped.

pub mod ashby;
pub mod greenhouse;
pub mod http;
pub mod lever;
pub mod remotive;
pub mod text;

use serde::{Deserialize, Serialize};
use url::Url;

use waypoint_core::JobDraft;
pub use http::Http;

#[derive(Debug, thiserror::Error)]
pub enum SourceError {
    #[error("invalid URL: {0}")]
    BadUrl(String),
    #[error("refusing to contact untrusted host {0:?}")]
    UntrustedHost(String),
    #[error("invalid board slug {0:?}")]
    BadSlug(String),
    #[error("unknown provider {0:?}")]
    UnknownProvider(String),
    #[error("HTTP {0} {1}")]
    Status(u16, String),
    #[error(transparent)]
    Request(#[from] reqwest::Error),
}

/// Slugs are interpolated into URL paths, so only a conservative alphabet is allowed.
pub(crate) fn slug_ok(slug: &str) -> Result<(), SourceError> {
    let ok = !slug.is_empty() && slug.len() <= 100 && slug.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.')) && !slug.contains("..");
    if ok { Ok(()) } else { Err(SourceError::BadSlug(slug.into())) }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderInfo {
    pub id: &'static str,
    pub name: &'static str,
    /// "board" sources track one company; "search" sources take a keyword query.
    pub kind: &'static str,
    pub example: &'static str,
}

pub const PROVIDERS: &[ProviderInfo] = &[
    ProviderInfo { id: "greenhouse", name: "Greenhouse", kind: "board", example: "https://job-boards.greenhouse.io/anthropic" },
    ProviderInfo { id: "lever", name: "Lever", kind: "board", example: "https://jobs.lever.co/mistral" },
    ProviderInfo { id: "lever_eu", name: "Lever (EU)", kind: "board", example: "https://jobs.eu.lever.co/diabolocom" },
    ProviderInfo { id: "ashby", name: "Ashby", kind: "board", example: "https://jobs.ashbyhq.com/supabase" },
    ProviderInfo { id: "remotive", name: "Remotive", kind: "search", example: "rust" },
];

/// Fetch every posting from one board (or one search, for search-kind providers).
pub async fn fetch(http: &Http, provider: &str, slug: &str, company: &str) -> Result<Vec<JobDraft>, SourceError> {
    match provider {
        "greenhouse" => greenhouse::fetch(http, slug, company).await,
        "lever" => lever::fetch(http, slug, company, false).await,
        "lever_eu" => lever::fetch(http, slug, company, true).await,
        "ashby" => ashby::fetch(http, slug, company).await,
        "remotive" => remotive::fetch(http, slug).await,
        other => Err(SourceError::UnknownProvider(other.into())),
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DetectedBoard {
    pub provider: String,
    pub slug: String,
}

/// Recognize a careers-page URL and return the board it points at.
/// Port of the `resolveApiUrl` logic in career-ops greenhouse/lever/ashby providers.
pub fn detect(careers_url: &str) -> Option<DetectedBoard> {
    let url = Url::parse(careers_url.trim()).ok()?;
    if url.scheme() != "https" {
        return None;
    }
    let host = url.host_str()?.to_ascii_lowercase();
    let first = url.path_segments()?.find(|s| !s.is_empty()).map(str::to_string);
    let found = |provider: &str, slug: Option<String>| slug.filter(|s| slug_ok(s).is_ok()).map(|slug| DetectedBoard { provider: provider.into(), slug });
    match host.as_str() {
        "job-boards.greenhouse.io" | "job-boards.eu.greenhouse.io" | "boards.greenhouse.io" | "boards.eu.greenhouse.io" => {
            // Embed boards carry the token in ?for= and the path is literally "embed".
            let slug = match first.as_deref() {
                Some("embed") | None => url.query_pairs().find(|(k, _)| k == "for").map(|(_, v)| v.into_owned()),
                _ => first,
            };
            found("greenhouse", slug)
        }
        "boards-api.greenhouse.io" => {
            let segs: Vec<&str> = url.path_segments()?.collect();
            found("greenhouse", segs.iter().position(|s| *s == "boards").and_then(|i| segs.get(i + 1)).map(|s| s.to_string()))
        }
        "jobs.lever.co" => found("lever", first),
        "jobs.eu.lever.co" => found("lever_eu", first),
        "jobs.ashbyhq.com" => found("ashby", first),
        _ => None,
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct SeedBoard {
    pub provider: String,
    pub slug: String,
    pub company: String,
}

/// Starter list of companies with public ATS boards, derived from career-ops
/// `templates/portals.example.yml` (MIT). Users pick which to track during onboarding.
pub fn seed_boards() -> Vec<SeedBoard> {
    serde_json::from_str(include_str!("seed_boards.json")).expect("valid seed_boards.json")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(p: &str, s: &str) -> Option<DetectedBoard> {
        Some(DetectedBoard { provider: p.into(), slug: s.into() })
    }

    #[test]
    fn detects_known_boards() {
        assert_eq!(detect("https://job-boards.greenhouse.io/anthropic/jobs/123"), d("greenhouse", "anthropic"));
        assert_eq!(detect("https://boards.greenhouse.io/embed/job_board?for=stripe"), d("greenhouse", "stripe"));
        assert_eq!(detect("https://boards-api.greenhouse.io/v1/boards/acme/jobs"), d("greenhouse", "acme"));
        assert_eq!(detect("https://jobs.lever.co/mistral"), d("lever", "mistral"));
        assert_eq!(detect("https://jobs.eu.lever.co/diabolocom/abc"), d("lever_eu", "diabolocom"));
        assert_eq!(detect("https://jobs.ashbyhq.com/lakera.ai"), d("ashby", "lakera.ai"));
    }

    #[test]
    fn rejects_lookalikes_and_unsafe_slugs() {
        assert_eq!(detect("http://jobs.lever.co/mistral"), None);
        assert_eq!(detect("https://example.com/jobs?for=stripe"), None);
        assert_eq!(detect("https://jobs.lever.co.evil.test/x"), None);
        assert!(slug_ok("../etc").is_err());
        assert!(slug_ok("a/b").is_err());
    }

    #[test]
    fn seeds_parse_and_are_valid() {
        let seeds = seed_boards();
        assert!(seeds.len() > 50);
        assert!(seeds.iter().all(|s| slug_ok(&s.slug).is_ok() && PROVIDERS.iter().any(|p| p.id == s.provider)));
    }
}
