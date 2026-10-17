//! Resume documents: the model, Typst rendering, the fact gate and the ATS check.

pub mod ats;
pub mod factgate;
pub mod render;
pub mod resume;
pub mod tailor;

pub use resume::{ResumeDoc, ResumeEducation, ResumeRole, Template};

#[cfg(test)]
mod tests {
    use super::*;
    use waypoint_core::Profile;
    use waypoint_core::profile::{Education, Experience};

    fn sample() -> ResumeDoc {
        let p = Profile {
            name: "Ada Lovelace".into(),
            email: "ada@example.com".into(),
            headline: "Senior Backend Engineer".into(),
            location: "London".into(),
            summary: "Backend engineer focused on reliable, fast systems.".into(),
            skills: vec!["Rust".into(), "PostgreSQL".into(), "Kubernetes".into()],
            experience: vec![
                Experience { company: "Acme".into(), title: "Senior Engineer".into(), start: "2021".into(), end: "Present".into(), highlights: vec!["Cut p99 latency by 40%".into(), "Led a team of 5".into()], ..Default::default() },
                Experience { company: "Globex".into(), title: "Engineer".into(), start: "2018".into(), end: "2021".into(), highlights: vec!["Built the billing pipeline".into()], ..Default::default() },
            ],
            education: vec![Education { institution: "University of London".into(), degree: "BSc Mathematics".into(), start: "2014".into(), end: "2017".into() }],
            ..Default::default()
        };
        ResumeDoc::from_profile(&p)
    }

    #[test]
    fn both_templates_render_a_readable_pdf() {
        let doc = sample();
        for t in [Template::Clean, Template::Modern] {
            let compiled = render::compile(&doc, t).unwrap_or_else(|e| panic!("{t:?}: {e}"));
            let pages = compiled.pages().len();
            assert_eq!(pages, 1);
            let pdf = render::pdf(&compiled).unwrap();
            assert!(pdf.starts_with(b"%PDF"));
            assert_eq!(render::svg_pages(&compiled).len(), 1);
            let report = ats::check_pdf(&pdf, pages, &doc, &["Rust".into(), "Kafka".into()]);
            assert!(report.name_found, "{t:?} {report:?}");
            assert!(report.missing_sections.is_empty(), "{t:?} {report:?}");
            assert!(report.companies_in_order, "{t:?}");
            assert_eq!(report.coverage, 0.5);
        }
    }

    #[test]
    fn untouched_profile_passes_the_fact_gate() {
        let doc = sample();
        let p = Profile {
            name: doc.name.clone(),
            skills: doc.skills.clone(),
            experience: doc.experience.iter().map(|r| Experience { company: r.company.clone(), title: r.title.clone(), start: r.start.clone(), end: r.end.clone(), highlights: r.bullets.clone(), ..Default::default() }).collect(),
            summary: doc.summary.clone(),
            ..Default::default()
        };
        assert!(factgate::check(&doc, &p).is_empty());
    }
}
