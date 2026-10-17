//! ATS check: read the text back out of the generated PDF, the way an applicant tracking
//! system would, and report what a parser can and can't find. Port of the ideas in
//! ai-job-search `tools/verify_pdf.py` and career-ops `verify-ats.mjs` / `keyword-match.mjs`.

use serde::{Deserialize, Serialize};

use crate::resume::ResumeDoc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeywordHit {
    pub keyword: String,
    pub found: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AtsReport {
    pub pages: usize,
    pub extracted_chars: usize,
    pub name_found: bool,
    pub missing_sections: Vec<String>,
    pub companies_in_order: bool,
    pub keywords: Vec<KeywordHit>,
    /// Share of the posting's keywords present in the extracted text, 0-1.
    pub coverage: f64,
    pub problems: Vec<String>,
}

fn contains_word(haystack: &str, needle: &str) -> bool {
    let n = needle.to_lowercase();
    if n.is_empty() {
        return false;
    }
    let h = haystack;
    let mut start = 0;
    while let Some(pos) = h[start..].find(&n) {
        let i = start + pos;
        let before = h[..i].chars().next_back();
        let after = h[i + n.len()..].chars().next();
        let boundary = |c: Option<char>| c.is_none_or(|c| !c.is_alphanumeric());
        if boundary(before) && boundary(after) {
            return true;
        }
        start = i + n.len().max(1);
    }
    false
}

pub fn check_pdf(pdf: &[u8], pages: usize, doc: &ResumeDoc, keywords: &[String]) -> AtsReport {
    let raw = pdf_extract::extract_text_from_mem(pdf).unwrap_or_default();
    let text = raw.to_lowercase().split_whitespace().collect::<Vec<_>>().join(" ");
    let mut problems = Vec::new();

    if text.len() < 200 {
        problems.push("Almost no text could be read from the PDF, so most tracking systems would see a blank resume.".into());
    }
    let name_found = !doc.name.trim().is_empty() && text.contains(&doc.name.to_lowercase().split_whitespace().collect::<Vec<_>>().join(" "));
    if !name_found {
        problems.push("Your name couldn't be read from the PDF.".into());
    }

    let mut expected = vec![];
    if !doc.experience.is_empty() {
        expected.push("Experience");
    }
    if !doc.skills.is_empty() {
        expected.push("Skills");
    }
    if !doc.education.is_empty() {
        expected.push("Education");
    }
    let missing_sections: Vec<String> = expected.into_iter().filter(|s| !text.contains(&s.to_lowercase())).map(String::from).collect();
    for s in &missing_sections {
        problems.push(format!("The “{s}” heading couldn't be read."));
    }

    // Parsers read top to bottom; roles should come out in the order they're shown.
    let positions: Vec<Option<usize>> = doc.experience.iter().map(|r| text.find(&r.company.to_lowercase())).collect();
    let companies_in_order = positions.iter().all(Option::is_some) && positions.windows(2).all(|w| w[0] <= w[1]);
    if !companies_in_order && !doc.experience.is_empty() {
        problems.push("Your roles don't read back in order, which can scramble your work history in tracking systems.".into());
    }
    if pages > 2 {
        problems.push(format!("The resume runs to {pages} pages. Most recruiters expect one or two."));
    }

    let keywords: Vec<KeywordHit> = keywords.iter().map(|k| KeywordHit { keyword: k.clone(), found: contains_word(&text, k) }).collect();
    let coverage = if keywords.is_empty() { 1.0 } else { keywords.iter().filter(|k| k.found).count() as f64 / keywords.len() as f64 };

    AtsReport { pages, extracted_chars: text.len(), name_found, missing_sections, companies_in_order, keywords, coverage, problems }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn word_boundaries() {
        assert!(contains_word("built services in go and rust", "Go"));
        assert!(!contains_word("google cloud", "go"));
        assert!(contains_word("node.js, c++ and sql", "c++"));
    }
}
