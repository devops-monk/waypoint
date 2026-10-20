//! Personio's public XML job feed at `{company}.jobs.personio.de/xml` (no auth).
//! Reference: career-ops `providers/personio.mjs`.

use crate::http::Http;
use crate::text::{decode_entities, html_to_text, looks_remote, parse_date, xml_blocks, xml_text};
use crate::{SourceError, slug_ok};
use waypoint_core::JobDraft;

pub const HOSTS: &[&str] = &["*.jobs.personio.de", "*.jobs.personio.com"];

pub async fn fetch(http: &Http, slug: &str, company: &str) -> Result<Vec<JobDraft>, SourceError> {
    slug_ok(slug)?;
    if slug.contains('.') {
        return Err(SourceError::BadSlug(slug.into()));
    }
    let xml = http.get_text(&format!("https://{slug}.jobs.personio.de/xml"), HOSTS).await?;
    Ok(parse(&xml, slug, company))
}

fn parse(xml: &str, slug: &str, company: &str) -> Vec<JobDraft> {
    xml_blocks(xml, "position")
        .into_iter()
        .filter_map(|p| {
            let id = xml_text(p, "id");
            if id.is_empty() || !id.chars().all(|c| c.is_ascii_digit()) {
                return None;
            }
            // Description sections carry their own <name> tags; read the scalar fields without them.
            let descriptions = xml_blocks(p, "jobDescriptions").first().copied().unwrap_or("");
            let scalar = p.replace(descriptions, "");
            let title = xml_text(&scalar, "name");
            if title.is_empty() {
                return None;
            }
            let mut offices = vec![xml_text(&scalar, "office")];
            if let Some(extra) = xml_blocks(&scalar, "additionalOffices").first() {
                offices.extend(xml_blocks(extra, "office").into_iter().map(|o| decode_entities(o.trim())));
            }
            offices.retain(|o| !o.is_empty());
            offices.dedup();
            let location = offices.join(" · ");
            let description = xml_blocks(descriptions, "jobDescription")
                .into_iter()
                .map(|d| {
                    let heading = xml_text(d, "name");
                    let body = html_to_text(&xml_text(d, "value"));
                    if heading.is_empty() { body } else { format!("{heading}\n{body}") }
                })
                .filter(|t| !t.trim().is_empty())
                .collect::<Vec<_>>()
                .join("\n\n");
            let schedule = xml_text(&scalar, "schedule");
            Some(JobDraft {
                source: "personio".into(),
                board: Some(slug.into()),
                url: format!("https://{slug}.jobs.personio.de/job/{id}"),
                external_id: Some(id),
                company: company.into(),
                title,
                remote: looks_remote(&location),
                location,
                employment_type: Some(schedule).filter(|s| !s.is_empty()),
                department: Some(xml_text(&scalar, "department")).filter(|s| !s.is_empty()),
                description,
                salary: None,
                posted_at: parse_date(&xml_text(&scalar, "createdAt")),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_positions_with_descriptions() {
        let xml = r#"<?xml version="1.0"?><workzag-jobs><position>
            <id>1834171</id><office>Munich</office><additionalOffices><office>Berlin</office></additionalOffices>
            <department>Product and Tech</department><name>Staff Software Engineer</name>
            <jobDescriptions><jobDescription><name>Your role</name><value><![CDATA[<p>Build the data platform</p>]]></value></jobDescription></jobDescriptions>
            <schedule>full-time</schedule><createdAt>2024-11-13T14:10:41+00:00</createdAt>
        </position><position><id>abc</id><name>Bad</name></position></workzag-jobs>"#;
        let jobs = parse(xml, "personio", "Personio");
        assert_eq!(jobs.len(), 1);
        assert_eq!(jobs[0].title, "Staff Software Engineer");
        assert_eq!(jobs[0].location, "Munich · Berlin");
        assert_eq!(jobs[0].description, "Your role\nBuild the data platform");
        assert_eq!(jobs[0].url, "https://personio.jobs.personio.de/job/1834171");
    }
}
