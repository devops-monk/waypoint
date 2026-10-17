//! Cover letters: the document model and the checks run before export. The letter shares
//! the resume's header and template family so the two read as a matching set.
//!
//! Checks follow ai-job-search `03-writing-style.md` and `06-cover-letter-templates.md`:
//! every number traces to the profile or the posting, claims about yourself use only your
//! own skills, no em-dashes, no stock phrases, and it fits on one page (250-350 words).

use std::collections::HashSet;

use chrono::{Datelike, NaiveDate};
use serde::{Deserialize, Serialize};
use waypoint_core::Profile;

use crate::factgate::{COMMON, Issue, Severity, TERM, corpus, numbers};
use crate::resume::ResumeDoc;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct LetterDoc {
    pub name: String,
    /// Same contact line as the resume.
    pub contact: Vec<String>,
    pub date: String,
    /// Address block, one line each: "Hiring team", "Acme", "Copenhagen".
    pub recipient: Vec<String>,
    /// The headline: specific, not "Application for X".
    pub subject: String,
    pub salutation: String,
    pub opening: String,
    /// Three to five concrete achievements, shown as a list after the opening.
    pub highlights: Vec<String>,
    pub body: Vec<String>,
    pub closing: String,
    pub signature: String,
    /// "a4" or "us-letter".
    pub paper: String,
    /// ISO 639-1 language of the letter, for hyphenation and date format.
    pub lang: String,
}

impl LetterDoc {
    /// The words a reader reads: everything between the salutation and the sign-off.
    pub fn body_words(&self) -> usize {
        let mut n = self.opening.split_whitespace().count();
        n += self.highlights.iter().chain(&self.body).map(|t| t.split_whitespace().count()).sum::<usize>();
        n
    }

    /// Contact details, paper size and name come from the resume so both documents match.
    pub fn with_header(resume: &ResumeDoc, lang: &str) -> Self {
        let lang = clean_lang(lang);
        LetterDoc {
            name: resume.name.clone(),
            contact: resume.contact.clone(),
            date: format_date(chrono::Local::now().date_naive(), &lang, &resume.paper),
            signature: resume.name.clone(),
            paper: if resume.paper.is_empty() { "a4".into() } else { resume.paper.clone() },
            lang,
            ..Default::default()
        }
    }
}

/// A two or three letter lowercase code, else English.
pub fn clean_lang(lang: &str) -> String {
    let l = lang.trim().to_lowercase();
    let l = l.split(['-', '_']).next().unwrap_or("");
    if (2..=3).contains(&l.len()) && l.chars().all(|c| c.is_ascii_lowercase()) { l.to_string() } else { "en".into() }
}

/// The date the way a letter in that language writes it. US paper means US English order.
pub fn format_date(d: NaiveDate, lang: &str, paper: &str) -> String {
    const EN: [&str; 12] = ["January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December"];
    const DA: [&str; 12] = ["januar", "februar", "marts", "april", "maj", "juni", "juli", "august", "september", "oktober", "november", "december"];
    const NB: [&str; 12] = ["januar", "februar", "mars", "april", "mai", "juni", "juli", "august", "september", "oktober", "november", "desember"];
    const SV: [&str; 12] = ["januari", "februari", "mars", "april", "maj", "juni", "juli", "augusti", "september", "oktober", "november", "december"];
    const DE: [&str; 12] = ["Januar", "Februar", "März", "April", "Mai", "Juni", "Juli", "August", "September", "Oktober", "November", "Dezember"];
    const NL: [&str; 12] = ["januari", "februari", "maart", "april", "mei", "juni", "juli", "augustus", "september", "oktober", "november", "december"];
    const FR: [&str; 12] = ["janvier", "février", "mars", "avril", "mai", "juin", "juillet", "août", "septembre", "octobre", "novembre", "décembre"];
    const ES: [&str; 12] = ["enero", "febrero", "marzo", "abril", "mayo", "junio", "julio", "agosto", "septiembre", "octubre", "noviembre", "diciembre"];
    let (day, m, y) = (d.day(), d.month0() as usize, d.year());
    match lang {
        "da" => format!("{day}. {} {y}", DA[m]),
        "nb" | "no" | "nn" => format!("{day}. {} {y}", NB[m]),
        "de" => format!("{day}. {} {y}", DE[m]),
        "sv" => format!("{day} {} {y}", SV[m]),
        "nl" => format!("{day} {} {y}", NL[m]),
        "fr" => format!("{day} {} {y}", FR[m]),
        "es" => format!("{day} de {} de {y}", ES[m]),
        _ if paper == "us-letter" => format!("{} {day}, {y}", EN[m]),
        _ => format!("{day} {} {y}", EN[m]),
    }
}

/// Phrases recruiters skim past. From the ai-job-search writing rules plus common offenders.
const STOCK_PHRASES: &[&str] = &[
    "i am passionate about",
    "i'm passionate about",
    "great fit",
    "perfect fit",
    "leverage my",
    "hit the ground running",
    "drive results",
    "synergy",
    "synergies",
    "team player",
    "think outside the box",
    "go-getter",
    "self-starter",
    "results-driven",
    "detail-oriented",
    "dynamic environment",
    "fast-paced environment",
    "to whom it may concern",
    "i believe i would",
    "i think i could",
    "proven track record",
    "excited to apply",
];

/// Words that open or close a letter, never claims.
const LETTER_WORDS: &[&str] = &["Dear", "Kind", "Best", "Regards", "Sincerely", "Thank", "Thanks", "Hiring", "Manager", "Hello", "Your", "Our", "You", "Since", "When", "While", "What", "That", "These", "Those", "Having", "Today", "Here", "Most", "Recently", "Before", "After", "Because", "Both", "Each", "Every"];

fn first_person(sentence: &str) -> bool {
    sentence.split(|c: char| !c.is_alphanumeric() && c != '\'').any(|w| matches!(w, "I" | "I'm" | "I've" | "I'd" | "I'll" | "my" | "My" | "me" | "Me"))
}

fn sentences(text: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut start = 0;
    let b = text.as_bytes();
    for (i, c) in text.char_indices() {
        if matches!(c, '.' | '!' | '?') && b.get(i + 1).is_none_or(|n| n.is_ascii_whitespace()) {
            out.push(&text[start..=i]);
            start = i + 1;
        }
    }
    if start < text.len() {
        out.push(&text[start..]);
    }
    out
}

/// Check a letter against the profile and the posting it answers.
///
/// - A number must appear in the profile or the posting ("your 40-person team" is fine).
/// - In a sentence about yourself, a technical term found only in the posting is flagged:
///   describing their stack is fine, claiming it as yours needs to be true.
/// - Style: em-dashes, stock phrases, and length against the one-page budget.
pub fn check(letter: &LetterDoc, profile: &Profile, posting: &str, company: &str) -> Vec<Issue> {
    let mine = corpus(profile);
    let theirs = posting.to_lowercase();
    let known: HashSet<String> = numbers(&mine).into_iter().chain(numbers(&theirs)).collect();
    let company_words: HashSet<String> = company.split_whitespace().map(str::to_lowercase).collect();
    let english = letter.lang == "en" || letter.lang.is_empty();
    let mut issues = Vec::new();

    let mut check_text = |path: String, text: &str, always_about_me: bool| {
        for n in numbers(text) {
            if !known.contains(&n) {
                issues.push(Issue { severity: Severity::Block, path: path.clone(), message: format!("“{n}” isn't in your profile or the posting.") });
            }
        }
        // Capitalization tells us little in German and similar languages, so term checks
        // only run on English letters. Numbers are checked in every language.
        if english {
            let mut seen = HashSet::new();
            for s in sentences(text) {
                for m in TERM.find_iter(s) {
                    // Judge the clause, not the sentence: "your team runs Kafka, and I build Rust".
                    let from = s[..m.start()].rfind([',', ';', ':', '—']).map_or(0, |i| i + s[i..].chars().next().map_or(1, char::len_utf8));
                    let to = s[m.end()..].find([',', ';', ':', '—']).map_or(s.len(), |i| m.end() + i);
                    let about_me = always_about_me || first_person(&s[from..to]);
                    let term = m.as_str();
                    let lower = term.to_lowercase();
                    let sentence_start = s[..m.start()].trim().is_empty();
                    if sentence_start || COMMON.contains(&term) || LETTER_WORDS.contains(&term) || company_words.contains(&lower) || !seen.insert(lower.clone()) {
                        continue;
                    }
                    if mine.contains(&lower) {
                        continue;
                    }
                    if theirs.contains(&lower) {
                        if about_me {
                            issues.push(Issue { severity: Severity::Warn, path: path.clone(), message: format!("“{term}” comes from the posting, not your profile. Keep it only if you've really used it.") });
                        }
                    } else {
                        issues.push(Issue { severity: Severity::Warn, path: path.clone(), message: format!("“{term}” isn't in your profile or the posting. Keep it only if it's true.") });
                    }
                }
            }
        }
        if text.contains('—') || text.contains(" – ") || text.contains(" -- ") {
            issues.push(Issue { severity: Severity::Warn, path: path.clone(), message: "Swap the dash for a comma or a full stop. Dashes read as machine-written.".into() });
        }
        let lower = text.to_lowercase();
        for p in STOCK_PHRASES {
            if lower.contains(p) {
                issues.push(Issue { severity: Severity::Warn, path: path.clone(), message: format!("“{p}” is a stock phrase. Show it with an example instead.") });
            }
        }
    };

    check_text("subject".into(), &letter.subject, false);
    check_text("salutation".into(), &letter.salutation, false);
    check_text("opening".into(), &letter.opening, false);
    for (i, h) in letter.highlights.iter().enumerate() {
        check_text(format!("highlights.{i}"), h, true);
    }
    for (i, b) in letter.body.iter().enumerate() {
        check_text(format!("body.{i}"), b, false);
    }

    let words = letter.body_words();
    if words > 380 {
        issues.push(Issue { severity: Severity::Warn, path: "length".into(), message: format!("{words} words. Aim for 250 to 350 so it stays on one page and gets read.") });
    } else if words > 0 && words < 150 {
        issues.push(Issue { severity: Severity::Warn, path: "length".into(), message: format!("{words} words. A little more detail on what you'd do in this role would help.") });
    }
    issues
}

#[cfg(test)]
mod tests {
    use super::*;
    use waypoint_core::profile::Experience;

    fn profile() -> Profile {
        Profile {
            name: "Ada Lovelace".into(),
            skills: vec!["Rust".into(), "PostgreSQL".into()],
            experience: vec![Experience { company: "Acme".into(), title: "Senior Engineer".into(), highlights: vec!["Cut p99 latency by 40% on a Rust service handling 2M requests/day".into()], ..Default::default() }],
            ..Default::default()
        }
    }

    const POSTING: &str = "Globex is hiring a backend engineer to join a 12-person platform team. You will work with Rust, Kafka and Kubernetes.";

    fn letter() -> LetterDoc {
        LetterDoc {
            lang: "en".into(),
            subject: "Backend engineer for Globex's platform team".into(),
            salutation: "Dear Globex hiring team,".into(),
            opening: "Your platform team of 12 is building the Kafka pipeline Globex runs on, and I build fast Rust services.".into(),
            highlights: vec!["Cut p99 latency by 40% on a Rust service handling 2M requests a day".into()],
            body: vec!["I would bring the same focus on latency to your services.".into()],
            ..Default::default()
        }
    }

    #[test]
    fn grounded_letter_has_no_blocks_and_describing_their_stack_is_fine() {
        let issues = check(&letter(), &profile(), POSTING, "Globex");
        assert!(issues.iter().all(|i| i.severity != Severity::Block), "{issues:?}");
        assert!(!issues.iter().any(|i| i.message.contains("Kafka")), "Kafka described as theirs, not claimed: {issues:?}");
    }

    #[test]
    fn invented_numbers_block_and_borrowed_skills_warn() {
        let mut l = letter();
        l.highlights.push("Ran Kafka clusters at 15x the scale".into());
        l.body.push("I have deep experience with Kubernetes — and Terraform.".into());
        let issues = check(&l, &profile(), POSTING, "Globex");
        assert!(issues.iter().any(|i| i.severity == Severity::Block && i.message.contains("15x")));
        assert!(issues.iter().any(|i| i.path == "highlights.1" && i.message.contains("Kafka")));
        assert!(issues.iter().any(|i| i.path == "body.1" && i.message.contains("Kubernetes")));
        assert!(issues.iter().any(|i| i.path == "body.1" && i.message.contains("Terraform") && i.message.contains("or the posting")));
        assert!(issues.iter().any(|i| i.path == "body.1" && i.message.contains("dash")));
    }

    #[test]
    fn stock_phrases_and_length_are_flagged() {
        let mut l = letter();
        l.body = vec!["I am passionate about latency and would be a great fit.".into(); 40];
        let issues = check(&l, &profile(), POSTING, "Globex");
        assert!(issues.iter().any(|i| i.message.contains("i am passionate about")));
        assert!(issues.iter().any(|i| i.path == "length"));
    }

    #[test]
    fn dates_follow_the_letter_language() {
        let d = NaiveDate::from_ymd_opt(2026, 10, 17).unwrap();
        assert_eq!(format_date(d, "en", "a4"), "17 October 2026");
        assert_eq!(format_date(d, "en", "us-letter"), "October 17, 2026");
        assert_eq!(format_date(d, "da", "a4"), "17. oktober 2026");
        assert_eq!(format_date(d, "de", "a4"), "17. Oktober 2026");
        assert_eq!(clean_lang("en-GB"), "en");
        assert_eq!(clean_lang("Danish!"), "en");
    }
}
