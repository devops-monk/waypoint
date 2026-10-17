//! The resume document: what gets typeset. It starts as a straight copy of the profile;
//! tailoring reorders, selects and rewords, and the fact gate checks the result against
//! the profile before export.

use serde::{Deserialize, Serialize};
use waypoint_core::Profile;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ResumeDoc {
    pub name: String,
    pub headline: String,
    /// Email, location, links — rendered on one line under the name.
    pub contact: Vec<String>,
    pub summary: String,
    pub experience: Vec<ResumeRole>,
    pub education: Vec<ResumeEducation>,
    pub skills: Vec<String>,
    pub languages: Vec<String>,
    /// "a4" or "us-letter".
    pub paper: String,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ResumeRole {
    pub company: String,
    pub title: String,
    pub location: String,
    pub start: String,
    pub end: String,
    pub bullets: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ResumeEducation {
    pub institution: String,
    pub degree: String,
    pub start: String,
    pub end: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Template {
    /// Quiet, single column, black and grey. Safest for applicant tracking systems.
    Clean,
    /// Single column with an accent color on the name and section rules.
    Modern,
}

impl Template {
    pub fn source(self) -> &'static str {
        match self {
            Template::Clean => include_str!("../templates/clean.typ"),
            Template::Modern => include_str!("../templates/modern.typ"),
        }
    }

    /// The cover letter in the same family, so resume and letter match.
    pub fn letter_source(self) -> &'static str {
        match self {
            Template::Clean => include_str!("../templates/letter_clean.typ"),
            Template::Modern => include_str!("../templates/letter_modern.typ"),
        }
    }
}

impl ResumeDoc {
    pub fn from_profile(p: &Profile) -> Self {
        let mut contact = Vec::new();
        for c in [&p.email, &p.location] {
            if !c.trim().is_empty() {
                contact.push(c.trim().to_string());
            }
        }
        contact.extend(p.links.iter().map(|l| l.trim().trim_start_matches("https://").trim_start_matches("http://").trim_end_matches('/').to_string()).filter(|l| !l.is_empty()));
        ResumeDoc {
            name: p.name.clone(),
            headline: p.headline.clone(),
            contact,
            summary: p.summary.clone(),
            experience: p
                .experience
                .iter()
                .map(|e| ResumeRole { company: e.company.clone(), title: e.title.clone(), location: e.location.clone(), start: e.start.clone(), end: e.end.clone(), bullets: e.highlights.clone() })
                .collect(),
            education: p.education.iter().map(|e| ResumeEducation { institution: e.institution.clone(), degree: e.degree.clone(), start: e.start.clone(), end: e.end.clone() }).collect(),
            skills: p.skills.clone(),
            languages: p.languages.iter().map(|l| if l.level.trim().is_empty() { l.name.clone() } else { format!("{} ({})", l.name, l.level) }).collect(),
            paper: "a4".into(),
        }
    }
}
