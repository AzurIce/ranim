//! Native compilation of Typst documents into vector path data.
//!
//! ```
//! let output = ranim_typst::compile("$ x^2 + y^2 = 1 $").unwrap();
//! assert!(!output.document.pages[0].paths.is_empty());
//! ```

#![warn(missing_docs)]

mod collector;
mod world;

use std::{
    collections::BTreeMap,
    num::NonZeroUsize,
    ops::Range,
    sync::{Mutex, OnceLock},
};

use lru::LruCache;
use ranim_core::{
    color::{AlphaColor, Srgb},
    glam::{DVec2, DVec3},
};
use thiserror::Error;
use typst::diag::Warned;
use typst_layout::PagedDocument;

/// A compiled Typst document represented as vector paths.
#[derive(Debug, Clone)]
pub struct TypstDocument {
    /// The document pages in source order.
    pub pages: Vec<TypstPage>,
}

impl TypstDocument {
    /// Consumes the document and flattens all pages into one vector.
    pub fn into_paths(self) -> Vec<TypstPath> {
        self.pages.into_iter().flat_map(|page| page.paths).collect()
    }
}

impl From<TypstDocument> for Vec<TypstPath> {
    fn from(document: TypstDocument) -> Self {
        document.into_paths()
    }
}

/// One compiled Typst page.
#[derive(Debug, Clone)]
pub struct TypstPage {
    /// The Typst page size in points.
    pub size: DVec2,
    /// Vector paths in paint order, centered around the origin.
    pub paths: Vec<TypstPath>,
    /// Typst labels mapped to indices in `paths`.
    pub groups: BTreeMap<String, Vec<usize>>,
    /// Glyph metadata in text order.
    pub glyphs: Vec<GlyphInfo>,
}

impl From<TypstPage> for Vec<TypstPath> {
    fn from(page: TypstPage) -> Self {
        page.paths
    }
}

/// A quadratic Bézier path emitted by Typst.
#[derive(Debug, Clone, PartialEq)]
pub struct TypstPath {
    /// Ranim-style quadratic Bézier points.
    pub points: Vec<DVec3>,
    /// Solid fill color, when present.
    pub fill: Option<AlphaColor<Srgb>>,
    /// Solid stroke, when present.
    pub stroke: Option<TypstStroke>,
}

/// A solid path stroke emitted by Typst.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TypstStroke {
    /// Stroke color.
    pub color: AlphaColor<Srgb>,
    /// Stroke width in Typst points after transforms.
    pub width: f32,
}

/// Metadata for one shaped Typst glyph.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GlyphInfo {
    /// Index of the glyph path in its page, or `None` for whitespace and unsupported glyphs.
    pub item_index: Option<usize>,
    /// The glyph's text slice.
    pub text: String,
    /// Byte range inside the containing Typst text run.
    pub text_range: Range<usize>,
}

/// A non-fatal limitation encountered while converting a Typst document.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TypstWarning {
    /// Raster and vector image items are not representable as `VItem`.
    ImageUnsupported,
    /// Bitmap, SVG, and layered color font glyphs are not yet converted.
    ColorGlyphUnsupported,
    /// Clip paths are not applied.
    ClipPathUnsupported,
    /// Gradient paints are replaced with white.
    GradientUnsupported,
    /// Tiling paints are replaced with white.
    TilingUnsupported,
    /// Ranim's `VItem` does not expose the even-odd fill rule.
    EvenOddFillUnsupported,
}

/// The successful result of compiling and collecting a Typst document.
#[derive(Debug, Clone)]
pub struct CompileOutput {
    /// The converted document.
    pub document: TypstDocument,
    /// Typst compiler warnings rendered as messages.
    pub compiler_warnings: Vec<String>,
    /// Non-fatal conversion limitations.
    pub conversion_warnings: Vec<TypstWarning>,
}

/// Options controlling how Typst pages are converted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CompileOptions {
    /// Include the page background as the first path on each page.
    pub include_page_fill: bool,
    /// Center each page's collected content around the Ranim origin.
    pub center_content: bool,
}

impl Default for CompileOptions {
    fn default() -> Self {
        Self {
            include_page_fill: false,
            center_content: true,
        }
    }
}

/// An error produced while compiling Typst source.
#[derive(Debug, Error)]
pub enum TypstError {
    /// Typst rejected the source.
    #[error("Typst compilation failed: {0}")]
    Compile(String),
}

/// Compiles Typst source into vector paths.
pub fn compile(source: &str) -> Result<CompileOutput, TypstError> {
    compile_with_options(source, CompileOptions::default())
}

/// Compiles Typst source with explicit conversion options.
pub fn compile_with_options(
    source: &str,
    options: CompileOptions,
) -> Result<CompileOutput, TypstError> {
    let key = (source.to_owned(), options);
    if let Some(output) = compile_cache().lock().unwrap().get(&key).cloned() {
        return Ok(output);
    }

    let world = world::SingleSourceWorld::new(source);
    let Warned { output, warnings } = typst::compile::<PagedDocument>(&world);
    let document = output.map_err(|diagnostics| {
        TypstError::Compile(
            diagnostics
                .iter()
                .map(|diagnostic| diagnostic.message.as_str())
                .collect::<Vec<_>>()
                .join("\n"),
        )
    })?;
    let compiler_warnings = warnings
        .iter()
        .map(|diagnostic| diagnostic.message.to_string())
        .collect();
    let mut conversion_warnings = Vec::new();
    let document = collector::collect(&document, options, &mut conversion_warnings);

    let output = CompileOutput {
        document,
        compiler_warnings,
        conversion_warnings,
    };
    compile_cache().lock().unwrap().put(key, output.clone());
    Ok(output)
}

fn compile_cache() -> &'static Mutex<LruCache<(String, CompileOptions), CompileOutput>> {
    static CACHE: OnceLock<Mutex<LruCache<(String, CompileOptions), CompileOutput>>> =
        OnceLock::new();
    CACHE.get_or_init(|| {
        Mutex::new(LruCache::new(
            NonZeroUsize::new(256).expect("cache capacity is non-zero"),
        ))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const PAGE: &str = "#set page(width: auto, height: auto, margin: 0pt)\n";

    #[test]
    fn collects_text_glyphs() {
        let output = compile(&format!("{PAGE}Hello")).unwrap();
        let page = &output.document.pages[0];
        assert!(!page.paths.is_empty());
        assert_eq!(page.glyphs.len(), 5);
        assert_eq!(page.glyphs[0].text, "H");
    }

    #[test]
    fn collects_shapes_and_labels() {
        let source = format!("{PAGE}#box[#rect(width: 20pt, height: 10pt, fill: red)] <target>");
        let output = compile(&source).unwrap();
        let page = &output.document.pages[0];
        assert_eq!(page.paths.len(), 1);
        assert!(page.groups.contains_key("target"));
        assert!(page.paths[0].fill.unwrap().components[0] > 0.9);
    }

    #[test]
    fn applies_typst_transforms() {
        let source = format!("{PAGE}#rotate(30deg)[#rect(width: 20pt, height: 10pt, fill: red)]");
        let output = compile(&source).unwrap();
        let points = &output.document.pages[0].paths[0].points;
        assert!(points.iter().all(|point| point.is_finite()));
        assert!(points.iter().any(|point| point.x.abs() > 1.0));
        assert!(points.iter().any(|point| point.y.abs() > 1.0));
    }

    #[test]
    fn reports_unsupported_gradient() {
        let source =
            format!("{PAGE}#rect(width: 20pt, height: 10pt, fill: gradient.linear(red, blue))");
        let output = compile(&source).unwrap();
        assert_eq!(
            output.conversion_warnings,
            vec![TypstWarning::GradientUnsupported]
        );
    }

    #[test]
    fn reports_compile_errors() {
        assert!(compile("#let =").is_err());
    }
}
