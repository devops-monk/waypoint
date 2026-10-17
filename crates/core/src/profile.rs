//! The candidate profile: the only source of facts for evaluation and tailoring.
//! Anything generated for the user must trace back to fields here.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Profile {
    pub name: String,
    pub email: String,
    pub headline: String,
    pub location: String,
    pub summary: String,
    pub links: Vec<String>,
    pub experience: Vec<Experience>,
    pub education: Vec<Education>,
    pub skills: Vec<String>,
    pub languages: Vec<Language>,
    pub preferences: Preferences,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Experience {
    pub company: String,
    pub title: String,
    pub location: String,
    pub start: String,
    pub end: String,
    pub highlights: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Education {
    pub institution: String,
    pub degree: String,
    pub start: String,
    pub end: String,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Language {
    pub name: String,
    /// Free text as the user states it: "Native", "C1", "Professional working".
    pub level: String,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Preferences {
    pub target_roles: Vec<String>,
    pub locations: Vec<String>,
    pub remote_ok: bool,
    pub relocation_ok: bool,
    pub salary_floor: Option<f64>,
    pub salary_currency: String,
    /// Citizenship / work-permit context used by the eligibility gate.
    pub work_authorization: String,
    pub needs_sponsorship: bool,
    pub career_goals: Vec<String>,
    pub energizing: Vec<String>,
    pub draining: Vec<String>,
    pub deal_breakers: Vec<String>,
}

impl Profile {
    pub fn is_empty(&self) -> bool {
        self.experience.is_empty() && self.skills.is_empty() && self.summary.trim().is_empty()
    }
}
