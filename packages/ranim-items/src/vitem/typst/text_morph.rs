use ranim_core::{
    Extract,
    anchor::{DBounds3, SemanticBounds},
    color::{self, palette::css},
    components::{VecResizeTrait, width::Width},
    core_item::CoreItem,
    glam,
    traits::{
        Alignable, FillColor, Interpolatable, Opacity, RotateTransform, Scale, ShiftTransform,
        StrokeColor, StrokeWidth, With,
    },
    utils::resize_preserving_order_with_repeated_indices,
};

use crate::vitem::{
    VItem,
    typst::{TypstError, compile},
};

/// Text-like Typst content with glyph-aware morph alignment.
#[derive(Debug, Clone)]
pub struct TypstText {
    keys: Vec<String>,
    vitems: Vec<VItem>,
}

impl TypstText {
    /// Compiles text-like Typst source.
    ///
    /// Panics when Typst rejects the source. Use [`TypstText::try_new`] for a
    /// fallible constructor.
    pub fn new(source: &str) -> Self {
        Self::try_new(source).expect("failed to compile TypstText source")
    }

    /// Compiles text-like Typst source.
    pub fn try_new(source: &str) -> Result<Self, TypstError> {
        let output = compile(source)?;
        let mut keys = Vec::new();
        let mut vitems = Vec::new();

        for page in output.document.pages {
            let mut page_keys = vec![String::from("\0shape"); page.vitems.len()];
            for glyph in page.glyphs {
                if let Some(index) = glyph.item_index {
                    page_keys[index] = glyph.text;
                }
            }
            keys.extend(page_keys);
            vitems.extend(page.vitems);
        }
        Ok(Self { keys, vitems })
    }

    /// Compiles source inside a fixed layout block measured in Ranim scene units.
    pub fn try_new_with_layout_size(
        source: &str,
        layout_size: glam::DVec2,
    ) -> Result<Self, String> {
        const TYPST_POINTS_PER_UNIT: f64 = 72.0;

        let width = layout_size.x.max(1.0e-6) * TYPST_POINTS_PER_UNIT;
        let height = layout_size.y.max(1.0e-6) * TYPST_POINTS_PER_UNIT;
        let wrapped = format!("#block(width: {width}pt, height: {height}pt)[{source}]");
        let mut text = Self::try_new(&wrapped).map_err(|error| error.to_string())?;
        text.scale(glam::DVec3::splat(1.0 / TYPST_POINTS_PER_UNIT));
        Ok(text)
    }

    /// Compiles inline raw-code content.
    pub fn new_inline_code(code: &str) -> Self {
        Self::new(&format!("`{code}`"))
    }

    /// Compiles a fenced code block.
    pub fn new_multiline_code(code: &str, language: Option<&str>) -> Self {
        Self::new(&format!("```{}\n{code}```", language.unwrap_or_default()))
    }

    fn align_block(
        left: &[VItem],
        right: &[VItem],
        block: usize,
        left_out: &mut Vec<VItem>,
        right_out: &mut Vec<VItem>,
        keys_out: &mut Vec<String>,
    ) {
        let mut left = left.to_vec();
        let mut right = right.to_vec();
        if left.is_empty() {
            left.extend(right.iter().cloned().map(|item| {
                item.with(|item| {
                    item.shrink();
                })
            }));
        }
        if right.is_empty() {
            right.extend(left.iter().cloned().map(|item| {
                item.with(|item| {
                    item.shrink();
                })
            }));
        }
        if left.is_empty() {
            return;
        }
        let len = left.len().max(right.len());
        resize_items(&mut left, len);
        resize_items(&mut right, len);
        left.iter_mut()
            .zip(&mut right)
            .for_each(|(left, right)| align_vitems(left, right));
        left_out.extend(left);
        right_out.extend(right);
        keys_out.extend((0..len).map(|index| format!("\0aligned:{block}:{index}")));
    }
}

impl Alignable for TypstText {
    fn is_aligned(&self, other: &Self) -> bool {
        self.vitems.len() == other.vitems.len()
            && self
                .vitems
                .iter()
                .zip(&other.vitems)
                .all(|(left, right)| left.is_aligned(right))
    }

    fn align_with(&mut self, other: &mut Self) {
        let matches = lcs_matches(&self.keys, &other.keys);
        let mut left_out = Vec::new();
        let mut right_out = Vec::new();
        let mut keys_out = Vec::new();
        let mut left_start = 0;
        let mut right_start = 0;

        for (block, (left_match, right_match)) in matches.into_iter().enumerate() {
            Self::align_block(
                &self.vitems[left_start..left_match],
                &other.vitems[right_start..right_match],
                block,
                &mut left_out,
                &mut right_out,
                &mut keys_out,
            );
            let mut left = self.vitems[left_match].clone();
            let mut right = other.vitems[right_match].clone();
            align_vitems(&mut left, &mut right);
            left_out.push(left);
            right_out.push(right);
            keys_out.push(self.keys[left_match].clone());
            left_start = left_match + 1;
            right_start = right_match + 1;
        }

        Self::align_block(
            &self.vitems[left_start..],
            &other.vitems[right_start..],
            keys_out.len(),
            &mut left_out,
            &mut right_out,
            &mut keys_out,
        );
        self.keys = keys_out.clone();
        other.keys = keys_out;
        self.vitems = left_out;
        other.vitems = right_out;
    }
}

impl Interpolatable for TypstText {
    fn lerp(&self, target: &Self, t: f64) -> Self {
        Self {
            keys: self.keys.clone(),
            vitems: self
                .vitems
                .iter()
                .zip(&target.vitems)
                .map(|(left, right)| left.lerp(right, t))
                .collect(),
        }
    }
}

impl From<TypstText> for Vec<VItem> {
    fn from(text: TypstText) -> Self {
        text.vitems
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
        self.vitems.semantic_bounds()
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
        self.vitems.scale(scale);
        self
    }
}

impl FillColor for TypstText {
    fn fill_color(&self) -> color::AlphaColor<color::Srgb> {
        self.vitems
            .first()
            .map(FillColor::fill_color)
            .unwrap_or(css::WHITE)
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
        self.vitems
            .first()
            .map(StrokeColor::stroke_color)
            .unwrap_or(css::WHITE)
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
        self.vitems.iter_mut().for_each(|item| {
            item.apply_stroke_func(&f);
        });
        self
    }
    fn set_stroke_width(&mut self, width: f32) -> &mut Self {
        self.vitems.set_stroke_width(width);
        self
    }
}

fn lcs_matches(left: &[String], right: &[String]) -> Vec<(usize, usize)> {
    let mut lengths = vec![vec![0usize; right.len() + 1]; left.len() + 1];
    for left_index in (0..left.len()).rev() {
        for right_index in (0..right.len()).rev() {
            lengths[left_index][right_index] = if left[left_index] == right[right_index] {
                lengths[left_index + 1][right_index + 1] + 1
            } else {
                lengths[left_index + 1][right_index].max(lengths[left_index][right_index + 1])
            };
        }
    }
    let mut matches = Vec::new();
    let (mut left_index, mut right_index) = (0, 0);
    while left_index < left.len() && right_index < right.len() {
        if left[left_index] == right[right_index] {
            matches.push((left_index, right_index));
            left_index += 1;
            right_index += 1;
        } else if lengths[left_index + 1][right_index] >= lengths[left_index][right_index + 1] {
            left_index += 1;
        } else {
            right_index += 1;
        }
    }
    matches
}

fn resize_items(items: &mut Vec<VItem>, len: usize) {
    if items.len() == len {
        return;
    }
    let (mut resized, repeated) = resize_preserving_order_with_repeated_indices(items, len);
    for index in repeated {
        resized[index].set_opacity(0.0);
    }
    *items = resized;
}

fn align_vitems(left: &mut VItem, right: &mut VItem) {
    let supports_subpath_alignment = |item: &VItem| {
        item.vpoints
            .get_subpaths()
            .iter()
            .all(|subpath| subpath.len() >= 3)
    };
    if supports_subpath_alignment(left) && supports_subpath_alignment(right) {
        left.align_with(right);
        return;
    }

    let points_len = left.vpoints.len().max(right.vpoints.len()).max(3);
    left.vpoints.resize_preserving_order(points_len);
    right.vpoints.resize_preserving_order(points_len);
    let components_len = points_len.div_ceil(2);
    left.fill_rgbas.resize_preserving_order(components_len);
    right.fill_rgbas.resize_preserving_order(components_len);
    left.stroke_rgbas.resize_preserving_order(components_len);
    right.stroke_rgbas.resize_preserving_order(components_len);
    left.stroke_widths.resize_preserving_order(components_len);
    right.stroke_widths.resize_preserving_order(components_len);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aligns_unicode_and_ligatures_by_glyph_cluster() {
        let mut left = TypstText::new("office 世界");
        let mut right = TypstText::new("official 世界!");
        left.align_with(&mut right);
        assert!(left.is_aligned(&right));
        assert_eq!(left.keys, right.keys);
    }
}
