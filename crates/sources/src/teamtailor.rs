//! Teamtailor's public RSS feed at `{company}.teamtailor.com/jobs.rss` (no auth). Common
//! with Nordic employers. Reference: career-ops `providers/teamtailor.mjs`.

use crate::http::Http;
use crate::text::{html_to_text, parse_date, xml_blocks, xml_text};
use crate::{SourceError, slug_ok};
use waypoint_core::JobDraft;

pub const HOSTS: &[&str] = &["*.teamtailor.com"];

pub async fn fetch(http: &Http, slug: &str, company: &str) -> Result<Vec<JobDraft>, SourceError> {
    slug_ok(slug)?;
    if slug.contains('.') {
        return Err(SourceError::BadSlug(slug.into()));
    }
    let xml = http.get_text(&format!("https://{slug}.teamtailor.com/jobs.rss"), HOSTS).await?;
    Ok(parse(&xml, slug, company))
}

fn parse(xml: &str, slug: &str, company: &str) -> Vec<JobDraft> {
    xml_blocks(xml, "item")
        .into_iter()
        .filter_map(|item| {
            let url = xml_text(item, "link");
            let title = xml_text(item, "title");
            if !url.starts_with("https://") || title.is_empty() {
                return None;
            }
            let remote_status = xml_text(item, "remoteStatus").to_lowercase();
            let remote = matches!(remote_status.as_str(), "fully" | "temporary");
            let mut places: Vec<String> = xml_blocks(item, "tt:location").into_iter().map(|l| xml_text(l, "tt:name")).filter(|n| !n.is_empty()).collect();
            places.dedup();
            if remote {
                places.push("Remote".into());
            } else if remote_status == "hybrid" {
                places.push("Hybrid".into());
            }
            Some(JobDraft {
                source: "teamtailor".into(),
                board: Some(slug.into()),
                external_id: Some(xml_text(item, "guid")).filter(|g| !g.is_empty()),
                url,
                company: company.into(),
                title,
                location: places.join(" · "),
                remote,
                employment_type: None,
                department: Some(xml_text(item, "tt:department")).filter(|d| !d.is_empty()),
                description: html_to_text(&xml_text(item, "description")),
                salary: None,
                posted_at: parse_date(&xml_text(item, "pubDate")),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_feed_items() {
        let xml = r#"<rss xmlns:tt="https://teamtailor.com/locations"><channel><title>Lunar</title><link>https://lunar.teamtailor.com/jobs</link>
          <item><title>Team Lead</title><description>&lt;p&gt;Lead the team&lt;/p&gt;</description><pubDate>Fri, 16 Oct 2026 17:38:10 +0200</pubDate>
          <link>https://jobs.lunar.app/jobs/8563466-team-lead</link><remoteStatus>hybrid</remoteStatus><guid>0ad1</guid>
          <tt:locations><tt:location><tt:name>Copenhagen, Denmark</tt:name><tt:city>København</tt:city></tt:location></tt:locations>
          <tt:department>FCP</tt:department></item></channel></rss>"#;
        let jobs = parse(xml, "lunar", "Lunar");
        assert_eq!(jobs.len(), 1);
        assert_eq!(jobs[0].location, "Copenhagen, Denmark · Hybrid");
        assert_eq!(jobs[0].description, "Lead the team");
        assert_eq!(jobs[0].department.as_deref(), Some("FCP"));
        assert!(jobs[0].posted_at.is_some());
    }
}
