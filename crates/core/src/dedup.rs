//! Deterministic dedup keys for job postings.
//!
//! `job_key` is a port of ai-job-search `tools/job_key.py` (MIT): a pure function of
//! company + title (+ URL fallback) whose truncation is disambiguated by a hash of the
//! full slug, so a long title always keys the same way and two long titles never collide.
//! `canonical_url` strips tracking noise so the same posting reached via different links
//! dedups by URL first, the way career-ops `url-key.mjs` does.

use unicode_normalization::UnicodeNormalization;
use unicode_script::{Script, UnicodeScript};
use url::Url;

const COMPANY_MAX: usize = 40;
const TITLE_MAX: usize = 60;
const HASH_LEN: usize = 6;

fn sha1_short(text: &str) -> String {
    let hex = sha1_smol::Sha1::from(text.as_bytes()).digest().to_string();
    hex[..HASH_LEN].to_string()
}

/// Lowercase ASCII slug; anything outside `[a-z0-9]` becomes a single `-`.
/// Non-Latin scripts legitimately reduce to "".
pub fn slugify(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut pending_sep = false;
    for ch in text.nfkd().filter(char::is_ascii).flat_map(|c| c.to_lowercase()) {
        if ch.is_ascii_alphanumeric() {
            if pending_sep && !out.is_empty() {
                out.push('-');
            }
            pending_sep = false;
            out.push(ch);
        } else {
            pending_sep = true;
        }
    }
    out
}

/// True when the ASCII fold dropped non-Latin letters, so the slug under-identifies.
/// Latin letters without an ASCII decomposition ("ø", "ß") are deliberately not counted.
fn lost_letters(text: &str) -> bool {
    text.nfkd()
        .any(|ch| !ch.is_ascii() && ch.is_alphabetic() && !matches!(ch.script(), Script::Latin | Script::Common | Script::Inherited))
}

fn cap(slug: &str, limit: usize) -> String {
    if slug.len() <= limit {
        return slug.to_string();
    }
    // Slugs are pure ASCII, so byte slicing is char-safe.
    format!("{}-{}", slug[..limit].trim_end_matches('-'), sha1_short(slug))
}

fn job_id_in(url: &str) -> Option<&str> {
    let bytes = url.as_bytes();
    let mut start = None;
    for (i, b) in bytes.iter().enumerate() {
        match (b.is_ascii_digit(), start) {
            (true, None) => start = Some(i),
            (false, Some(s)) if i - s >= 6 => return Some(&url[s..i]),
            (false, Some(_)) => start = None,
            _ => {}
        }
    }
    start.filter(|s| bytes.len() - s >= 6).map(|s| &url[s..])
}

/// The canonical `<company-slug>_<title-slug>` key for one posting.
pub fn job_key(company: &str, title: &str, url: &str) -> String {
    let mut company_slug = cap(&slugify(company), COMPANY_MAX);
    if company_slug.is_empty() || lost_letters(company) {
        let name: String = company.trim().to_lowercase().nfc().collect();
        company_slug = if name.is_empty() {
            "unknown-company".into()
        } else if company_slug.is_empty() {
            format!("company-{}", sha1_short(&name))
        } else {
            format!("{company_slug}-{}", sha1_short(&name))
        };
    }

    let mut title_slug = cap(&slugify(title), TITLE_MAX);
    if title_slug.is_empty() || lost_letters(title) {
        let fragment = title_slug.clone();
        title_slug = if let Some(id) = job_id_in(url) {
            if fragment.is_empty() { id.to_string() } else { format!("{fragment}-{id}") }
        } else {
            let basis = slugify(if title.is_empty() { url } else { title });
            let digest = sha1_short(if url.is_empty() { title } else { url });
            if !fragment.is_empty() {
                format!("{fragment}-{digest}")
            } else if !basis.is_empty() {
                basis
            } else {
                format!("untitled-{digest}")
            }
        };
    }
    format!("{company_slug}_{title_slug}")
}

const TRACKING_PARAMS: &[&str] = &["gh_src", "gh_jid", "lever-source", "lever-origin", "source", "src", "ref", "referrer", "trk", "trackingid", "refid", "fbclid", "gclid", "mc_cid", "mc_eid"];

/// Normalize a posting URL for dedup: lowercase host, drop the DNS root dot, `www.`,
/// fragments, trailing slashes and tracking params (`utm_*` and known ATS referrers).
/// Returns the input unchanged when it does not parse.
pub fn canonical_url(raw: &str) -> String {
    let Ok(mut url) = Url::parse(raw.trim()) else {
        return raw.trim().to_string();
    };
    if let Some(host) = url.host_str() {
        let host = host.trim_end_matches('.').trim_start_matches("www.").to_ascii_lowercase();
        let _ = url.set_host(Some(&host));
    }
    url.set_fragment(None);
    let kept: Vec<(String, String)> = url
        .query_pairs()
        .filter(|(k, _)| {
            let k = k.to_ascii_lowercase();
            !k.starts_with("utm_") && !TRACKING_PARAMS.contains(&k.as_str())
        })
        .map(|(k, v)| (k.into_owned(), v.into_owned()))
        .collect();
    if kept.is_empty() {
        url.set_query(None);
    } else {
        url.query_pairs_mut().clear().extend_pairs(kept);
    }
    let mut out = url.to_string();
    while out.ends_with('/') && url.query().is_none() {
        out.pop();
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slug_drops_path_breaking_punctuation() {
        assert_eq!(job_key("Deloitte", "Junior Cybersecurity Analyst (OT/IoT)", ""), "deloitte_junior-cybersecurity-analyst-ot-iot");
        assert_eq!(job_key("Ops Consulting, LLC", "Malware Analyst", ""), "ops-consulting-llc_malware-analyst");
    }

    #[test]
    fn long_titles_are_deterministic_and_distinct() {
        let a = job_key("Deloitte", "Cyber Intelligence Center Security Analyst at the Copenhagen Office for Clients", "");
        let b = job_key("Deloitte", "Cyber Intelligence Center Security Analyst at the Copenhagen Office for Banks", "");
        assert_eq!(a, job_key("Deloitte", "Cyber Intelligence Center Security Analyst at the Copenhagen Office for Clients", ""));
        assert_ne!(a, b);
        assert!(a.split('_').nth(1).unwrap().len() <= TITLE_MAX + 1 + HASH_LEN);
    }

    #[test]
    fn latin_diacritics_fold_without_hash() {
        // Same as job_key.py: "ø" has no ASCII decomposition and is dropped, not separated.
        assert_eq!(job_key("Ørsted", "Ingeniør", ""), "rsted_ingenir");
        assert_eq!(job_key("Müller GmbH", "Entwickler", ""), "muller-gmbh_entwickler");
    }

    #[test]
    fn non_latin_titles_fall_back_to_url_id() {
        let a = job_key("Securion", "Программист 1С", "https://hh.ru/vacancy/12345678");
        let b = job_key("Securion", "Аналитик 1С", "https://hh.ru/vacancy/87654321");
        assert_eq!(a, "securion_1-12345678");
        assert_ne!(a, b);
    }

    #[test]
    fn mixed_script_companies_get_identity_hash() {
        assert_ne!(job_key("Сбер AI", "Engineer", ""), job_key("Яндекс AI", "Engineer", ""));
        assert_eq!(job_key("", "Engineer", ""), "unknown-company_engineer");
    }

    #[test]
    fn canonical_url_strips_tracking() {
        assert_eq!(
            canonical_url("https://WWW.Boards.Greenhouse.io/acme/jobs/123/?gh_src=abc&utm_source=li#apply"),
            "https://boards.greenhouse.io/acme/jobs/123"
        );
        assert_eq!(canonical_url("https://jobs.lever.co/acme/x?lever-source=LinkedIn&team=eng"), "https://jobs.lever.co/acme/x?team=eng");
    }
}
