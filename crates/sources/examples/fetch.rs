//! Live smoke test: `cargo run -p waypoint-sources --example fetch -- <provider> <slug>`
use std::time::Duration;

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (provider, slug) = (args.first().map(String::as_str).unwrap_or("greenhouse"), args.get(1).map(String::as_str).unwrap_or("anthropic"));
    let http = waypoint_sources::Http::new(Duration::from_millis(500));
    match waypoint_sources::fetch(&http, provider, slug, slug).await {
        Ok(jobs) => {
            let with_desc = jobs.iter().filter(|j| !j.description.is_empty()).count();
            let with_salary = jobs.iter().filter(|j| j.salary.is_some()).count();
            println!("{provider}/{slug}: {} jobs, {with_desc} with description, {with_salary} with salary", jobs.len());
            if let Some(j) = jobs.first() {
                println!("  e.g. {} | {} | remote={} | {}", j.title, j.location, j.remote, j.url);
            }
        }
        Err(e) => println!("{provider}/{slug}: ERROR {e}"),
    }
}
