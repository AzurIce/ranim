use std::sync::OnceLock;

use chrono::{DateTime, Datelike, Local};
use typst::{
    Library, LibraryExt, World,
    diag::{FileError, FileResult},
    foundations::{Bytes, Datetime, Duration},
    syntax::{FileId, Source},
    text::{Font, FontBook},
    utils::LazyHash,
};
use typst_kit::fonts::FontStore;

fn fonts() -> &'static FontStore {
    static FONTS: OnceLock<FontStore> = OnceLock::new();
    FONTS.get_or_init(|| {
        let mut fonts = FontStore::new();
        fonts.extend(typst_kit::fonts::embedded());
        fonts.extend(typst_kit::fonts::system());
        fonts
    })
}

pub(crate) struct SingleSourceWorld {
    library: LazyHash<Library>,
    book: LazyHash<FontBook>,
    source: Source,
    now: OnceLock<DateTime<Local>>,
}

impl SingleSourceWorld {
    pub(crate) fn new(source: &str) -> Self {
        Self {
            library: LazyHash::new(Library::default()),
            book: fonts().book().clone(),
            source: Source::detached(source),
            now: OnceLock::new(),
        }
    }
}

impl World for SingleSourceWorld {
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
        if id == self.source.id() {
            Ok(self.source.clone())
        } else {
            Err(FileError::NotFound(id.vpath().get_without_slash().into()))
        }
    }

    fn file(&self, id: FileId) -> FileResult<Bytes> {
        Err(FileError::NotFound(id.vpath().get_without_slash().into()))
    }

    fn font(&self, index: usize) -> Option<Font> {
        fonts().font(index)
    }

    fn today(&self, offset: Option<Duration>) -> Option<Datetime> {
        let now = self.now.get_or_init(Local::now);
        let date = match offset {
            None => now.naive_local(),
            Some(offset) => now.naive_utc() + chrono::Duration::seconds(offset.seconds() as i64),
        };

        Datetime::from_ymd(
            date.year(),
            date.month().try_into().ok()?,
            date.day().try_into().ok()?,
        )
    }
}
