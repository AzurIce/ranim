use std::collections::BTreeMap;

use ranim_core::{
    glam::DVec2,
    traits::{FillColor, StrokeColor, StrokeWidth},
};

use super::VItem;

pub use ranim_typst::{CompileOptions, GlyphInfo, TypstError, TypstWarning};

mod text_morph;
pub use text_morph::TypstText;

/// A Typst document converted to Ranim vector items.
#[derive(Debug, Clone)]
pub struct TypstDocument {
    /// Pages in source order.
    pub pages: Vec<TypstPage>,
}

impl TypstDocument {
    /// Flattens all pages into one vector.
    pub fn into_vitems(self) -> Vec<VItem> {
        self.pages
            .into_iter()
            .flat_map(|page| page.vitems)
            .collect()
    }
}

/// One converted Typst page.
#[derive(Debug, Clone)]
pub struct TypstPage {
    /// Page size in Typst points.
    pub size: DVec2,
    /// Vector items in paint order.
    pub vitems: Vec<VItem>,
    /// Typst labels mapped to indices in `vitems`.
    pub groups: BTreeMap<String, Vec<usize>>,
    /// Glyph metadata in text order.
    pub glyphs: Vec<GlyphInfo>,
}

/// The successful result of compiling Typst into Ranim items.
#[derive(Debug, Clone)]
pub struct CompileOutput {
    /// Converted document.
    pub document: TypstDocument,
    /// Typst compiler warning messages.
    pub compiler_warnings: Vec<String>,
    /// Unsupported conversion features encountered.
    pub conversion_warnings: Vec<TypstWarning>,
}

/// Compiles Typst source into Ranim vector items.
pub fn compile(source: &str) -> Result<CompileOutput, TypstError> {
    compile_with_options(source, CompileOptions::default())
}

/// Compiles Typst source with explicit options.
pub fn compile_with_options(
    source: &str,
    options: CompileOptions,
) -> Result<CompileOutput, TypstError> {
    let output = ranim_typst::compile_with_options(source, options)?;
    let pages = output
        .document
        .pages
        .into_iter()
        .map(|page| TypstPage {
            size: page.size,
            vitems: page.paths.into_iter().map(path_to_vitem).collect(),
            groups: page.groups,
            glyphs: page.glyphs,
        })
        .collect();
    Ok(CompileOutput {
        document: TypstDocument { pages },
        compiler_warnings: output.compiler_warnings,
        conversion_warnings: output.conversion_warnings,
    })
}

/// Compiles Typst source and returns only flattened Ranim vector items.
pub fn compile_vitems(source: &str) -> Result<Vec<VItem>, TypstError> {
    Ok(compile(source)?.document.into_vitems())
}

fn path_to_vitem(path: ranim_typst::TypstPath) -> VItem {
    let mut item = VItem::from_vpoints(path.points);
    match path.fill {
        Some(fill) => {
            item.set_fill_color(fill);
        }
        None => {
            item.set_fill_opacity(0.0);
        }
    }
    match path.stroke {
        Some(stroke) => {
            item.set_stroke_color(stroke.color);
            item.set_stroke_width(stroke.width);
        }
        None => {
            item.set_stroke_opacity(0.0);
            item.set_stroke_width(0.0);
        }
    }
    item
}
