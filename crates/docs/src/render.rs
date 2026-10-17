//! Embedded Typst: compiles a template plus the resume as JSON, with bundled fonts and
//! no file system or network access.

use std::sync::OnceLock;

use anyhow::{Result, anyhow};
use chrono::Datelike;
use typst::diag::{FileError, FileResult, SourceDiagnostic};
use typst::foundations::{Bytes, Datetime, Duration};
use typst_layout::PagedDocument;
use typst::syntax::{FileId, RootedPath, Source, VirtualPath, VirtualRoot};
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;
use typst::{Library, LibraryExt, World};

use crate::resume::{ResumeDoc, Template};

struct Fonts {
    book: LazyHash<FontBook>,
    fonts: Vec<Font>,
}

fn fonts() -> &'static Fonts {
    static FONTS: OnceLock<Fonts> = OnceLock::new();
    FONTS.get_or_init(|| {
        let bundled: [&'static [u8]; 5] = [
            include_bytes!("../fonts/Inter-Regular.ttf"),
            include_bytes!("../fonts/Inter-Italic.ttf"),
            include_bytes!("../fonts/Inter-Medium.ttf"),
            include_bytes!("../fonts/Inter-SemiBold.ttf"),
            include_bytes!("../fonts/Inter-Bold.ttf"),
        ];
        // Typst's own fonts are the fallback for scripts Inter doesn't cover.
        let fonts: Vec<Font> = bundled.into_iter().chain(typst_assets::fonts()).flat_map(|data| Font::iter(Bytes::new(data))).collect();
        Fonts { book: LazyHash::new(FontBook::from_fonts(&fonts)), fonts }
    })
}

fn library() -> &'static LazyHash<Library> {
    static LIB: OnceLock<LazyHash<Library>> = OnceLock::new();
    LIB.get_or_init(|| LazyHash::new(Library::default()))
}

struct ResumeWorld {
    main: Source,
    data_id: FileId,
    data: Bytes,
}

impl World for ResumeWorld {
    fn library(&self) -> &LazyHash<Library> {
        library()
    }

    fn book(&self) -> &LazyHash<FontBook> {
        &fonts().book
    }

    fn main(&self) -> FileId {
        self.main.id()
    }

    fn source(&self, id: FileId) -> FileResult<Source> {
        if id == self.main.id() { Ok(self.main.clone()) } else { Err(FileError::NotFound(std::path::PathBuf::from("resume asset"))) }
    }

    fn file(&self, id: FileId) -> FileResult<Bytes> {
        if id == self.data_id { Ok(self.data.clone()) } else { Err(FileError::NotFound(std::path::PathBuf::from("resume asset"))) }
    }

    fn font(&self, index: usize) -> Option<Font> {
        fonts().fonts.get(index).cloned()
    }

    fn today(&self, _offset: Option<Duration>) -> Option<Datetime> {
        let now = chrono::Local::now();
        Datetime::from_ymd(now.year(), now.month() as u8, now.day() as u8)
    }
}

fn diagnostics(errors: &[SourceDiagnostic]) -> anyhow::Error {
    anyhow!("couldn't lay out the resume: {}", errors.iter().map(|e| e.message.to_string()).collect::<Vec<_>>().join("; "))
}

pub fn compile(doc: &ResumeDoc, template: Template) -> Result<PagedDocument> {
    let file = |name: &str| FileId::new(RootedPath::new(VirtualRoot::Project, VirtualPath::new(name).expect("static path")));
    let main_id = file("main.typ");
    let world = ResumeWorld {
        main: Source::new(main_id, template.source().to_string()),
        data_id: file("data.json"),
        data: Bytes::new(serde_json::to_vec(doc)?),
    };
    typst::compile::<PagedDocument>(&world).output.map_err(|e| diagnostics(e.as_slice()))
}

pub fn pdf(document: &PagedDocument) -> Result<Vec<u8>> {
    typst_pdf::pdf(document, &typst_pdf::PdfOptions::default()).map_err(|e| diagnostics(e.as_slice()))
}

pub fn svg_pages(document: &PagedDocument) -> Vec<String> {
    document.pages().iter().map(|p| typst_svg::svg(p, &typst_svg::SvgOptions::default())).collect()
}
