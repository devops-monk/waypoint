//! Word (.docx) export for resumes and cover letters. Same content and template family as
//! the PDFs, laid out as plain single-column Word paragraphs: real bullet lists, a right
//! tab for dates and no text boxes or tables, which is what applicant tracking systems
//! and recruiters editing the file handle best. Arial is used because every Mac and
//! Windows machine has it; Inter, the PDF font, usually isn't installed on the reader's.

use std::io::Cursor;

use anyhow::Result;
use docx_rs::{
    AbstractNumbering, AlignmentType, Docx, IndentLevel, Level, LevelJc, LevelText, LineSpacing, NumberFormat, Numbering, NumberingId, PageMargin, Paragraph, ParagraphBorder, ParagraphBorderPosition, Run, RunFonts, SpecialIndentType, Start, Tab, TabValueType,
};

use crate::letter::LetterDoc;
use crate::resume::{ResumeDoc, Template};

const FONT: &str = "Arial";
const INK: &str = "1D1D1F";
const MUTED: &str = "6E6E73";
const RULE: &str = "D2D2D7";
const ACCENT: &str = "4A57E8";
const ACCENT_RULE: &str = "B9BFF6";
const BULLETS: usize = 1;

struct Page {
    width: u32,
    height: u32,
    margin: i32,
}

impl Page {
    fn for_paper(paper: &str) -> Self {
        // Twentieths of a point. ~1.8 cm side margins, like the PDF templates.
        if paper == "us-letter" { Page { width: 12240, height: 15840, margin: 1020 } } else { Page { width: 11906, height: 16838, margin: 1020 } }
    }

    /// Where a right-aligned tab stop lands: the right edge of the text column.
    fn right_tab(&self) -> usize {
        (self.width as i32 - 2 * self.margin) as usize
    }
}

struct Style {
    template: Template,
}

impl Style {
    fn accent(&self) -> &'static str {
        if self.template == Template::Modern { ACCENT } else { INK }
    }

    fn rule(&self) -> &'static str {
        if self.template == Template::Modern { ACCENT_RULE } else { RULE }
    }
}

fn base(page: &Page) -> Docx {
    Docx::new()
        .page_size(page.width, page.height)
        .page_margin(PageMargin { top: 850, bottom: 850, left: page.margin, right: page.margin, header: 425, footer: 425, gutter: 0 })
        .default_fonts(RunFonts::new().ascii(FONT).hi_ansi(FONT).cs(FONT).east_asia(FONT))
        .default_size(20) // 10 pt; Arial runs wider than Inter
        .add_abstract_numbering(
            AbstractNumbering::new(BULLETS).add_level(Level::new(0, Start::new(1), NumberFormat::new("bullet"), LevelText::new("•"), LevelJc::new("left")).indent(Some(300), Some(SpecialIndentType::Hanging(220)), None, None)),
        )
        .add_numbering(Numbering::new(BULLETS, BULLETS))
}

fn text(s: &str) -> Run {
    Run::new().add_text(s).color(INK)
}

fn muted(s: &str) -> Run {
    Run::new().add_text(s).color(MUTED)
}

fn spaced(p: Paragraph, before: u32, after: u32) -> Paragraph {
    p.line_spacing(LineSpacing::new().before(before).after(after))
}

fn rule_below(mut p: Paragraph, color: &str) -> Paragraph {
    p.property = p.property.set_border(ParagraphBorder::new(ParagraphBorderPosition::Bottom).size(4).space(2).color(color));
    p
}

fn header(docx: Docx, name: &str, headline: &str, contact: &[String], style: &Style) -> Docx {
    let mut docx = docx.add_paragraph(spaced(Paragraph::new().add_run(Run::new().add_text(name).bold().size(40).color(style.accent())), 0, 20));
    if !headline.is_empty() {
        docx = docx.add_paragraph(spaced(Paragraph::new().add_run(Run::new().add_text(headline).size(23).color("424245")), 0, 20));
    }
    if !contact.is_empty() {
        docx = docx.add_paragraph(spaced(Paragraph::new().add_run(muted(&contact.join("   |   ")).size(18)), 0, 60));
    }
    docx
}

fn section(title: &str, style: &Style) -> Paragraph {
    let p = Paragraph::new().add_run(Run::new().add_text(title).bold().size(22).color(style.accent())).keep_next(true);
    rule_below(spaced(p, 220, 80), style.rule())
}

fn bullet(s: &str) -> Paragraph {
    spaced(Paragraph::new().add_run(text(s)).numbering(NumberingId::new(BULLETS), IndentLevel::new(0)), 0, 40)
}

/// "What, where" on the left and the dates flush right on the same line.
fn line_with_dates(left: Vec<Run>, dates: &str, page: &Page) -> Paragraph {
    let mut p = Paragraph::new().add_tab(Tab::new().val(TabValueType::Right).pos(page.right_tab())).keep_next(true);
    for r in left {
        p = p.add_run(r);
    }
    if !dates.trim().is_empty() {
        p = p.add_run(Run::new().add_tab()).add_run(muted(dates).size(19));
    }
    p
}

fn dates(start: &str, end: &str) -> String {
    match (start.trim(), end.trim()) {
        ("", "") => String::new(),
        (s, "") => s.to_string(),
        ("", e) => e.to_string(),
        (s, e) => format!("{s} – {e}"),
    }
}

fn pack(docx: Docx) -> Result<Vec<u8>> {
    let mut buf = Cursor::new(Vec::new());
    docx.build().pack(&mut buf)?;
    Ok(buf.into_inner())
}

pub fn resume(doc: &ResumeDoc, template: Template) -> Result<Vec<u8>> {
    let page = Page::for_paper(&doc.paper);
    let style = Style { template };
    let mut docx = header(base(&page), &doc.name, &doc.headline, &doc.contact, &style);

    if !doc.summary.trim().is_empty() {
        docx = docx.add_paragraph(section("Summary", &style)).add_paragraph(spaced(Paragraph::new().add_run(text(&doc.summary)), 0, 40));
    }
    if !doc.experience.is_empty() {
        docx = docx.add_paragraph(section("Experience", &style));
        for (i, r) in doc.experience.iter().enumerate() {
            let mut left = vec![Run::new().add_text(&r.title).bold().color(INK)];
            if !r.company.is_empty() {
                left.push(text(&format!(", {}", r.company)));
            }
            if !r.location.is_empty() {
                left.push(muted(&format!("  ·  {}", r.location)));
            }
            docx = docx.add_paragraph(spaced(line_with_dates(left, &dates(&r.start, &r.end), &page), if i == 0 { 0 } else { 140 }, 40));
            for b in r.bullets.iter().filter(|b| !b.trim().is_empty()) {
                docx = docx.add_paragraph(bullet(b));
            }
        }
    }
    if !doc.skills.is_empty() {
        docx = docx.add_paragraph(section("Skills", &style)).add_paragraph(Paragraph::new().add_run(text(&doc.skills.join(", "))));
    }
    if !doc.education.is_empty() {
        docx = docx.add_paragraph(section("Education", &style));
        for e in &doc.education {
            let mut left = vec![Run::new().add_text(&e.degree).bold().color(INK)];
            if !e.institution.is_empty() {
                left.push(text(&format!(", {}", e.institution)));
            }
            docx = docx.add_paragraph(spaced(line_with_dates(left, &dates(&e.start, &e.end), &page), 0, 40));
        }
    }
    if !doc.languages.is_empty() {
        docx = docx.add_paragraph(section("Languages", &style)).add_paragraph(Paragraph::new().add_run(text(&doc.languages.join(", "))));
    }
    pack(docx)
}

pub fn letter(l: &LetterDoc, template: Template) -> Result<Vec<u8>> {
    let page = Page::for_paper(&l.paper);
    let style = Style { template };
    let mut docx = header(base(&page), &l.name, "", &l.contact, &style);
    docx = docx.add_paragraph(rule_below(spaced(Paragraph::new(), 0, 200), style.rule()));

    // Recipient block, with the date flush right on its first line.
    let recipient: Vec<&String> = l.recipient.iter().filter(|r| !r.trim().is_empty()).collect();
    let first = recipient.first().map(|s| s.as_str()).unwrap_or("");
    docx = docx.add_paragraph(spaced(line_with_dates(vec![text(first)], &l.date, &page), 0, 0));
    for r in recipient.iter().skip(1) {
        docx = docx.add_paragraph(spaced(Paragraph::new().add_run(text(r)), 0, 0));
    }

    let para = |s: &str| spaced(Paragraph::new().add_run(text(s)), 0, 160).line_spacing(LineSpacing::new().after(160).line(276));
    if !l.subject.trim().is_empty() {
        docx = docx.add_paragraph(spaced(Paragraph::new().add_run(Run::new().add_text(&l.subject).bold().size(22).color(style.accent())), 360, 120));
    } else {
        docx = docx.add_paragraph(spaced(Paragraph::new(), 240, 0));
    }
    if !l.salutation.trim().is_empty() {
        docx = docx.add_paragraph(para(&l.salutation));
    }
    if !l.opening.trim().is_empty() {
        docx = docx.add_paragraph(para(&l.opening));
    }
    for h in l.highlights.iter().filter(|h| !h.trim().is_empty()) {
        docx = docx.add_paragraph(bullet(h).line_spacing(LineSpacing::new().after(80).line(276)));
    }
    if !l.highlights.is_empty() {
        docx = docx.add_paragraph(spaced(Paragraph::new(), 0, 0));
    }
    for b in l.body.iter().filter(|b| !b.trim().is_empty()) {
        docx = docx.add_paragraph(para(b));
    }
    docx = docx.add_paragraph(spaced(Paragraph::new().add_run(text(&l.closing)), 120, 480));
    docx = docx.add_paragraph(Paragraph::new().add_run(Run::new().add_text(&l.signature).bold().color(INK)).align(AlignmentType::Left));
    pack(docx)
}

/// The visible text of a .docx, for checks: paragraphs separated by newlines.
pub fn plain_text(docx: &[u8]) -> Result<String> {
    let d = docx_rs::read_docx(docx)?;
    let mut out = String::new();
    for child in &d.document.children {
        if let docx_rs::DocumentChild::Paragraph(p) = child {
            out.push_str(&p.raw_text());
            out.push('\n');
        }
    }
    Ok(out)
}
