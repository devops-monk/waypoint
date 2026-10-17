//! Render sample resumes and cover letters: `cargo run -p waypoint-docs --example render -- <out-dir>`
use waypoint_core::Profile;
use waypoint_core::profile::{Education, Experience, Language};
use waypoint_docs::{LetterDoc, ResumeDoc, Template, render};

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

    let mut letter = LetterDoc::with_header(&doc, "en");
    letter.recipient = vec!["Hiring team".into(), "Supabase".into(), "Remote".into()];
    letter.subject = "Senior backend engineer with eight years of realtime systems in Elixir".into();
    letter.salutation = "Dear Supabase hiring team,".into();
    letter.opening = "Supabase Realtime is an Elixir service that has to fan out database changes to a very large number of clients, and that is the problem I have spent the last four years on at Acme Chat. I'm applying for the Senior Backend Engineer role on the Realtime team.".into();
    letter.highlights = vec![
        "Built a Phoenix Channels service handling 2M concurrent connections with p99 delivery under 120 ms".into(),
        "Led the Postgres 12 to 15 migration across 40 databases with zero downtime".into(),
        "Cut reconciliation time from 6 hours to 25 minutes by moving batch jobs to Kafka streams".into(),
    ];
    letter.body = vec![
        "In the first months I would focus on the parts of Realtime that page people: connection churn, backpressure when a large table changes at once, and the metrics that tell you which of the two you're looking at. At Acme Chat I owned exactly that, from design docs to on-call.".into(),
        "Your posting asks for open-source experience. Most of my work has been in private repositories, so I'd be learning the public review rhythm, but I've mentored four engineers through code review and enjoy doing it in the open.".into(),
        "I would welcome a conversation about the role.".into(),
    ];
    letter.closing = "Kind regards,".into();
    for (t, name) in [(Template::Clean, "letter_clean"), (Template::Modern, "letter_modern")] {
        let d = render::compile_letter(&letter, t).expect("compile letter");
        std::fs::write(format!("{out}/{name}.pdf"), render::pdf(&d).unwrap()).unwrap();
        std::fs::write(format!("{out}/{name}.svg"), &render::svg_pages(&d)[0]).unwrap();
        println!("{name}: {} page(s), {} words", d.pages().len(), letter.body_words());
    }
}
