//! Application documents: resume and cover letter models, Typst rendering, the fact gate
//! and the ATS check.

pub mod ats;
pub mod factgate;
pub mod letter;
pub mod render;
pub mod resume;
pub mod tailor;
pub mod word;

pub use letter::LetterDoc;
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
    fn both_letter_templates_fit_one_page_and_read_back() {
        let resume = sample();
        let mut l = LetterDoc::with_header(&resume, "en");
        l.recipient = vec!["Hiring team".into(), "Globex".into(), "Copenhagen".into()];
        l.subject = "Backend engineer for Globex's payments platform".into();
        l.salutation = "Dear Globex hiring team,".into();
        l.opening = "Globex is rebuilding its payments platform in Rust, and that is the work I have done for the last four years. ".repeat(2);
        l.highlights = vec!["Cut p99 latency by 40% on the Acme API".into(), "Led a team of 5 through a database migration".into(), "Built the billing pipeline at Globex".into()];
        l.body = vec!["In the first months I would focus on the reliability of your settlement services. ".repeat(4), "I would welcome a conversation about the role.".into()];
        l.closing = "Kind regards,".into();
        for t in [Template::Clean, Template::Modern] {
            let compiled = render::compile_letter(&l, t).unwrap_or_else(|e| panic!("{t:?}: {e}"));
            assert_eq!(compiled.pages().len(), 1, "{t:?}");
            let text = pdf_extract::extract_text_from_mem(&render::pdf(&compiled).unwrap()).unwrap();
            for needle in ["Ada Lovelace", "Dear Globex hiring team", "Cut p99 latency", "Kind regards"] {
                assert!(text.contains(needle), "{t:?} missing {needle}");
            }
        }
    }

    #[test]
    fn word_exports_read_back_with_every_section() {
        let doc = sample();
        for t in [Template::Clean, Template::Modern] {
            let bytes = word::resume(&doc, t).unwrap();
            assert!(bytes.starts_with(b"PK"), "a .docx is a zip");
            let text = word::plain_text(&bytes).unwrap();
            for needle in ["Ada Lovelace", "Summary", "Experience", "Senior Engineer, Acme", "Cut p99 latency by 40%", "2021 – Present", "Skills", "Rust, PostgreSQL, Kubernetes", "Education", "BSc Mathematics"] {
                assert!(text.contains(needle), "{t:?} resume missing {needle:?}:\n{text}");
            }
            let mut l = LetterDoc::with_header(&doc, "en");
            l.recipient = vec!["Hiring team".into(), "Globex".into()];
            l.subject = "Backend engineer for Globex".into();
            l.salutation = "Dear Globex hiring team,".into();
            l.opening = "I build fast Rust services.".into();
            l.highlights = vec!["Cut p99 latency by 40%".into()];
            l.body = vec!["I would welcome a conversation.".into()];
            l.closing = "Kind regards,".into();
            let text = word::plain_text(&word::letter(&l, t).unwrap()).unwrap();
            for needle in ["Hiring team", "Globex", "Dear Globex hiring team,", "Cut p99 latency by 40%", "Kind regards,", "Ada Lovelace"] {
                assert!(text.contains(needle), "{t:?} letter missing {needle:?}:\n{text}");
            }
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
