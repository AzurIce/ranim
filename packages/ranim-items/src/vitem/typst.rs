use std::{
    collections::HashMap,
    io::Write,
    num::NonZeroUsize,
    sync::{Arc, Mutex, OnceLock},
};

use chrono::{DateTime, Datelike, Local};
use diff_match_patch_rs::{Efficient, Ops};
use lru::LruCache;
use regex::bytes::Regex;
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

use crate::vitem::{Group, VItem, svg::SvgItem};
use ranim_core::{
    Extract, color,
    components::width::Width,
    glam,
    primitives::{CoreItem, vitem::VItemPrimitive},
    traits::{Anchor, *},
};

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
                let world = typst_world().lock().unwrap();
                let world = world.with_source_str(typst_str);
                // world.set_source(typst_str);
                let document = typst::compile(&world)
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

fn typst_world() -> &'static Arc<Mutex<TypstWorld>> {
    static WORLD: OnceLock<Arc<Mutex<TypstWorld>>> = OnceLock::new();
    WORLD.get_or_init(|| Arc::new(Mutex::new(TypstWorld::new())))
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

struct FileEntry {
    bytes: Bytes,
    /// This field is filled on demand.
    source: Option<Source>,
}

impl FileEntry {
    fn source(&mut self, id: FileId) -> FileResult<Source> {
        // Fallible `get_or_insert`.
        let source = if let Some(source) = &self.source {
            source
        } else {
            let contents = std::str::from_utf8(&self.bytes).map_err(|_| FileError::InvalidUtf8)?;
            // Defuse the BOM!
            let contents = contents.trim_start_matches('\u{feff}');
            let source = Source::new(id, contents.into());
            self.source.insert(source)
        };
        Ok(source.clone())
    }
}

pub(crate) struct TypstWorld {
    library: LazyHash<Library>,
    book: LazyHash<FontBook>,
    files: Mutex<HashMap<FileId, FileEntry>>,
}

impl TypstWorld {
    pub(crate) fn new() -> Self {
        let fonts = fonts();
        Self {
            library: LazyHash::new(Library::default()),
            book: LazyHash::new(fonts.book.clone()),
            files: Mutex::new(HashMap::new()),
        }
    }
    pub(crate) fn with_source_str(&self, source: &str) -> TypstWorldWithSource<'_> {
        self.with_source(Source::detached(source))
    }
    pub(crate) fn with_source(&self, source: Source) -> TypstWorldWithSource<'_> {
        TypstWorldWithSource {
            world: self,
            source,
            now: OnceLock::new(),
        }
    }

    // from https://github.com/mattfbacon/typst-bot
    // TODO: package things
    // Weird pattern because mapping a MutexGuard is not stable yet.
    fn file<T>(&self, id: FileId, map: impl FnOnce(&mut FileEntry) -> T) -> FileResult<T> {
        let mut files = self.files.lock().unwrap();
        if let Some(entry) = files.get_mut(&id) {
            return Ok(map(entry));
        }
        // `files` must stay locked here so we don't download the same package multiple times.
        // TODO proper multithreading, maybe with typst-kit.

        // 'x: {
        // 	if let Some(package) = id.package() {
        // 		let package_dir = self.ensure_package(package)?;
        // 		let Some(path) = id.vpath().resolve(&package_dir) else {
        // 			break 'x;
        // 		};
        // 		let contents = std::fs::read(&path).map_err(|error| FileError::from_io(error, &path))?;
        // 		let entry = files.entry(id).or_insert(FileEntry {
        // 			bytes: Bytes::new(contents),
        // 			source: None,
        // 		});
        // 		return Ok(map(entry));
        // 	}
        // }

        Err(FileError::NotFound(id.vpath().as_rootless_path().into()))
    }
}

pub(crate) struct TypstWorldWithSource<'a> {
    world: &'a TypstWorld,
    source: Source,
    now: OnceLock<DateTime<Local>>,
}

impl World for TypstWorldWithSource<'_> {
    fn library(&self) -> &LazyHash<Library> {
        &self.world.library
    }

    fn book(&self) -> &LazyHash<FontBook> {
        &self.world.book
    }

    fn main(&self) -> FileId {
        self.source.id()
    }

    fn source(&self, id: FileId) -> FileResult<Source> {
        if id == self.source.id() {
            Ok(self.source.clone())
        } else {
            self.world.file(id, |entry| entry.source(id))?
        }
    }

    fn file(&self, id: FileId) -> FileResult<Bytes> {
        self.world.file(id, |file| file.bytes.clone())
    }

    fn font(&self, index: usize) -> Option<Font> {
        fonts().fonts[index].get()
    }

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

/// A Text item construted through typst
///
/// Note that the methods this item provides assumes that the typst string
/// you provide only produces text output, otherwise undefined behaviours may happens.
#[derive(Clone)]
pub struct TypstText {
    chars: String,
    vitems: Group<VItem>,
}

impl TypstText {
    fn _new(str: &str) -> Self {
        let svg = SvgItem::new(typst_svg(str));
        let chars = str.to_string();

        let vitems = Group::<VItem>::from(svg);
        assert_eq!(chars.len(), vitems.len());
        Self { chars, vitems }
    }
    /// Create a TypstText with typst string.
    ///
    /// The typst string you provide should only produces text output,
    /// otherwise undefined behaviours may happens.
    pub fn new(typst_str: &str) -> Self {
        let svg = SvgItem::new(typst_svg(typst_str));
        let chars = typst_str
            .replace(" ", "")
            .replace("\n", "")
            .replace("\r", "")
            .replace("\t", "");

        let vitems = Group::<VItem>::from(svg);
        assert_eq!(chars.len(), vitems.len());
        Self { chars, vitems }
    }

    /// Inline code
    pub fn new_inline_code(code: &str) -> Self {
        let svg = SvgItem::new(typst_svg(format!("`{code}`").as_str()));
        let chars = code
            .replace(" ", "")
            .replace("\n", "")
            .replace("\r", "")
            .replace("\t", "");

        let vitems = Group::<VItem>::from(svg);
        assert_eq!(chars.len(), vitems.len());
        Self { chars, vitems }
    }

    /// Multiline code
    pub fn new_multiline_code(code: &str, language: Option<&str>) -> Self {
        let language = language.unwrap_or("");
        // Self::new(format!("```{language}\n{code}\n```").as_str())
        let svg = SvgItem::new(typst_svg(format!("```{language}\n{code}```").as_str()));
        let chars = code
            .replace(" ", "")
            .replace("\n", "")
            .replace("\r", "")
            .replace("\t", "");

        let vitems = Group::<VItem>::from(svg);
        assert_eq!(chars.len(), vitems.len());
        Self { chars, vitems }
    }
}

impl Alignable for TypstText {
    fn is_aligned(&self, other: &Self) -> bool {
        self.vitems.len() == other.vitems.len()
            && self
                .vitems
                .iter()
                .zip(&other.vitems)
                .all(|(a, b)| a.is_aligned(b))
    }
    fn align_with(&mut self, other: &mut Self) {
        let dmp = diff_match_patch_rs::DiffMatchPatch::new();
        let diffs = dmp
            .diff_main::<Efficient>(&self.chars, &other.chars)
            .unwrap();

        let len = self.vitems.len().max(other.vitems.len());
        let mut vitems_self: Vec<VItem> = Vec::with_capacity(len);
        let mut vitems_other: Vec<VItem> = Vec::with_capacity(len);
        let mut ia = 0;
        let mut ib = 0;
        let mut last_neq_idx_a = 0;
        let mut last_neq_idx_b = 0;
        let align_and_push_diff = |vitems_self: &mut Vec<VItem>,
                                   vitems_other: &mut Vec<VItem>,
                                   ia,
                                   ib,
                                   last_neq_idx_a,
                                   last_neq_idx_b| {
            if last_neq_idx_a != ia || last_neq_idx_b != ib {
                let mut vitems_a = self.vitems[last_neq_idx_a..ia]
                    .iter()
                    .cloned()
                    .collect::<Group<_>>();
                let mut vitems_b = other.vitems[last_neq_idx_b..ib]
                    .iter()
                    .cloned()
                    .collect::<Group<_>>();
                if vitems_a.is_empty() {
                    vitems_a.extend(vitems_b.iter().map(|x| {
                        x.clone().with(|x| {
                            x.shrink();
                        })
                    }));
                }
                if vitems_b.is_empty() {
                    vitems_b.extend(vitems_a.iter().map(|x| {
                        x.clone().with(|x| {
                            x.shrink();
                        })
                    }));
                }
                if last_neq_idx_a != ia && last_neq_idx_b != ib {
                    vitems_a.align_with(&mut vitems_b);
                }
                vitems_self.extend(vitems_a);
                vitems_other.extend(vitems_b);
            }
        };

        for diff in &diffs {
            // println!("[{ia}] {last_neq_idx_a} [{ib}] {last_neq_idx_b}");
            // println!("{diff:?}");
            match diff.op() {
                Ops::Equal => {
                    align_and_push_diff(
                        &mut vitems_self,
                        &mut vitems_other,
                        ia,
                        ib,
                        last_neq_idx_a,
                        last_neq_idx_b,
                    );
                    let l = diff.size();
                    vitems_self.extend(self.vitems[ia..ia + l].iter().cloned());
                    vitems_other.extend(other.vitems[ib..ib + l].iter().cloned());
                    ia += l;
                    ib += l;
                    last_neq_idx_a = ia;
                    last_neq_idx_b = ib;
                }
                Ops::Delete => {
                    ia += diff.size();
                }
                Ops::Insert => {
                    ib += diff.size();
                }
            }
        }
        align_and_push_diff(
            &mut vitems_self,
            &mut vitems_other,
            ia,
            ib,
            last_neq_idx_a,
            last_neq_idx_b,
        );

        assert_eq!(vitems_self.len(), vitems_other.len());
        vitems_self
            .iter_mut()
            .zip(vitems_other.iter_mut())
            .for_each(|(a, b)| {
                // println!("{i} {}", a.is_aligned(b));
                // println!("{} {}", a.vpoints.len(), b.vpoints.len());
                if !a.is_aligned(b) {
                    a.align_with(b);
                }
            });

        self.vitems = Group(vitems_self);
        other.vitems = Group(vitems_other);
    }
}

impl Interpolatable for TypstText {
    fn lerp(&self, target: &Self, t: f64) -> Self {
        let vitems = self
            .vitems
            .iter()
            .zip(&target.vitems)
            .map(|(a, b)| a.lerp(b, t))
            .collect::<Group<_>>();
        Self {
            chars: self.chars.clone(),
            vitems,
        }
    }
}

impl From<TypstText> for Group<VItem> {
    fn from(value: TypstText) -> Self {
        value.vitems
    }
}

impl Extract for TypstText {
    type Target = CoreItem;
    fn extract(&self) -> Vec<Self::Target> {
        self.vitems.extract()
    }
}

impl BoundingBox for TypstText {
    fn get_bounding_box(&self) -> [glam::DVec3; 3] {
        self.vitems.get_bounding_box()
    }
}

impl Shift for TypstText {
    fn shift(&mut self, shift: glam::DVec3) -> &mut Self {
        self.vitems.shift(shift);
        self
    }
}

impl Rotate for TypstText {
    fn rotate_by_anchor(&mut self, angle: f64, axis: glam::DVec3, anchor: Anchor) -> &mut Self {
        self.vitems.rotate_by_anchor(angle, axis, anchor);
        self
    }
}

impl Scale for TypstText {
    fn scale_by_anchor(&mut self, scale: glam::DVec3, anchor: Anchor) -> &mut Self {
        self.vitems.scale_by_anchor(scale, anchor);
        self
    }
}

impl FillColor for TypstText {
    fn fill_color(&self) -> color::AlphaColor<color::Srgb> {
        self.vitems[0].fill_color()
    }
    fn set_fill_color(&mut self, color: color::AlphaColor<color::Srgb>) -> &mut Self {
        self.vitems.set_fill_color(color);
        self
    }
    fn set_fill_opacity(&mut self, opacity: f32) -> &mut Self {
        self.vitems.set_fill_opacity(opacity);
        self
    }
}

impl StrokeColor for TypstText {
    fn stroke_color(&self) -> color::AlphaColor<color::Srgb> {
        self.vitems[0].fill_color()
    }
    fn set_stroke_color(&mut self, color: color::AlphaColor<color::Srgb>) -> &mut Self {
        self.vitems.set_stroke_color(color);
        self
    }
    fn set_stroke_opacity(&mut self, opacity: f32) -> &mut Self {
        self.vitems.set_stroke_opacity(opacity);
        self
    }
}

impl Opacity for TypstText {
    fn set_opacity(&mut self, opacity: f32) -> &mut Self {
        self.vitems.set_fill_opacity(opacity);
        self.vitems.set_stroke_opacity(opacity);
        self
    }
}

impl StrokeWidth for TypstText {
    fn stroke_width(&self) -> f32 {
        self.vitems.stroke_width()
    }
    fn apply_stroke_func(&mut self, f: impl for<'a> Fn(&'a mut [Width])) -> &mut Self {
        self.vitems.iter_mut().for_each(|vitem| {
            vitem.apply_stroke_func(&f);
        });
        self
    }
    fn set_stroke_width(&mut self, width: f32) -> &mut Self {
        self.vitems.set_stroke_width(width);
        self
    }
}

/// remove `r"<path[^>]*(?:>.*?<\/path>|\/>)"`
pub fn get_typst_element(svg: &str) -> String {
    let re = Regex::new(r"<path[^>]*(?:>.*?<\/path>|\/>)").unwrap();
    let removed_bg = re.replace(svg.as_bytes(), b"");

    // println!("{}", String::from_utf8_lossy(&output));
    // println!("{}", String::from_utf8_lossy(&removed_bg));
    String::from_utf8_lossy(&removed_bg).to_string()
}

/// Compiles typst code to SVG string by spawning a typst process
pub fn compile_typst_code(typst_code: &str) -> String {
    let mut child = std::process::Command::new("typst")
        .arg("compile")
        .arg("-")
        .arg("-")
        .arg("-fsvg")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .expect("failed to spawn typst");

    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(typst_code.as_bytes())
            .expect("failed to write to typst's stdin");
    }

    let output = child.wait_with_output().unwrap().stdout;
    let output = String::from_utf8_lossy(&output);

    get_typst_element(&output)
}

#[cfg(test)]
mod tests {
    use std::time::Instant;

    use super::*;

    /*
    fonts search: 322.844709ms
    world construct: 1.901541ms
    set source: 958ns
    file: 736
    file: 818
    document compile: 89.835583ms
    svg output: 185.458µs
    get element: 730.792µs
     */
    #[test]
    fn test_single_file_typst_world_foo() {
        let start = Instant::now();
        fonts();
        println!("fonts search: {:?}", start.elapsed());

        let start = Instant::now();
        let world = TypstWorld::new();
        println!("world construct: {:?}", start.elapsed());

        let start = Instant::now();
        let world = world.with_source_str("r");
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

    #[test]
    fn foo() {
        let code_a = r#"#include <iostream>
using namespace std;

int main() {
    cout << "Hello World!" << endl;
}
"#;
        let mut code_a = TypstText::new_multiline_code(code_a, Some("cpp"));
        let code_b = r#"fn main() {
    println!("Hello World!");
}"#;
        let mut code_b = TypstText::new_multiline_code(code_b, Some("rust"));

        code_a.align_with(&mut code_b);
    }
}
