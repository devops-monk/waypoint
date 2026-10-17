//! Render sample resumes: `cargo run -p waypoint-docs --example render -- <out-dir>`
use waypoint_core::Profile;
use waypoint_core::profile::{Education, Experience, Language};
use waypoint_docs::{ResumeDoc, Template, render};

fn main() {
    let out = std::env::args().nth(1).unwrap_or_else(|| ".".into());
    let p = Profile {
        name: "Priya Raman".into(),
        email: "priya@example.com".into(),
        headline: "Senior Backend Engineer".into(),
        location: "Bengaluru, India".into(),
        links: vec!["https://github.com/priya".into(), "https://linkedin.com/in/priya".into()],
        summary: "Backend engineer with eight years building realtime and payments systems. I like owning services end to end, from design docs to on-call.".into(),
        skills: ["Elixir", "Go", "PostgreSQL", "Kafka", "Kubernetes", "AWS", "gRPC", "Observability"].map(String::from).to_vec(),
        experience: vec![
            Experience { company: "Acme Chat".into(), title: "Senior Backend Engineer".into(), location: "Remote".into(), start: "Mar 2021".into(), end: "Present".into(), highlights: vec![
                "Built a Phoenix Channels service handling 2M concurrent connections with p99 delivery under 120 ms".into(),
                "Led the Postgres 12 to 15 migration across 40 databases with zero downtime".into(),
                "Mentored 4 engineers; two were promoted within a year".into()] },
            Experience { company: "PayFast".into(), title: "Backend Engineer".into(), location: "Bengaluru".into(), start: "Jul 2017".into(), end: "Feb 2021".into(), highlights: vec![
                "Designed the idempotent payouts API now processing ₹900 crore a month".into(),
                "Cut reconciliation time from 6 hours to 25 minutes by moving batch jobs to Kafka streams".into()] },
        ],
        education: vec![Education { institution: "IIT Madras".into(), degree: "B.Tech, Computer Science".into(), start: "2013".into(), end: "2017".into() }],
        languages: vec![Language { name: "English".into(), level: "Fluent".into() }, Language { name: "Tamil".into(), level: "Native".into() }, Language { name: "Hindi".into(), level: "Professional".into() }],
        ..Default::default()
    };
    let doc = ResumeDoc::from_profile(&p);
    for (t, name) in [(Template::Clean, "clean"), (Template::Modern, "modern")] {
        let d = render::compile(&doc, t).expect("compile");
        std::fs::write(format!("{out}/{name}.pdf"), render::pdf(&d).unwrap()).unwrap();
        std::fs::write(format!("{out}/{name}.svg"), &render::svg_pages(&d)[0]).unwrap();
        println!("{name}: {} page(s)", d.pages().len());
    }
}
