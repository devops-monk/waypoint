//! Small text helpers shared by sources: HTML → readable plain text, remote detection, dates.

use std::sync::LazyLock;

use chrono::{DateTime, TimeZone, Utc};
use regex::Regex;

static BLOCK: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)</?(p|div|br|ul|ol|h[1-6]|tr|section|article)\b[^>]*>").unwrap());
static LI: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)<li\b[^>]*>").unwrap());
static LI_END: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)</li\s*>").unwrap());
static SCRIPT: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?is)<(script|style)\b.*?</(script|style)>").unwrap());
static TAG: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"<[^>]+>").unwrap());
static BLANKS: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\n[ \t]*\n(\s*\n)+").unwrap());
static SPACES: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[ \t\u{a0}]+").unwrap());
static REMOTE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)\b(remote|anywhere|distributed|work from home|wfh|telecommute)\b").unwrap());

pub fn decode_entities(s: &str) -> String {
    let mut out = s.replace("&nbsp;", " ").replace("&lt;", "<").replace("&gt;", ">").replace("&quot;", "\"").replace("&#39;", "'").replace("&#x27;", "'").replace("&rsquo;", "’").replace("&lsquo;", "‘").replace("&ldquo;", "“").replace("&rdquo;", "”").replace("&mdash;", "—").replace("&ndash;", "–").replace("&hellip;", "…").replace("&bull;", "•");
    // Numeric entities.
    static NUM: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"&#(x[0-9a-fA-F]+|\d+);").unwrap());
    out = NUM
        .replace_all(&out, |c: &regex::Captures| {
            let v = &c[1];
            let code = if let Some(hex) = v.strip_prefix('x') { u32::from_str_radix(hex, 16).ok() } else { v.parse().ok() };
            code.and_then(char::from_u32).map(String::from).unwrap_or_default()
        })
        .into_owned();
    out.replace("&amp;", "&")
}

/// Convert an HTML fragment (possibly entity-escaped, as Greenhouse sends it) to plain text
/// that keeps paragraph and bullet structure.
pub fn html_to_text(html: &str) -> String {
    if html.trim().is_empty() {
        return String::new();
    }
    // Greenhouse double-escapes: the HTML itself arrives as &lt;p&gt;...
    let html = if html.contains("&lt;") && !html.contains('<') { decode_entities(html) } else { html.to_string() };
    let s = SCRIPT.replace_all(&html, "");
    let s = LI_END.replace_all(&s, "");
    let s = LI.replace_all(&s, "\n• ");
    let s = BLOCK.replace_all(&s, "\n");
    let s = TAG.replace_all(&s, "");
    let s = decode_entities(&s);
    let s = SPACES.replace_all(&s, " ");
    let s: String = s.lines().map(str::trim).collect::<Vec<_>>().join("\n");
    BLANKS.replace_all(&s, "\n\n").trim().to_string()
}

pub fn looks_remote(location: &str) -> bool {
    REMOTE.is_match(location)
}

pub fn from_millis(ms: i64) -> Option<DateTime<Utc>> {
    Utc.timestamp_millis_opt(ms).single()
}

/// Every `<tag>…</tag>` block in an XML document, in order. Feeds here are flat and
/// machine-written, so a regex reader is enough and keeps a full XML parser out.
pub fn xml_blocks<'a>(xml: &'a str, tag: &str) -> Vec<&'a str> {
    let re = Regex::new(&format!(r"(?s)<{0}\b[^>]*>(.*?)</{0}>", regex::escape(tag))).expect("tag regex");
    re.captures_iter(xml).filter_map(|c| c.get(1)).map(|m| m.as_str()).collect()
}

/// Text of the first `<tag>` in a block, with CDATA unwrapped and entities decoded.
pub fn xml_text(block: &str, tag: &str) -> String {
    xml_blocks(block, tag).first().map(|t| {
        let t = t.trim();
        let t = t.strip_prefix("<![CDATA[").and_then(|t| t.strip_suffix("]]>")).unwrap_or(t);
        decode_entities(t).trim().to_string()
    }).unwrap_or_default()
}

pub fn parse_date(s: &str) -> Option<DateTime<Utc>> {
    let s = s.trim();
    DateTime::parse_from_rfc3339(s)
        .or_else(|_| DateTime::parse_from_rfc2822(s))
        .map(|d| d.with_timezone(&Utc))
        .ok()
        .or_else(|| chrono::NaiveDateTime::parse_from_str(s.get(..19)?, "%Y-%m-%dT%H:%M:%S").ok().map(|n| n.and_utc()))
        .or_else(|| chrono::NaiveDateTime::parse_from_str(s.get(..19)?, "%Y-%m-%d %H:%M:%S").ok().map(|n| n.and_utc()))
        .or_else(|| s.parse::<i64>().ok().filter(|n| *n > 1_000_000_000 && *n < 10_000_000_000).and_then(|n| Utc.timestamp_opt(n, 0).single()))
        .or_else(|| chrono::NaiveDate::parse_from_str(s.get(..10)?, "%Y-%m-%d").ok().and_then(|d| d.and_hms_opt(0, 0, 0)).map(|n| n.and_utc()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn html_to_text_keeps_structure() {
        let html = "&lt;h2&gt;About&lt;/h2&gt;&lt;p&gt;We build &amp;amp; ship.&lt;/p&gt;&lt;ul&gt;&lt;li&gt;Rust&lt;/li&gt;&lt;li&gt;SQL&lt;/li&gt;&lt;/ul&gt;";
        assert_eq!(html_to_text(html), "About\n\nWe build & ship.\n\n• Rust\n• SQL");
        assert_eq!(html_to_text("<p>a&nbsp;b</p><script>x()</script>"), "a b");
    }

    #[test]
    fn remote_and_dates() {
        assert!(looks_remote("Remote - EU"));
        assert!(!looks_remote("Copenhagen"));
        assert!(parse_date("2026-03-01T10:00:00Z").is_some());
        assert!(parse_date("2026-03-01T10:00:00").is_some());
        assert!(parse_date("2026-03-01").is_some());
        assert!(parse_date("Fri, 16 Oct 2026 17:38:10 +0200").is_some());
        assert!(parse_date("2026-10-14 11:37:16 UTC").is_some());
        assert!(parse_date("2026-10-18T07:01:29.578").is_some());
        assert_eq!(parse_date("1791341968").unwrap().timestamp(), 1791341968);
    }

    #[test]
    fn reads_simple_xml() {
        let xml = "<rss><item><title>A &amp; B</title><x/></item><item><title><![CDATA[C]]></title></item></rss>";
        let items = xml_blocks(xml, "item");
        assert_eq!(items.len(), 2);
        assert_eq!(xml_text(items[0], "title"), "A & B");
        assert_eq!(xml_text(items[1], "title"), "C");
        assert_eq!(xml_text(items[1], "missing"), "");
    }
}
