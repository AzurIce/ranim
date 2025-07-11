use std::{
    num::NonZeroUsize,
    ops::Deref,
    sync::{Arc, Mutex, OnceLock},
};

use chrono::{DateTime, Datelike, Local};
use lru::LruCache;
use sha1::{Digest, Sha1};
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

struct TypstLruCache {
    inner: LruCache<[u8; 20], String>,
}

impl TypstLruCache {
    fn new(cap: NonZeroUsize) -> Self {
        Self {
            inner: LruCache::new(cap),
        }
    }
    // fn get(&mut self, typst_str: &str) -> Option<&String> {
    //     let mut sha1 = Sha1::new();
    //     sha1.update(typst_str.as_bytes());
    //     let sha1 = sha1.finalize();
    //     self.inner.get::<[u8; 20]>(sha1.as_ref())
    // }
    fn get_or_insert(&mut self, typst_str: &str) -> &String {
        let mut sha1 = Sha1::new();
        sha1.update(typst_str.as_bytes());
        let sha1 = sha1.finalize();
        self.inner
            .get_or_insert_ref(AsRef::<[u8; 20]>::as_ref(&sha1), || {
                // let world = SingleFileTypstWorld::new(typst_str);
                let mut world = typst_world().lock().unwrap();
                world.set_source(typst_str);
                let document = typst::compile(world.deref())
                    .output
                    .expect("failed to compile typst source");

                let svg = typst_svg::svg_merged(&document, Abs::pt(2.0));
                get_typst_element(&svg)
            })
    }
}

fn typst_lru() -> &'static Arc<Mutex<TypstLruCache>> {
    static LRU: OnceLock<Arc<Mutex<TypstLruCache>>> = OnceLock::new();
    LRU.get_or_init(|| {
        Arc::new(Mutex::new(TypstLruCache::new(
            NonZeroUsize::new(256).unwrap(),
        )))
    })
}

fn fonts() -> &'static Fonts {
    static FONTS: OnceLock<Fonts> = OnceLock::new();
    FONTS.get_or_init(|| FontSearcher::new().include_system_fonts(true).search())
}

fn typst_world() -> &'static Arc<Mutex<SingleFileTypstWorld>> {
    static WORLD: OnceLock<Arc<Mutex<SingleFileTypstWorld>>> = OnceLock::new();
    WORLD.get_or_init(|| Arc::new(Mutex::new(SingleFileTypstWorld::new(""))))
}

/// Compiles typst string to SVG string
pub fn typst_svg(source: &str) -> String {
    typst_lru().lock().unwrap().get_or_insert(source).clone()
    // let world = SingleFileTypstWorld::new(source);
    // let document = typst::compile(&world)
    //     .output
    //     .expect("failed to compile typst source");

    // let svg = typst_svg::svg_merged(&document, Abs::pt(2.0));
    // get_typst_element(&svg)
}

pub(crate) struct SingleFileTypstWorld {
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
    pub fn set_source(&mut self, source: impl AsRef<str>) {
        self.source = Source::detached(source.as_ref().to_string());
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
    use std::time::Instant;

    use super::*;

    #[test]
    fn test_single_file_typst_world_foo() {
        let source = "R";

        let start = Instant::now();
        let mut world = SingleFileTypstWorld::new(source);
        println!("world construct: {:?}", start.elapsed());

        let start = Instant::now();
        world.set_source("r");
        println!("set source: {:?}", start.elapsed());

        let start = Instant::now();
        let document = typst::compile(&world)
            .output
            .expect("failed to compile typst source");
        println!("document compile: {:?}", start.elapsed());

        let start = Instant::now();
        let svg = typst_svg::svg_merged(&document, Abs::pt(2.0));
        println!("svg output: {:?}", start.elapsed());

        let start = Instant::now();
        let res = get_typst_element(&svg);
        println!("get element: {:?}", start.elapsed());

        println!("{res}");
        // println!("{}", typst_svg!(source))
    }
}
