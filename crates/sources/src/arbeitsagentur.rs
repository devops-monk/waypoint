//! Germany's Federal Employment Agency job search (Bundesagentur für Arbeit), using the
//! public client key its own website uses. Reference: career-ops `providers/arbeitsagentur.mjs`.
//! Query: "keywords" or "keywords @ city". The listing carries no description text.

use serde::Deserialize;

use crate::http::Http;
use crate::text::{looks_remote, parse_date};
use crate::{SourceError, query_ok, split_query};
use waypoint_core::JobDraft;

pub const HOSTS: &[&str] = &["rest.arbeitsagentur.de"];
const CLIENT_KEY: &str = "jobboerse-jobsuche";

#[derive(Deserialize)]
struct Results {
    #[serde(default)]
    ergebnisliste: Vec<Offer>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Offer {
    referenznummer: Option<String>,
    stellenangebots_titel: Option<String>,
    firma: Option<String>,
    hauptberuf: Option<String>,
    #[serde(default)]
    stellenlokationen: Vec<Lokation>,
    #[serde(default)]
    homeofficemoeglich: bool,
    #[serde(default)]
    arbeitszeit_vollzeit: bool,
    vertragsdauer: Option<String>,
    externe_url: Option<String>,
    datum_erste_veroeffentlichung: Option<String>,
}

#[derive(Deserialize)]
struct Lokation {
    adresse: Option<Adresse>,
}

#[derive(Deserialize)]
struct Adresse {
    ort: Option<String>,
    land: Option<String>,
}

pub async fn fetch(http: &Http, query: &str) -> Result<Vec<JobDraft>, SourceError> {
    query_ok(query)?;
    let (what, place) = split_query(query);
    let mut url = url::Url::parse("https://rest.arbeitsagentur.de/jobboerse/jobsuche-service/pc/v6/jobs").expect("static url");
    {
        let mut q = url.query_pairs_mut();
        q.append_pair("was", what).append_pair("size", "100").append_pair("page", "1").append_pair("angebotsart", "1").append_pair("veroeffentlichtseit", "30");
        if let Some(p) = place {
            q.append_pair("wo", p).append_pair("umkreis", "50");
        }
    }
    let r: Results = http.get_json_with(url.as_str(), HOSTS, &[("X-API-Key", CLIENT_KEY)]).await?;
    Ok(r.ergebnisliste.into_iter().filter_map(|o| map(o, query)).collect())
}

fn map(o: Offer, query: &str) -> Option<JobDraft> {
    let refnr = o.referenznummer.filter(|r| r.chars().all(|c| c.is_ascii_alphanumeric() || c == '-'))?;
    let title = o.stellenangebots_titel.filter(|t| !t.trim().is_empty())?;
    let mut places: Vec<String> = o
        .stellenlokationen
        .into_iter()
        .filter_map(|l| l.adresse)
        .map(|a| {
            let land = a.land.filter(|l| l != "DEUTSCHLAND").map(|l| titlecase(&l));
            [a.ort, land].into_iter().flatten().collect::<Vec<_>>().join(", ")
        })
        .filter(|p| !p.is_empty())
        .collect();
    places.dedup();
    if o.homeofficemoeglich {
        places.push("Homeoffice möglich".into());
    }
    let location = places.join(" · ");
    let facts: Vec<String> = [
        o.hauptberuf.map(|h| format!("Beruf: {h}")),
        Some(if o.arbeitszeit_vollzeit { "Vollzeit".to_string() } else { "Teilzeit".to_string() }),
        o.vertragsdauer.map(|v| if v == "UNBEFRISTET" { "Unbefristet".into() } else { "Befristet".into() }),
    ]
    .into_iter()
    .flatten()
    .collect();
    Some(JobDraft {
        source: "arbeitsagentur".into(),
        board: Some(query.into()),
        url: o.externe_url.filter(|u| u.starts_with("https://")).unwrap_or_else(|| format!("https://www.arbeitsagentur.de/jobsuche/jobdetail/{refnr}")),
        external_id: Some(refnr),
        company: o.firma.unwrap_or_default(),
        title,
        remote: o.homeofficemoeglich || looks_remote(&location),
        location,
        employment_type: None,
        department: None,
        description: format!("{}\n\nDie vollständige Stellenbeschreibung steht auf der verlinkten Seite.", facts.join(". ")),
        salary: None,
        posted_at: o.datum_erste_veroeffentlichung.as_deref().and_then(parse_date),
    })
}

fn titlecase(s: &str) -> String {
    let lower = s.to_lowercase();
    let mut c = lower.chars();
    c.next().map(|f| f.to_uppercase().collect::<String>() + c.as_str()).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_offers() {
        let raw = r#"{"ergebnisliste": [
          {"referenznummer": "11949-17265300-S", "stellenangebotsTitel": "Softwareentwickler/in", "firma": "Micro-Epsilon", "hauptberuf": "Softwareentwickler/in",
           "stellenlokationen": [{"adresse": {"ort": "Innsbruck", "land": "OESTERREICH"}}], "homeofficemoeglich": true, "arbeitszeitVollzeit": true,
           "vertragsdauer": "UNBEFRISTET", "datumErsteVeroeffentlichung": "2026-10-18"},
          {"referenznummer": "../x", "stellenangebotsTitel": "Bad"}
        ]}"#;
        let r: Results = serde_json::from_str(raw).unwrap();
        let jobs: Vec<_> = r.ergebnisliste.into_iter().filter_map(|o| map(o, "rust")).collect();
        assert_eq!(jobs.len(), 1);
        assert_eq!(jobs[0].location, "Innsbruck, Oesterreich · Homeoffice möglich");
        assert_eq!(jobs[0].url, "https://www.arbeitsagentur.de/jobsuche/jobdetail/11949-17265300-S");
        assert!(jobs[0].remote);
    }
}
