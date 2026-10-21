//! People you know: LinkedIn's own "Connections.csv" export, matched against the companies
//! you're looking at. Port of career-ops `linkedin-join.mjs` (company token matching and
//! the export's quirks). Connections are other people's data: they stay in the local
//! database and can be removed in one go.

use std::collections::HashSet;

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Contact {
    pub id: i64,
    pub first_name: String,
    pub last_name: String,
    pub company: String,
    pub position: String,
    pub email: String,
    pub url: String,
    pub connected_on: Option<NaiveDate>,
}

impl Contact {
    pub fn name(&self) -> String {
        format!("{} {}", self.first_name, self.last_name).trim().to_string()
    }
}

// ── Company matching ────────────────────────────────────────────────────

/// Words that say nothing about which company it is.
const GENERIC: &[&str] = &[
    "the", "a", "an", "and", "of", "for", "inc", "incorporated", "llc", "llp", "ltd", "limited", "corp", "corporation", "co", "company", "gmbh", "mbh", "ag", "kg", "sa", "sas", "sarl", "bv", "nv", "ab", "oy", "as", "aps", "plc", "pty", "srl", "spa", "kk", "pte", "pvt",
    "holdings", "holding", "group", "groupe", "international", "global", "worldwide", "enterprises", "enterprise", "ventures", "capital", "partners", "associates", "consulting", "consultancy", "industries", "industrial", "technologies", "technology", "tech", "software",
    "systems", "system", "solutions", "solution", "services", "service", "labs", "lab", "digital", "data", "cloud", "ai", "health", "healthcare", "medical", "media", "network", "networks", "platform", "platforms", "security", "cybersecurity", "cyber", "financial", "finance",
    "insurance", "energy", "studio", "studios", "stealth", "startup", "unknown", "confidential", "undisclosed", "anon", "anonymous", "client", "various",
];

/// Fold accents and Nordic/German letters so "Ørsted" matches "Orsted" and "Straße" "Strasse".
fn fold(c: char) -> &'static str {
    match c {
        'à' | 'á' | 'â' | 'ã' | 'ä' | 'å' | 'ā' | 'ą' => "a",
        'æ' => "ae",
        'ç' | 'č' | 'ć' => "c",
        'ď' | 'ð' => "d",
        'è' | 'é' | 'ê' | 'ë' | 'ē' | 'ę' | 'ě' => "e",
        'ì' | 'í' | 'î' | 'ï' | 'ı' | 'ī' => "i",
        'ł' => "l",
        'ñ' | 'ń' | 'ň' => "n",
        'ò' | 'ó' | 'ô' | 'õ' | 'ö' | 'ø' | 'ō' | 'ő' => "o",
        'œ' => "oe",
        'ř' => "r",
        'ß' => "ss",
        'š' | 'ś' => "s",
        'ť' => "t",
        'ù' | 'ú' | 'û' | 'ü' | 'ū' | 'ů' | 'ű' => "u",
        'ý' | 'ÿ' => "y",
        'ž' | 'ź' | 'ż' => "z",
        'þ' => "th",
        _ => "",
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CompanyTokens {
    pub key: String,
    pub distinctive: Vec<String>,
}

pub fn company_tokens(name: &str) -> CompanyTokens {
    // Parentheticals are aliases or regions ("Meta (Facebook)"); keep only the main name.
    let mut base = String::new();
    let mut depth = 0;
    for c in name.chars() {
        match c {
            '(' => depth += 1,
            ')' => depth = (depth - 1i32).max(0),
            _ if depth == 0 => base.push(c),
            _ => {}
        }
    }
    let all: Vec<String> = base
        .to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|t| !t.is_empty())
        .map(|t| t.chars().map(|c| { let f = fold(c); if f.is_empty() { c.to_string() } else { f.to_string() } }).collect::<String>())
        .collect();
    // Single letters are fragments of legal forms ("A/S", "S.A."), never the name.
    CompanyTokens { key: all.concat(), distinctive: all.into_iter().filter(|t| t.chars().count() > 1 && !GENERIC.contains(&t.as_str())).collect() }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Match {
    /// Same name, ignoring spacing and punctuation: "PayPal" and "Pay Pal".
    Exact,
    /// Same distinctive words, only filler differs: "Akamai" and "Akamai Technologies".
    Strong,
    /// Shares a distinctive word but has others too: "Epic" and "Epic Games". Probably not.
    Weak,
}

pub fn match_company(a: &CompanyTokens, b: &CompanyTokens) -> Option<Match> {
    if a.key.is_empty() || b.key.is_empty() {
        return None;
    }
    if a.key == b.key {
        return Some(Match::Exact);
    }
    let da: HashSet<&String> = a.distinctive.iter().collect();
    let db: HashSet<&String> = b.distinctive.iter().collect();
    if da.is_empty() || db.is_empty() {
        return None;
    }
    let shared: Vec<&&String> = da.intersection(&db).collect();
    if shared.is_empty() {
        return None;
    }
    let identical = da == db;
    let substantive = shared.iter().any(|t| t.chars().count() >= 3);
    Some(if identical && substantive { Match::Strong } else { Match::Weak })
}

// ── Connections.csv ─────────────────────────────────────────────────────

/// RFC 4180 rows: quoted fields, doubled quotes, commas and newlines inside quotes.
fn csv_rows(text: &str) -> Vec<Vec<String>> {
    let mut rows = Vec::new();
    let mut row = Vec::new();
    let mut field = String::new();
    let mut quoted = false;
    let mut chars = text.trim_start_matches('\u{feff}').chars().peekable();
    while let Some(c) = chars.next() {
        match (quoted, c) {
            (true, '"') if chars.peek() == Some(&'"') => {
                field.push('"');
                chars.next();
            }
            (true, '"') => quoted = false,
            (true, c) => field.push(c),
            (false, '"') if field.is_empty() => quoted = true,
            (false, ',') => row.push(std::mem::take(&mut field)),
            (false, '\r') => {}
            (false, '\n') => {
                row.push(std::mem::take(&mut field));
                rows.push(std::mem::take(&mut row));
            }
            (false, c) => field.push(c),
        }
    }
    if !field.is_empty() || !row.is_empty() {
        row.push(field);
        rows.push(row);
    }
    rows
}

/// "15 Oct 2024" (LinkedIn's format) or "2024-10-15".
fn parse_connected(s: &str) -> Option<NaiveDate> {
    let s = s.trim();
    NaiveDate::parse_from_str(s, "%Y-%m-%d").or_else(|_| NaiveDate::parse_from_str(s, "%d %b %Y")).ok()
}

#[derive(Debug, Default, PartialEq)]
pub struct Parsed {
    pub contacts: Vec<Contact>,
    /// Rows skipped because they had no company (LinkedIn leaves it blank for many).
    pub without_company: usize,
}

/// Parse LinkedIn's export. It starts with a few lines of notes before the header row.
pub fn parse_connections(text: &str) -> Result<Parsed, String> {
    let rows = csv_rows(text);
    let header = rows
        .iter()
        .position(|r| { let l: Vec<String> = r.iter().map(|c| c.trim().to_lowercase()).collect(); l.contains(&"first name".to_string()) && l.contains(&"company".to_string()) })
        .ok_or("This doesn't look like LinkedIn's Connections.csv: there's no row with First Name and Company columns.")?;
    let cols: Vec<String> = rows[header].iter().map(|c| c.trim().to_lowercase()).collect();
    let col = |name: &str| cols.iter().position(|c| c == name);
    let (first, last, url, email, company, position, connected) = (col("first name"), col("last name"), col("url"), col("email address"), col("company"), col("position"), col("connected on"));
    let mut out = Parsed::default();
    for r in &rows[header + 1..] {
        let get = |i: Option<usize>| i.and_then(|i| r.get(i)).map(|s| s.trim().to_string()).unwrap_or_default();
        let c = Contact { first_name: get(first), last_name: get(last), company: get(company), position: get(position), email: get(email), url: get(url), connected_on: parse_connected(&get(connected)), ..Default::default() };
        if c.first_name.is_empty() && c.last_name.is_empty() {
            continue;
        }
        if c.company.is_empty() {
            out.without_company += 1;
            continue;
        }
        if !c.url.is_empty() && !c.url.starts_with("https://") {
            continue;
        }
        out.contacts.push(c);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn m(a: &str, b: &str) -> Option<Match> {
        match_company(&company_tokens(a), &company_tokens(b))
    }

    #[test]
    fn matches_company_spellings_but_not_lookalikes() {
        assert_eq!(m("PayPal", "Pay Pal"), Some(Match::Exact));
        assert_eq!(m("Supabase", "Supabase Inc."), Some(Match::Strong));
        assert_eq!(m("Akamai", "Akamai Technologies"), Some(Match::Strong));
        assert_eq!(m("Ørsted A/S", "Orsted"), Some(Match::Strong));
        assert_eq!(m("Meta (Facebook)", "Meta"), Some(Match::Exact));
        assert_eq!(m("Epic", "Epic Games"), Some(Match::Weak));
        assert_eq!(m("Stealth Startup", "Stealth AI"), None);
        assert_eq!(m("Lunar", "Wise"), None);
    }

    #[test]
    fn parses_linkedin_export_with_notes_and_quotes() {
        let csv = "\u{feff}Notes:\n\"When exporting your connection data, you may notice that some of the email addresses are missing.\"\n\nFirst Name,Last Name,URL,Email Address,Company,Position,Connected On\nAda,Lovelace,https://www.linkedin.com/in/ada,,\"Supabase, Inc.\",\"Engineer, Realtime\",15 Oct 2024\nGrace,Hopper,https://www.linkedin.com/in/grace,grace@example.com,,Admiral,01 Jan 2020\nLinus,\"O\"\"Brien\",https://www.linkedin.com/in/linus,,Lunar,Staff Engineer,2023-05-02\n";
        let p = parse_connections(csv).unwrap();
        assert_eq!(p.without_company, 1);
        assert_eq!(p.contacts.len(), 2);
        assert_eq!(p.contacts[0].company, "Supabase, Inc.");
        assert_eq!(p.contacts[0].position, "Engineer, Realtime");
        assert_eq!(p.contacts[0].connected_on, NaiveDate::from_ymd_opt(2024, 10, 15));
        assert_eq!(p.contacts[1].name(), "Linus O\"Brien");
        assert!(parse_connections("name,email\nx,y").is_err());
    }
}
