//! Job sources. Every source uses a public, documented-for-embedding endpoint (ATS job
//! board widgets, open job APIs). LinkedIn and Indeed are deliberately not scraped.

pub mod adzuna;
pub mod arbeitsagentur;
pub mod ashby;
pub mod greenhouse;
pub mod hackernews;
pub mod himalayas;
pub mod http;
pub mod jobnet;
pub mod lever;
pub mod personio;
pub mod platsbanken;
pub mod recruitee;
pub mod remoteok;
pub mod remotive;
pub mod smartrecruiters;
pub mod teamtailor;
pub mod text;
pub mod workable;
pub mod workday;

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
    #[error("add your {0} API key in Settings to use this source")]
    MissingKey(&'static str),
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

/// Search queries go into query strings (always URL-encoded), never paths.
pub(crate) fn query_ok(q: &str) -> Result<(), SourceError> {
    let ok = !q.trim().is_empty() && q.chars().count() <= 120 && !q.chars().any(char::is_control);
    if ok { Ok(()) } else { Err(SourceError::BadSlug(q.into())) }
}

/// "rust developer @ Berlin" → ("rust developer", Some("Berlin")).
pub fn split_query(q: &str) -> (&str, Option<&str>) {
    match q.split_once('@') {
        Some((what, place)) if !place.trim().is_empty() => (what.trim(), Some(place.trim())),
        Some((what, _)) => (what.trim(), None),
        None => (q.trim(), None),
    }
}

/// Every word of the query appears in the text, for feeds without server-side search.
pub(crate) fn matches_query(text: &str, query: &str) -> bool {
    let text = text.to_lowercase();
    split_query(query).0.split_whitespace().all(|w| text.contains(&w.to_lowercase()))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderInfo {
    pub id: &'static str,
    pub name: &'static str,
    /// "board" sources track one company; "search" sources take a keyword query.
    pub kind: &'static str,
    pub example: &'static str,
    /// Where it's most useful, shown when choosing a search source.
    pub coverage: &'static str,
    /// Whether the search understands "keywords @ place".
    pub place: bool,
    /// An API key the user must add first.
    pub needs_key: Option<&'static str>,
}

const fn board(id: &'static str, name: &'static str, example: &'static str) -> ProviderInfo {
    ProviderInfo { id, name, kind: "board", example, coverage: "", place: false, needs_key: None }
}

const fn search(id: &'static str, name: &'static str, example: &'static str, coverage: &'static str, place: bool) -> ProviderInfo {
    ProviderInfo { id, name, kind: "search", example, coverage, place, needs_key: None }
}

pub const PROVIDERS: &[ProviderInfo] = &[
    board("greenhouse", "Greenhouse", "https://job-boards.greenhouse.io/anthropic"),
    board("lever", "Lever", "https://jobs.lever.co/mistral"),
    board("lever_eu", "Lever (EU)", "https://jobs.eu.lever.co/diabolocom"),
    board("ashby", "Ashby", "https://jobs.ashbyhq.com/supabase"),
    board("workday", "Workday", "https://nvidia.wd5.myworkdayjobs.com/NVIDIAExternalCareerSite"),
    board("smartrecruiters", "SmartRecruiters", "https://jobs.smartrecruiters.com/BoschGroup"),
    board("workable", "Workable", "https://apply.workable.com/huggingface"),
    board("recruitee", "Recruitee", "https://bunq.recruitee.com"),
    board("personio", "Personio", "https://personio.jobs.personio.de"),
    board("teamtailor", "Teamtailor", "https://lunar.teamtailor.com/jobs"),
    search("remotive", "Remotive", "rust", "Remote roles worldwide", false),
    search("himalayas", "Himalayas", "product designer", "Remote roles worldwide, often with salaries", false),
    search("remoteok", "Remote OK", "data engineer", "Remote roles worldwide", false),
    search("hackernews", "Hacker News", "rust", "This month's “Who is hiring?” thread: startups, often remote", false),
    ProviderInfo { id: "adzuna", name: "Adzuna", kind: "search", example: "in:backend engineer @ Bengaluru", coverage: "US, UK, India and most of Europe", place: true, needs_key: Some("Adzuna") },
    search("arbeitsagentur", "Arbeitsagentur", "Softwareentwickler @ Berlin", "Germany's national job board (Bundesagentur für Arbeit)", true),
    search("platsbanken", "Platsbanken", "utvecklare @ Göteborg", "Sweden's national job board", true),
    search("jobnet", "Jobnet", "udvikler @ Aarhus", "Denmark's national job board", true),
];

/// Keys for sources that need the user's own API credentials.
#[derive(Debug, Clone, Default)]
pub struct Keys {
    pub adzuna: Option<adzuna::AdzunaKey>,
}

/// Fetch every posting from one board (or one search, for search-kind providers).
pub async fn fetch(http: &Http, provider: &str, slug: &str, company: &str, keys: &Keys) -> Result<Vec<JobDraft>, SourceError> {
    match provider {
        "greenhouse" => greenhouse::fetch(http, slug, company).await,
        "lever" => lever::fetch(http, slug, company, false).await,
        "lever_eu" => lever::fetch(http, slug, company, true).await,
        "ashby" => ashby::fetch(http, slug, company).await,
        "workday" => workday::fetch(http, slug, company).await,
        "smartrecruiters" => smartrecruiters::fetch(http, slug, company).await,
        "workable" => workable::fetch(http, slug, company).await,
        "recruitee" => recruitee::fetch(http, slug, company).await,
        "personio" => personio::fetch(http, slug, company).await,
        "teamtailor" => teamtailor::fetch(http, slug, company).await,
        "remotive" => remotive::fetch(http, slug).await,
        "himalayas" => himalayas::fetch(http, slug).await,
        "remoteok" => remoteok::fetch(http, slug).await,
        "hackernews" => hackernews::fetch(http, slug).await,
        "adzuna" => adzuna::fetch(http, slug, keys.adzuna.as_ref()).await,
        "arbeitsagentur" => arbeitsagentur::fetch(http, slug).await,
        "platsbanken" => platsbanken::fetch(http, slug).await,
        "jobnet" => jobnet::fetch(http, slug).await,
        other => Err(SourceError::UnknownProvider(other.into())),
    }
}

/// Sources whose listings omit descriptions can fetch one job's text on demand.
pub fn has_lazy_description(provider: &str) -> bool {
    matches!(provider, "workday" | "smartrecruiters")
}

/// The full description of one job from a source that lists without them.
pub async fn fetch_description(http: &Http, provider: &str, board: &str, url: &str) -> Result<Option<String>, SourceError> {
    match provider {
        "workday" => workday::fetch_description(http, board, url).await,
        "smartrecruiters" => smartrecruiters::fetch_description(http, board, url).await,
        _ => Ok(None),
    }
}

/// Validate a board slug or search query for a provider before it's saved.
pub fn validate(provider: &str, slug: &str) -> Result<(), SourceError> {
    let p = PROVIDERS.iter().find(|p| p.id == provider).ok_or_else(|| SourceError::UnknownProvider(provider.into()))?;
    match (p.kind, provider) {
        (_, "workday") => workday::Site::parse(slug).map(|_| ()),
        (_, "adzuna") => query_ok(slug).and_then(|_| adzuna::parse_query(slug).map(|_| ())),
        ("search", _) => query_ok(slug),
        _ => slug_ok(slug),
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
    let segments: Vec<&str> = url.path_segments()?.filter(|s| !s.is_empty()).collect();
    let first = segments.first().map(|s| s.to_string());
    // Subdomain boards: {slug}.recruitee.com, {slug}.jobs.personio.de, {slug}.teamtailor.com.
    let sub = |suffix: &str| host.strip_suffix(suffix).filter(|s| !s.is_empty() && !s.contains('.') && *s != "www").map(str::to_string);
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
        "jobs.smartrecruiters.com" | "careers.smartrecruiters.com" => found("smartrecruiters", first),
        "apply.workable.com" => found("workable", first.filter(|s| s != "j" && s != "api")),
        h if h.ends_with(".myworkdayjobs.com") => found("workday", workday::detect(h, &segments)),
        h if h.ends_with(".recruitee.com") => found("recruitee", sub(".recruitee.com")),
        h if h.ends_with(".jobs.personio.de") => found("personio", sub(".jobs.personio.de")),
        h if h.ends_with(".jobs.personio.com") => found("personio", sub(".jobs.personio.com")),
        h if h.ends_with(".teamtailor.com") => found("teamtailor", sub(".teamtailor.com")),
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
        assert_eq!(detect("https://nvidia.wd5.myworkdayjobs.com/en-US/NVIDIAExternalCareerSite"), d("workday", "nvidia.wd5.NVIDIAExternalCareerSite"));
        assert_eq!(detect("https://jobs.smartrecruiters.com/BoschGroup/744000156179239-x"), d("smartrecruiters", "BoschGroup"));
        assert_eq!(detect("https://apply.workable.com/huggingface/"), d("workable", "huggingface"));
        assert_eq!(detect("https://apply.workable.com/j/81B46579FE"), None);
        assert_eq!(detect("https://bunq.recruitee.com/o/head-of-product"), d("recruitee", "bunq"));
        assert_eq!(detect("https://personio.jobs.personio.com/job/1"), d("personio", "personio"));
        assert_eq!(detect("https://lunar.teamtailor.com/jobs"), d("teamtailor", "lunar"));
        assert_eq!(detect("https://a.b.teamtailor.com/jobs"), None);
    }

    #[test]
    fn rejects_lookalikes_and_unsafe_slugs() {
        assert_eq!(detect("http://jobs.lever.co/mistral"), None);
        assert_eq!(detect("https://example.com/jobs?for=stripe"), None);
        assert_eq!(detect("https://jobs.lever.co.evil.test/x"), None);
        assert!(slug_ok("../etc").is_err());
        assert!(slug_ok("a/b").is_err());
        assert!(validate("workday", "acme.wd1.External").is_ok());
        assert!(validate("workday", "acme").is_err());
        assert!(validate("adzuna", "gb:data analyst @ Leeds").is_ok());
        assert!(validate("adzuna", "data analyst").is_err());
        assert!(validate("himalayas", "product designer").is_ok());
        assert!(validate("greenhouse", "a b").is_err());
    }

    #[test]
    fn queries_split_on_place() {
        assert_eq!(split_query("rust developer @ Berlin"), ("rust developer", Some("Berlin")));
        assert_eq!(split_query("rust"), ("rust", None));
        assert!(matches_query("Senior RUST engineer, remote", "rust engineer @ anywhere"));
        assert!(!matches_query("Go engineer", "rust"));
    }

    #[test]
    fn seeds_parse_and_are_valid() {
        let seeds = seed_boards();
        assert!(seeds.len() > 50);
        assert!(seeds.iter().all(|s| slug_ok(&s.slug).is_ok() && PROVIDERS.iter().any(|p| p.id == s.provider)));
    }
}
