//! Applying an AI tailoring plan. The plan only names profile items by index and offers
//! reworded text; companies, titles, dates and education are always copied from the
//! profile here, so the model has no way to change them.

use serde::{Deserialize, Serialize};
use waypoint_core::Profile;

use crate::factgate::{self, Issue};
use crate::resume::{ResumeDoc, ResumeRole};

/// What the model returns (see `waypoint_ai::prompts::tailor_schema`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TailorPlan {
    pub summary: String,
    pub roles: Vec<PlanRole>,
    pub skills: Vec<String>,
    pub rationale: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanRole {
    pub role_index: usize,
    pub bullets: Vec<PlanBullet>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanBullet {
    pub source_index: usize,
    pub text: String,
}

/// One bullet as the review screen shows it: the original, the suggestion, and any
/// fact-gate findings on the suggestion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BulletChange {
    pub source_index: usize,
    pub original: String,
    pub proposed: String,
    pub issues: Vec<Issue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoleChange {
    pub role_index: usize,
    pub company: String,
    pub title: String,
    pub bullets: Vec<BulletChange>,
    /// Profile bullets the plan left out, so the user can put them back.
    pub omitted: Vec<(usize, String)>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Proposal {
    pub summary_original: String,
    pub summary_proposed: String,
    pub summary_issues: Vec<Issue>,
    pub roles: Vec<RoleChange>,
    pub skills_proposed: Vec<String>,
    pub skill_issues: Vec<Issue>,
    pub rationale: String,
    /// The resume with every suggestion accepted (blocked suggestions fall back to the original).
    pub doc: ResumeDoc,
}

/// Build the review proposal. Out-of-range indices from the model are ignored, every
/// profile role is kept in profile order, and a role the plan skipped keeps its bullets.
pub fn propose(profile: &Profile, plan: &TailorPlan, base: &ResumeDoc) -> Proposal {
    let mut roles = Vec::new();
    let mut doc_roles = Vec::new();
    for (ri, exp) in profile.experience.iter().enumerate() {
        let planned = plan.roles.iter().find(|r| r.role_index == ri);
        let mut bullets = Vec::new();
        let mut used = Vec::new();
        if let Some(pr) = planned {
            for b in &pr.bullets {
                let Some(original) = exp.highlights.get(b.source_index) else { continue };
                if used.contains(&b.source_index) || b.text.trim().is_empty() {
                    continue;
                }
                used.push(b.source_index);
                bullets.push(BulletChange { source_index: b.source_index, original: original.clone(), proposed: b.text.trim().to_string(), issues: vec![] });
            }
        }
        if bullets.is_empty() {
            // Nothing usable for this role: keep it exactly as written.
            bullets = exp.highlights.iter().enumerate().map(|(i, h)| BulletChange { source_index: i, original: h.clone(), proposed: h.clone(), issues: vec![] }).collect();
            used = (0..exp.highlights.len()).collect();
        }
        let omitted = exp.highlights.iter().enumerate().filter(|(i, _)| !used.contains(i)).map(|(i, h)| (i, h.clone())).collect();
        doc_roles.push(ResumeRole {
            company: exp.company.clone(),
            title: exp.title.clone(),
            location: exp.location.clone(),
            start: exp.start.clone(),
            end: exp.end.clone(),
            bullets: bullets.iter().map(|b| b.proposed.clone()).collect(),
        });
        roles.push(RoleChange { role_index: ri, company: exp.company.clone(), title: exp.title.clone(), bullets, omitted });
    }

    let mut doc = ResumeDoc {
        summary: if plan.summary.trim().is_empty() { base.summary.clone() } else { plan.summary.trim().to_string() },
        experience: doc_roles,
        skills: if plan.skills.is_empty() { base.skills.clone() } else { plan.skills.clone() },
        ..base.clone()
    };

    // Attach fact-gate findings to the suggestion they belong to.
    let issues = factgate::check(&doc, profile);
    let mut summary_issues = Vec::new();
    let mut skill_issues = Vec::new();
    for issue in issues {
        let parts: Vec<&str> = issue.path.split('.').collect();
        match parts.as_slice() {
            ["summary"] => summary_issues.push(issue),
            ["skills", _] => skill_issues.push(issue),
            ["experience", r, "bullets", b] => {
                if let (Ok(r), Ok(b)) = (r.parse::<usize>(), b.parse::<usize>())
                    && let Some(bc) = roles.get_mut(r).and_then(|role| role.bullets.get_mut(b)) {
                        bc.issues.push(issue);
                    }
            }
            _ => {}
        }
    }

    // Blocked suggestions fall back to the original wording in the default doc.
    for (r, role) in roles.iter().enumerate() {
        for (b, bc) in role.bullets.iter().enumerate() {
            if bc.issues.iter().any(|i| i.severity == factgate::Severity::Block) {
                doc.experience[r].bullets[b] = bc.original.clone();
            }
        }
    }
    let summary_blocked = summary_issues.iter().any(|i| i.severity == factgate::Severity::Block);
    if summary_blocked {
        doc.summary = base.summary.clone();
    }
    let blocked_skills: Vec<String> = skill_issues.iter().filter(|i| i.severity == factgate::Severity::Block).filter_map(|i| i.path.split('.').nth(1)?.parse::<usize>().ok()).filter_map(|i| plan.skills.get(i).cloned()).collect();
    doc.skills.retain(|s| !blocked_skills.contains(s));

    Proposal {
        summary_original: base.summary.clone(),
        summary_proposed: plan.summary.trim().to_string(),
        summary_issues,
        roles,
        skills_proposed: plan.skills.clone(),
        skill_issues,
        rationale: plan.rationale.clone(),
        doc,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::factgate::Severity;
    use waypoint_core::profile::Experience;

    fn profile() -> Profile {
        Profile {
            name: "Ada".into(),
            summary: "Backend engineer.".into(),
            skills: vec!["Rust".into(), "PostgreSQL".into(), "Go".into()],
            experience: vec![
                Experience { company: "Acme".into(), title: "Senior Engineer".into(), start: "2021".into(), end: "Present".into(), highlights: vec!["Cut p99 latency by 40% in Rust".into(), "Ran the hiring loop".into(), "Wrote the on-call guide".into()], ..Default::default() },
                Experience { company: "Globex".into(), title: "Engineer".into(), start: "2018".into(), end: "2021".into(), highlights: vec!["Built billing in Go".into()], ..Default::default() },
            ],
            ..Default::default()
        }
    }

    #[test]
    fn plan_reorders_rewords_and_omits_but_never_changes_role_facts() {
        let p = profile();
        let base = ResumeDoc::from_profile(&p);
        let plan = TailorPlan {
            summary: "Backend engineer who makes Rust services fast.".into(),
            roles: vec![PlanRole { role_index: 0, bullets: vec![PlanBullet { source_index: 2, text: "Wrote the team's on-call guide".into() }, PlanBullet { source_index: 0, text: "Reduced p99 latency 40% in a Rust service".into() }, PlanBullet { source_index: 9, text: "out of range".into() }] }],
            skills: vec!["Rust".into(), "PostgreSQL".into()],
            rationale: "".into(),
        };
        let prop = propose(&p, &plan, &base);
        assert_eq!(prop.doc.experience[0].bullets, vec!["Wrote the team's on-call guide", "Reduced p99 latency 40% in a Rust service"]);
        assert_eq!(prop.roles[0].omitted, vec![(1, "Ran the hiring loop".to_string())]);
        // The role the plan skipped keeps its bullets; facts come from the profile.
        assert_eq!(prop.doc.experience[1].bullets, vec!["Built billing in Go"]);
        assert_eq!(prop.doc.experience[0].title, "Senior Engineer");
        assert_eq!(prop.doc.skills, vec!["Rust", "PostgreSQL"]);
    }

    #[test]
    fn fabricated_suggestions_are_flagged_and_fall_back() {
        let p = profile();
        let base = ResumeDoc::from_profile(&p);
        let plan = TailorPlan {
            summary: "".into(),
            roles: vec![PlanRole { role_index: 0, bullets: vec![PlanBullet { source_index: 0, text: "Cut p99 latency by 75% in Rust".into() }] }],
            skills: vec!["Rust".into(), "Terraform".into()],
            rationale: "".into(),
        };
        let prop = propose(&p, &plan, &base);
        assert!(prop.roles[0].bullets[0].issues.iter().any(|i| i.severity == Severity::Block));
        assert_eq!(prop.doc.experience[0].bullets[0], "Cut p99 latency by 40% in Rust");
        assert_eq!(prop.doc.skills, vec!["Rust"]);
        assert_eq!(prop.doc.summary, "Backend engineer.");
    }
}
