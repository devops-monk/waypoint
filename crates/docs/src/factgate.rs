//! The fact gate: a tailored resume may reorder, select and reword, but every role, date,
//! number and skill must trace back to the profile. Same rule as career-ops
//! `verify-cv-facts.mjs` and ai-job-search's honesty rules: "reformulate, never fabricate".

use std::collections::HashSet;
use std::sync::LazyLock;

use regex::Regex;
use serde::{Deserialize, Serialize};
use waypoint_core::Profile;

use crate::resume::ResumeDoc;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    /// Must be fixed before export: the claim isn't in the profile.
    Block,
    /// Worth a look; may be a fair rewording.
    Warn,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Issue {
    pub severity: Severity,
    /// Dotted path into the resume, e.g. `experience.0.bullets.2`.
    pub path: String,
    pub message: String,
}

/// Numbers with their unit, e.g. "40%", "2M", "$3.5M", "10x", "150+".
static NUMBER: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)[$€£₹]?\d[\d,.]*\s?(?:%|k|m|mm|bn|b|x|\+|million|billion)?").unwrap());
/// Capitalized or mixed-case technical terms: "Kubernetes", "GraphQL", "AWS", "Node.js".
static TERM: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\b(?:[A-Z][a-z]+[A-Z]\w*|[A-Z]{2,}[a-z]*|[A-Z][a-z]+(?:\.js|\.NET)|[A-Z][a-z]{2,})\b").unwrap());

fn norm(s: &str) -> String {
    s.trim().to_lowercase().split_whitespace().collect::<Vec<_>>().join(" ")
}

fn norm_number(raw: &str) -> String {
    let s = raw.to_lowercase().replace([',', ' ', '$', '€', '£', '₹'], "");
    let s = s.trim_end_matches('.').to_string();
    s.replace("million", "m").replace("billion", "b").replace("mm", "m").replace("bn", "b")
}

fn numbers(text: &str) -> Vec<String> {
    NUMBER
        .find_iter(text)
        .map(|m| m.as_str())
        // Ignore bare single digits ("3 teams" vs "three teams" is a fair rewording) and years.
        .filter(|n| {
            let digits: String = n.chars().filter(char::is_ascii_digit).collect();
            digits.len() > 1 && !(digits.len() == 4 && (digits.starts_with("19") || digits.starts_with("20")) && n.chars().all(|c| c.is_ascii_digit()))
        })
        .map(norm_number)
        .collect()
}

/// Words that open sentences or are ordinary English, never treated as claims.
const COMMON: &[&str] = &["Led", "Built", "Designed", "Developed", "Managed", "Created", "Improved", "Reduced", "Increased", "Launched", "Owned", "Drove", "Delivered", "Implemented", "Migrated", "Shipped", "Worked", "Partnered", "Mentored", "Scaled", "Automated", "Wrote", "Ran", "The", "This", "With", "Across", "Over", "From", "And", "For", "Senior", "Lead", "Staff", "Principal", "Engineer", "Engineering", "Team", "Teams", "Product", "Customer", "Customers", "Data"];

/// All profile text, lowercased, as the evidence corpus.
fn corpus(p: &Profile) -> String {
    let mut parts = vec![p.name.clone(), p.headline.clone(), p.summary.clone(), p.location.clone()];
    parts.extend(p.skills.iter().cloned());
    for e in &p.experience {
        parts.extend([e.company.clone(), e.title.clone(), e.location.clone()]);
        parts.extend(e.highlights.iter().cloned());
    }
    for e in &p.education {
        parts.extend([e.institution.clone(), e.degree.clone()]);
    }
    parts.join("\n").to_lowercase()
}

pub fn check(doc: &ResumeDoc, profile: &Profile) -> Vec<Issue> {
    let mut issues = Vec::new();
    let text = corpus(profile);
    let known_numbers: HashSet<String> = numbers(&text).into_iter().collect();

    // Roles must match a profile role exactly on company, title and dates.
    for (i, r) in doc.experience.iter().enumerate() {
        let matched = profile.experience.iter().find(|e| norm(&e.company) == norm(&r.company));
        match matched {
            None => issues.push(Issue { severity: Severity::Block, path: format!("experience.{i}"), message: format!("“{}” isn't a company in your profile.", r.company) }),
            Some(e) => {
                if norm(&e.title) != norm(&r.title) {
                    issues.push(Issue { severity: Severity::Block, path: format!("experience.{i}.title"), message: format!("Your title at {} is “{}” in your profile, not “{}”.", e.company, e.title, r.title) });
                }
                if norm(&e.start) != norm(&r.start) || norm(&e.end) != norm(&r.end) {
                    issues.push(Issue { severity: Severity::Block, path: format!("experience.{i}.dates"), message: format!("Dates at {} differ from your profile ({} – {}).", e.company, e.start, e.end) });
                }
            }
        }
    }

    let check_text = |path: String, t: &str, issues: &mut Vec<Issue>| {
        for n in numbers(t) {
            if !known_numbers.contains(&n) {
                issues.push(Issue { severity: Severity::Block, path: path.clone(), message: format!("“{n}” doesn't appear anywhere in your profile.") });
            }
        }
        let mut seen = HashSet::new();
        for m in TERM.find_iter(t) {
            let term = m.as_str();
            if COMMON.contains(&term) || !seen.insert(term.to_string()) {
                continue;
            }
            if !text.contains(&term.to_lowercase()) {
                issues.push(Issue { severity: Severity::Warn, path: path.clone(), message: format!("“{term}” isn't mentioned in your profile. Keep it only if it's true.") });
            }
        }
    };

    check_text("summary".into(), &doc.summary, &mut issues);
    for (i, r) in doc.experience.iter().enumerate() {
        for (j, b) in r.bullets.iter().enumerate() {
            check_text(format!("experience.{i}.bullets.{j}"), b, &mut issues);
        }
    }

    let skills: HashSet<String> = profile.skills.iter().map(|s| norm(s)).collect();
    for (i, s) in doc.skills.iter().enumerate() {
        if !skills.contains(&norm(s)) && !text.contains(&norm(s)) {
            issues.push(Issue { severity: Severity::Block, path: format!("skills.{i}"), message: format!("“{s}” isn't one of your skills.") });
        }
    }
    issues
}

#[cfg(test)]
mod tests {
    use super::*;
    use waypoint_core::profile::Experience;

    fn profile() -> Profile {
        Profile {
            name: "Ada".into(),
            skills: vec!["Rust".into(), "PostgreSQL".into()],
            experience: vec![Experience {
                company: "Acme".into(),
                title: "Senior Engineer".into(),
                start: "2021".into(),
                end: "Present".into(),
                highlights: vec!["Cut p99 latency by 40% on a Rust service handling 2M requests/day".into()],
                ..Default::default()
            }],
            ..Default::default()
        }
    }

    #[test]
    fn faithful_rewording_passes() {
        let mut doc = ResumeDoc::from_profile(&profile());
        doc.experience[0].bullets = vec!["Reduced p99 latency 40% for a Rust service serving 2M requests a day".into()];
        assert!(check(&doc, &profile()).iter().all(|i| i.severity != Severity::Block), "{:?}", check(&doc, &profile()));
    }

    #[test]
    fn invented_numbers_titles_and_skills_are_blocked() {
        let mut doc = ResumeDoc::from_profile(&profile());
        doc.experience[0].title = "Staff Engineer".into();
        doc.experience[0].bullets = vec!["Cut latency by 65% and saved $1.2M".into()];
        doc.skills.push("Kubernetes".into());
        let issues = check(&doc, &profile());
        let blocks: Vec<_> = issues.iter().filter(|i| i.severity == Severity::Block).map(|i| i.path.as_str()).collect();
        assert!(blocks.contains(&"experience.0.title"));
        assert_eq!(blocks.iter().filter(|p| **p == "experience.0.bullets.0").count(), 2, "65% and $1.2M");
        assert!(blocks.contains(&"skills.2"));
    }

    #[test]
    fn unknown_terms_warn_not_block() {
        let mut doc = ResumeDoc::from_profile(&profile());
        doc.experience[0].bullets = vec!["Built Rust services on Kubernetes".into()];
        let issues = check(&doc, &profile());
        assert!(issues.iter().any(|i| i.severity == Severity::Warn && i.message.contains("Kubernetes")));
        assert!(issues.iter().all(|i| i.severity != Severity::Block));
    }
}
