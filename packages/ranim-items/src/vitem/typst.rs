use std::{
    collections::HashMap,
    io::Write,
    num::NonZeroUsize,
    panic::{AssertUnwindSafe, catch_unwind},
    sync::{Arc, Mutex, OnceLock},
};

use chrono::{DateTime, Datelike, Local};
use diff_match_patch_rs::{Efficient, Ops};
use lru::LruCache;
use regex::bytes::Regex;
use sha1::{Digest, Sha1};
use typst::{
    Library, LibraryExt, World,
    diag::{FileError, FileResult, SourceDiagnostic},
    foundations::{Bytes, Datetime, Duration},
    layout::Abs,
    syntax::{FileId, Source},
    text::{Font, FontBook},
    utils::LazyHash,
};
use typst_kit::fonts::FontStore;

use crate::vitem::{VItem, svg::SvgItem};
use ranim_core::Extract;
use ranim_core::traits::{Interpolatable, Resize, resize_xy_by_bounds};
use ranim_core::{
    anchor::{BoundsAnchor, DBounds3, SemanticBounds},
    color,
    components::width::Width,
    core_item::CoreItem,
    glam::{self, DVec2, dvec2, dvec3},
    traits::{
        Alignable, FillColor, Opacity, RotateTransform, Scale, ShiftTransform, StrokeColor,
        StrokeWidth, With,
    },
};

/// Default conversion from one Ranim scene unit to Typst points.
pub const DEFAULT_TYPST_PT_PER_UNIT: f64 = 72.0;

const MIN_TYPST_PT_PER_UNIT: f64 = 1.0e-6;

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
    fn try_get_or_insert(&mut self, typst_str: &str) -> Result<&String, String> {
        let mut sha1 = Sha1::new();
        sha1.update(typst_str.as_bytes());
        let sha1 = sha1.finalize();
        self.inner
            .try_get_or_insert_ref(AsRef::<[u8; 20]>::as_ref(&sha1), || {
                compile_typst_to_svg(typst_str)
            })
    }

    fn get_or_insert(&mut self, typst_str: &str) -> &String {
        self.try_get_or_insert(typst_str)
            .unwrap_or_else(|err| panic!("failed to compile typst source: {err}"))
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

fn fonts() -> &'static FontStore {
    static FONTS: OnceLock<FontStore> = OnceLock::new();
    FONTS.get_or_init(|| {
        let mut fonts = FontStore::new();
        fonts.extend(typst_kit::fonts::embedded());
        fonts.extend(typst_kit::fonts::system());
        fonts
    })
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

/// Compiles typst string to SVG string without panicking on compile errors.
pub fn try_typst_svg(source: &str) -> Result<String, String> {
    typst_lru()
        .lock()
        .unwrap()
        .try_get_or_insert(source)
        .cloned()
}

fn compile_typst_to_svg(source_text: &str) -> Result<String, String> {
    let source = Source::detached(source_text);
    let world = typst_world().lock().unwrap();
    let world = world.with_source(source.clone());
    let document = typst::compile(&world)
        .output
        .map_err(|diagnostics| format_typst_diagnostics(&source, &diagnostics))?;

    let svg = typst_svg::svg_merged(
        &document,
        &typst_svg::SvgOptions::default(),
        Abs::pt(2.0),
    );
    Ok(get_typst_element(&svg))
}

fn format_typst_diagnostics(source: &Source, diagnostics: &[SourceDiagnostic]) -> String {
    diagnostics
        .iter()
        .map(|diagnostic| {
            let location = source
                .range(diagnostic.span)
                .and_then(|range| source.lines().byte_to_line_column(range.start))
                .map(|(line, col)| format!("{}:{}", line + 1, col + 1))
                .unwrap_or_else(|| "unknown".to_owned());

            let mut message = format!(
                "{:?} at {location}: {}",
                diagnostic.severity, diagnostic.message
            );
            for hint in &diagnostic.hints {
                message.push_str("\nhint: ");
                message.push_str(hint);
            }
            message
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn panic_payload(payload: Box<dyn std::any::Any + Send>) -> String {
    if let Some(message) = payload.downcast_ref::<String>() {
        message.clone()
    } else if let Some(message) = payload.downcast_ref::<&'static str>() {
        (*message).to_owned()
    } else {
        "unknown panic".to_owned()
    }
}

fn normalize_pt_per_unit(pt_per_unit: f64) -> f64 {
    if pt_per_unit.is_finite() {
        pt_per_unit.abs().max(MIN_TYPST_PT_PER_UNIT)
    } else {
        DEFAULT_TYPST_PT_PER_UNIT
    }
}

fn ranim_units_to_typst_pt(units: f64, pt_per_unit: f64) -> f64 {
    units.max(1.0e-6) * normalize_pt_per_unit(pt_per_unit)
}

fn wrap_typst_source(source: &str, layout_size: Option<DVec2>, pt_per_unit: f64) -> String {
    if let Some(size) = layout_size {
        let width = ranim_units_to_typst_pt(size.x, pt_per_unit);
        if size.y > 0.0 {
            let height = ranim_units_to_typst_pt(size.y, pt_per_unit);
            format!("#block(width: {width}pt, height: {height}pt)[{source}]")
        } else {
            format!("#block(width: {width}pt)[{source}]")
        }
    } else {
        source.to_owned()
    }
}

fn vitems_from_typst_svg(svg: String, pt_per_unit: f64) -> Result<Vec<VItem>, String> {
    let svg = catch_unwind(AssertUnwindSafe(|| SvgItem::new(svg)))
        .map_err(|payload| format!("failed to parse typst SVG: {}", panic_payload(payload)))?;
    let mut vitems = Vec::<VItem>::from(svg);
    // Typst SVG coordinates are in points. Scale them once, at the boundary,
    // so user source can keep using native Typst units.
    let scale = 1.0 / normalize_pt_per_unit(pt_per_unit);
    vitems.scale(dvec3(scale, scale, scale));
    vitems.as_mut_slice().apply_stroke_func(|widths| {
        widths.iter_mut().for_each(|width| width.0 *= scale as f32);
    });
    Ok(vitems)
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
            book: fonts.book().clone(),
            files: Mutex::new(HashMap::new()),
        }
    }
    #[allow(dead_code)]
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

        Err(FileError::NotFound(id.vpath().get_without_slash().into()))
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
        fonts().font(index)
    }

    fn today(&self, offset: Option<Duration>) -> Option<Datetime> {
        let now = self.now.get_or_init(chrono::Local::now);

        let naive = match offset {
            None => now.naive_local(),
            Some(o) => now.naive_utc() + chrono::Duration::seconds(o.seconds() as i64),
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
/// Plain text sources get character-level alignment for interpolation. More
/// complex Typst sources can still render, but alignment falls back to the
/// generated vector item order.
#[derive(Clone)]
pub struct TypstText {
    source: String,
    chars: String,
    vitems: Vec<VItem>,
    pt_per_unit: f64,
    layout_size: Option<DVec2>,
}

impl TypstText {
    fn _new(str: &str) -> Self {
        Self::new(str)
    }
    /// Create a TypstText with typst string.
    ///
    /// Plain text sources get character-level alignment for interpolation.
    pub fn new(typst_str: &str) -> Self {
        Self::try_new(typst_str).unwrap_or_else(|err| panic!("failed to compile typst text: {err}"))
    }

    /// Create a TypstText with typst string without panicking on compile errors.
    pub fn try_new(typst_str: &str) -> Result<Self, String> {
        Self::try_new_with_pt_per_unit(typst_str, DEFAULT_TYPST_PT_PER_UNIT)
    }

    /// Create a TypstText with typst string and a custom Typst-point to Ranim-unit ratio.
    ///
    /// User source keeps Typst's native unit syntax. Parsed SVG coordinates
    /// are scaled by `1 / pt_per_unit` when converted into Ranim scene units.
    pub fn try_new_with_pt_per_unit(typst_str: &str, pt_per_unit: f64) -> Result<Self, String> {
        let pt_per_unit = normalize_pt_per_unit(pt_per_unit);
        let wrapped = wrap_typst_source(typst_str, None, pt_per_unit);
        let svg = try_typst_svg(&wrapped)?;
        Self::from_svg_with_layout(typst_str, typst_str, svg, None, pt_per_unit)
    }

    /// Create a TypstText with a block layout width in Ranim scene units.
    ///
    /// The source text is wrapped in a Typst block before compilation, but
    /// character alignment still uses the original text.
    pub fn try_new_with_layout_width(typst_str: &str, width: f64) -> Result<Self, String> {
        Self::try_new_with_layout_size(typst_str, dvec2(width, 0.0))
    }

    /// Create a TypstText with a block layout size in Ranim scene units.
    ///
    /// The generated Typst source converts the requested Ranim layout size to
    /// native Typst points for the wrapper block. Parsed SVG coordinates are
    /// then converted back to Ranim units using the same ratio.
    pub fn try_new_with_layout_size(typst_str: &str, layout_size: DVec2) -> Result<Self, String> {
        Self::try_new_with_layout_size_and_pt_per_unit(
            typst_str,
            layout_size,
            DEFAULT_TYPST_PT_PER_UNIT,
        )
    }

    /// Create a TypstText with a block layout size and custom unit ratio.
    pub fn try_new_with_layout_size_and_pt_per_unit(
        typst_str: &str,
        layout_size: DVec2,
        pt_per_unit: f64,
    ) -> Result<Self, String> {
        let pt_per_unit = normalize_pt_per_unit(pt_per_unit);
        let layout_size = dvec2(layout_size.x.max(1.0e-6), layout_size.y.max(0.0));
        let wrapped = wrap_typst_source(typst_str, Some(layout_size), pt_per_unit);
        let svg = try_typst_svg(&wrapped)?;
        Self::from_svg_with_layout(typst_str, typst_str, svg, Some(layout_size), pt_per_unit)
    }

    fn from_svg_with_layout(
        source_text: &str,
        chars_text: &str,
        svg: String,
        layout_size: Option<DVec2>,
        pt_per_unit: f64,
    ) -> Result<Self, String> {
        let pt_per_unit = normalize_pt_per_unit(pt_per_unit);

        let vitems = vitems_from_typst_svg(svg, pt_per_unit)?;
        let chars = typst_alignment_chars(chars_text, vitems.len());
        let layout_size = layout_size.map(|size| {
            let bounds = vitems.semantic_bounds();
            let world_size = bounds.world_size();
            dvec2(size.x.max(1.0e-6), size.y.max(world_size.y))
        });
        Ok(Self {
            source: source_text.to_owned(),
            chars,
            vitems,
            pt_per_unit,
            layout_size,
        })
    }

    fn compile_current_layout(&self) -> Result<Vec<VItem>, String> {
        let source = wrap_typst_source(&self.source, self.layout_size, self.pt_per_unit);
        let svg = try_typst_svg(&source)?;
        let vitems = vitems_from_typst_svg(svg, self.pt_per_unit)?;
        Ok(vitems)
    }

    fn relayout_preserving_semantic_min(&mut self, min: glam::DVec3) -> Result<(), String> {
        let vitems = self.compile_current_layout()?;
        self.chars = typst_alignment_chars(&self.source, vitems.len());
        self.vitems = vitems;
        let new_min = self.semantic_bounds().world_min();
        self.vitems.shift(min - new_min);
        Ok(())
    }

    /// Returns the Typst points represented by one Ranim scene unit.
    pub fn pt_per_unit(&self) -> f64 {
        self.pt_per_unit
    }

    /// Returns the optional layout size in Ranim scene units.
    pub fn layout_size(&self) -> Option<DVec2> {
        self.layout_size
    }

    /// Inline code
    pub fn new_inline_code(code: &str) -> Self {
        let source = format!("`{code}`");
        let wrapped = wrap_typst_source(&source, None, DEFAULT_TYPST_PT_PER_UNIT);
        let svg = typst_svg(&wrapped);
        let chars = code
            .replace(" ", "")
            .replace("\n", "")
            .replace("\r", "")
            .replace("\t", "");

        let vitems = vitems_from_typst_svg(svg, DEFAULT_TYPST_PT_PER_UNIT)
            .expect("failed to parse typst SVG");
        assert_eq!(chars.len(), vitems.len());
        Self {
            source,
            chars,
            vitems,
            pt_per_unit: DEFAULT_TYPST_PT_PER_UNIT,
            layout_size: None,
        }
    }

    /// Multiline code
    pub fn new_multiline_code(code: &str, language: Option<&str>) -> Self {
        let language = language.unwrap_or("");
        let source = format!("```{language}\n{code}```");
        let wrapped = wrap_typst_source(&source, None, DEFAULT_TYPST_PT_PER_UNIT);
        let svg = typst_svg(&wrapped);
        let chars = code
            .replace(" ", "")
            .replace("\n", "")
            .replace("\r", "")
            .replace("\t", "");

        let vitems = vitems_from_typst_svg(svg, DEFAULT_TYPST_PT_PER_UNIT)
            .expect("failed to parse typst SVG");
        assert_eq!(chars.len(), vitems.len());
        Self {
            source,
            chars,
            vitems,
            pt_per_unit: DEFAULT_TYPST_PT_PER_UNIT,
            layout_size: None,
        }
    }
}

fn typst_text_chars(text: &str) -> String {
    text.replace(" ", "")
        .replace("\n", "")
        .replace("\r", "")
        .replace("\t", "")
}

fn typst_alignment_chars(text: &str, vitems_len: usize) -> String {
    let chars = typst_text_chars(text);
    if chars.chars().count() == vitems_len {
        chars
    } else {
        "x".repeat(vitems_len)
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
                let mut vitems_a = self.vitems[last_neq_idx_a..ia].to_vec();
                let mut vitems_b = other.vitems[last_neq_idx_b..ib].to_vec();
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

        self.vitems = vitems_self;
        other.vitems = vitems_other;
    }
}

impl Interpolatable for TypstText {
    fn lerp(&self, target: &Self, t: f64) -> Self {
        let vitems = self
            .vitems
            .iter()
            .zip(&target.vitems)
            .map(|(a, b)| a.lerp(b, t))
            .collect::<Vec<_>>();
        Self {
            source: self.source.clone(),
            chars: self.chars.clone(),
            vitems,
            pt_per_unit: self.pt_per_unit.lerp(&target.pt_per_unit, t),
            layout_size: match (self.layout_size, target.layout_size) {
                (Some(a), Some(b)) => Some(a.lerp(b, t)),
                (Some(a), None) => Some(a),
                (None, Some(b)) => Some(b),
                (None, None) => None,
            },
        }
    }
}

impl From<TypstText> for Vec<VItem> {
    fn from(value: TypstText) -> Self {
        value.vitems
    }
}

impl Extract for TypstText {
    type Target = CoreItem;
    fn extract_into(&self, buf: &mut Vec<Self::Target>) {
        self.vitems.extract_into(buf);
    }
}

impl SemanticBounds for TypstText {
    fn semantic_bounds(&self) -> DBounds3 {
        let bounds = self.vitems.semantic_bounds();
        let min = bounds.world_min();
        let max = bounds.world_max();
        if let Some(size) = self.layout_size {
            DBounds3::new(min, min + dvec3(size.x, size.y, max.z - min.z))
        } else {
            bounds
        }
    }
}

impl ShiftTransform for TypstText {
    fn shift(&mut self, shift: glam::DVec3) -> &mut Self {
        self.vitems.shift(shift);
        self
    }
}

impl RotateTransform for TypstText {
    fn rotate_on_axis(&mut self, axis: glam::DVec3, angle: f64) -> &mut Self {
        self.vitems.rotate_on_axis(axis, angle);
        self
    }
}

impl Scale for TypstText {
    fn scale(&mut self, scale: glam::DVec3) -> &mut Self {
        let bounds = self.semantic_bounds();
        let min = bounds.world_min();
        let size = bounds.size();
        let target_min = min * scale;
        self.layout_size = Some(dvec2(
            (size.x * scale.x.abs()).max(1.0e-6),
            (size.y * scale.y.abs()).max(1.0e-6),
        ));
        if self.relayout_preserving_semantic_min(target_min).is_err() {
            self.vitems.scale(scale);
        }
        self
    }
}

impl Resize<glam::DVec3> for TypstText {
    fn resize_about_bounds(
        &mut self,
        bounds: DBounds3,
        anchor: BoundsAnchor,
        size: glam::DVec3,
    ) -> &mut Self {
        resize_xy_by_bounds(self, bounds, anchor, size.truncate());
        self
    }
}

impl Resize<f64> for TypstText {
    fn resize_about_bounds(
        &mut self,
        bounds: DBounds3,
        anchor: BoundsAnchor,
        size: f64,
    ) -> &mut Self {
        Resize::<glam::DVec3>::resize_about_bounds(self, bounds, anchor, glam::DVec3::splat(size));
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
        self.vitems[0].stroke_color()
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
    let re = Regex::new(r#"\s+(?:viewBox|width|height)="[^"]*""#).unwrap();
    let removed_size = re.replace_all(&removed_bg, b"");

    // println!("{}", String::from_utf8_lossy(&output));
    // println!("{}", String::from_utf8_lossy(&removed_bg));
    String::from_utf8_lossy(&removed_size).to_string()
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

    output.to_string()
}

#[cfg(test)]
mod tests {
    use std::time::Instant;

    use super::*;
    use ranim_core::{
        Extract,
        color::{AlphaColor, Srgb, rgb8},
        traits::{FillColor, Scale, SemanticBounds, StrokeColor},
    };

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
        let svg = typst_svg::svg_merged(&document, &typst_svg::SvgOptions::default(), Abs::pt(2.0));
        println!("{svg}");
        println!("svg output: {:?}", start.elapsed());

        let start = Instant::now();
        let res = get_typst_element(&svg);
        println!("get element: {:?}", start.elapsed());

        println!("{res}");
        // println!("{}", typst_svg!(source))
    }

    #[test]
    fn stroke_color_getter_reads_stroke_not_fill() {
        let mut text = TypstText::try_new("R").expect("typst text should compile");
        text.set_fill_color(rgb8(10, 20, 30));
        text.set_stroke_color(rgb8(200, 150, 100));

        assert_color_near(text.fill_color(), rgb8(10, 20, 30));
        assert_color_near(text.stroke_color(), rgb8(200, 150, 100));
    }

    #[test]
    fn layout_width_is_in_ranim_units() {
        let text =
            TypstText::try_new_with_layout_width("R", 2.0).expect("typst text should compile");

        assert_eq!(text.layout_size().unwrap().x, 2.0);
        assert_eq!(text.pt_per_unit(), DEFAULT_TYPST_PT_PER_UNIT);
        assert!((text.semantic_bounds_size().x - 2.0).abs() < 1.0e-9);
        assert!(text.extract().semantic_bounds_size().x < 2.0);
    }

    #[test]
    fn pt_per_unit_changes_extracted_size_not_layout_frame() {
        let a = TypstText::try_new_with_layout_size_and_pt_per_unit("R", dvec2(2.0, 0.0), 72.0)
            .expect("typst text should compile");
        let b = TypstText::try_new_with_layout_size_and_pt_per_unit("R", dvec2(2.0, 0.0), 36.0)
            .expect("typst text should compile");

        assert!((a.semantic_bounds_size().x - 2.0).abs() < 1.0e-9);
        assert!((b.semantic_bounds_size().x - 2.0).abs() < 1.0e-9);
        assert!(b.extract().semantic_bounds_size().x > a.extract().semantic_bounds_size().x * 1.5);
    }

    #[test]
    fn wrapped_layout_source_uses_native_typst_units() {
        let source = wrap_typst_source("R", Some(dvec2(2.0, 1.0)), 72.0);

        assert_eq!(source, "#block(width: 144pt, height: 72pt)[R]");
        assert!(!source.contains("rapt"));
    }

    #[test]
    fn native_typst_units_are_scaled_to_ranim_units() {
        let item = TypstText::try_new("#rect(width: 2in, height: 1in, fill: red)")
            .expect("typst source should compile");
        let size = item.extract().semantic_bounds_size();

        assert!((size.x - 2.0).abs() < 1.0e-6);
        assert!((size.y - 1.0).abs() < 1.0e-6);
    }

    #[test]
    fn scale_updates_typst_layout_bounds_without_scaling_extracted_primitives() {
        let mut text =
            TypstText::try_new_with_layout_width("R", 2.0).expect("typst text should compile");
        let before_extracted = text.extract().semantic_bounds_size();
        let before_semantic = text.semantic_bounds_size();

        text.scale(dvec3(2.0, 1.5, 1.0));

        let after_extracted = text.extract().semantic_bounds_size();
        let after_semantic = text.semantic_bounds_size();
        assert!((after_semantic.x - before_semantic.x * 2.0).abs() < 1.0e-6);
        assert!((after_semantic.y - before_semantic.y * 1.5).abs() < 1.0e-6);
        assert!((after_extracted.x - before_extracted.x * 2.0).abs() > 1.0e-3);
    }

    fn assert_color_near(actual: AlphaColor<Srgb>, expected: AlphaColor<Srgb>) {
        for (actual, expected) in actual.components.into_iter().zip(expected.components) {
            assert!(
                (actual - expected).abs() <= 1.0e-6,
                "{actual} != {expected}"
            );
        }
    }

    ///
    /// ```
    /// <svg class="typst-doc" viewBox="0 0 11.483999999999998 11" width="11.483999999999998pt" height="11pt" xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" xmlns:h5="http://www.w3.org/1999/xhtml">
    ///    <path class="typst-shape" fill="#ffffff" fill-rule="nonzero" d="M 0 0v 11 h 11.484 v -11 Z "/>
    ///    <g>
    ///        <g class="typst-text" transform="matrix(1 0 0 -1 0 11)">
    ///            <use xlink:href="#gB5279FC30F2C6542A76CE0CDC73F9462" x="0" y="0" fill="#000000" fill-rule="nonzero"/>
    ///            <use xlink:href="#gC5A0A6F735BE491513D9F5FD3BD367ED" x="6.457" y="0" fill="#000000" fill-rule="nonzero"/>
    ///        </g>
    ///    </g>
    /// ```
    /// ```
    /// <svg class="typst-doc" xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" xmlns:h5="http://www.w3.org/1999/xhtml">
    /// <g>
    ///     <g class="typst-text" transform="matrix(1 0 0 -1 0 11)">
    ///         <use xlink:href="#gB5279FC30F2C6542A76CE0CDC73F9462" x="0" y="0" fill="#000000" fill-rule="nonzero"/>
    ///         <use xlink:href="#gC5A0A6F735BE491513D9F5FD3BD367ED" x="6.457" y="0" fill="#000000" fill-rule="nonzero"/>
    ///     </g>
    /// </g>
    /// ```
    #[test]
    fn foo_page() {
        let text = r#"Ra"#;
        let res = compile_typst_code(text);
        println!("{res}");

        let res = typst_svg(text);
        println!("{res}");
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
