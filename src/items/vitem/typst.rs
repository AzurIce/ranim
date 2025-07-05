use std::sync::OnceLock;

use chrono::{DateTime, Datelike, Local};
use typst::{
    Library, World,
    diag::{FileError, FileResult},
    foundations::{Bytes, Datetime},
    layout::Abs,
    syntax::{FileId, Source},
    text::{Font, FontBook},
    utils::LazyHash,
};
use typst_kit::fonts::{FontSearcher, Fonts};

use crate::utils::typst::get_typst_element;

fn fonts() -> &'static Fonts {
    static FONTS: OnceLock<Fonts> = OnceLock::new();
    FONTS.get_or_init(|| FontSearcher::new().include_system_fonts(true).search())
}

pub fn typst_svg(source: &str) -> String {
    let world = SingleFileTypstWorld::new(source);
    let document = typst::compile(&world)
        .output
        .expect("failed to compile typst source");

    let svg = typst_svg::svg_merged(&document, Abs::pt(2.0));
    get_typst_element(&svg)
}

pub struct SingleFileTypstWorld {
    source: Source,

    library: LazyHash<Library>,
    book: LazyHash<FontBook>,
    now: OnceLock<DateTime<Local>>,
}

impl SingleFileTypstWorld {
    pub fn new(source: impl AsRef<str>) -> Self {
        let source = source.as_ref().to_string();
        let fonts = fonts();

        Self {
            library: LazyHash::new(Library::default()),
            book: LazyHash::new(fonts.book.clone()),
            source: Source::detached(source),
            now: OnceLock::new(),
        }
    }
}

impl World for SingleFileTypstWorld {
    fn library(&self) -> &LazyHash<Library> {
        &self.library
    }

    fn book(&self) -> &LazyHash<FontBook> {
        &self.book
    }

    fn main(&self) -> FileId {
        self.source.id()
    }

    fn source(&self, id: FileId) -> FileResult<Source> {
        if id == self.main() {
            Ok(self.source.clone())
        } else {
            Err(FileError::AccessDenied)
        }
    }

    fn file(&self, _id: FileId) -> FileResult<Bytes> {
        Err(FileError::AccessDenied)
    }

    fn font(&self, index: usize) -> Option<Font> {
        fonts().fonts[index].get()
    }

    // TODO: fix this
    fn today(&self, offset: Option<i64>) -> Option<Datetime> {
        let now = self.now.get_or_init(chrono::Local::now);

        let naive = match offset {
            None => now.naive_local(),
            Some(o) => now.naive_utc() + chrono::Duration::hours(o),
        };

        Datetime::from_ymd(
            naive.year(),
            naive.month().try_into().ok()?,
            naive.day().try_into().ok()?,
        )
    }
}

#[cfg(test)]
mod tests {
    use crate::typst_svg;

    use super::*;

    #[test]
    fn test_single_file_typst_world_foo() {
        let source = "R";

        println!("{}", typst_svg(source));
        println!("{}", typst_svg!(source))
    }
}
