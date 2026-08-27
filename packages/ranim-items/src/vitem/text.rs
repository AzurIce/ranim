use std::{
    cell::{Cell, Ref, RefCell},
    collections::HashMap,
};

use ranim_core::{
    Extract,
    color::{AlphaColor, Srgb},
    core_item::CoreItem,
    glam::{DMat3, DVec3},
    traits::{
        Aabb, Discard, FillColor, Locate, PointsFunc, ScaleTransform, ShiftTransform, StrokeColor,
        StrokeWidth, With,
    },
};
use typst::foundations::Repr;

use crate::vitem::{
    VItem,
    geometry::{Parallelogram, anchor::Origin},
    svg::SvgItem,
    typst::typst_svg,
};
pub use typst::text::{FontStretch, FontStyle, FontVariant, FontWeight};

/// Font information for text items
#[derive(Clone, Debug)]
pub struct TextFont {
    families: Vec<String>,
    variant: FontVariant,
    features: HashMap<String, u32>,
}

impl TextFont {
    /// Create a new font
    pub fn new(families: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            families: families.into_iter().map(|v| v.into()).collect(),
            variant: Default::default(),
            features: Default::default(),
        }
    }
    /// Set font weight
    pub fn with_weight(mut self, weight: FontWeight) -> Self {
        self.variant.weight = weight;
        self
    }
    /// Set font style
    pub fn with_style(mut self, style: FontStyle) -> Self {
        self.variant.style = style;
        self
    }
    /// Set font stretch
    pub fn with_stretch(mut self, stretch: FontStretch) -> Self {
        self.variant.stretch = stretch;
        self
    }
    /// Add OTF features
    pub fn with_features(
        mut self,
        features: impl IntoIterator<Item = (impl Into<String>, u32)>,
    ) -> Self {
        self.features
            .extend(features.into_iter().map(|(k, v)| (k.into(), v)));
        self
    }
}

impl Default for TextFont {
    fn default() -> Self {
        Self::new(["New Computer Modern", "Libertinus Serif"])
    }
}

/// Simple single-line text item
#[derive(Clone, Debug)]
pub struct TextItem {
    /// Text content
    text: String,
    /// Intrinsic em size in local coordinates.
    em_size: f64,
    /// Font info
    font: TextFont,
    /// Fill color
    fill_rgbas: AlphaColor<Srgb>,
    /// Stroke color
    stroke_rgbas: AlphaColor<Srgb>,
    /// Stroke width
    stroke_width: f32,
    /// Cached items
    items: RefCell<Option<Vec<VItem>>>,
    /// cached text inline size
    inline_length_em: Cell<Option<f64>>,
}

impl Locate<TextItem> for Origin {
    fn locate(&self, _target: &TextItem) -> DVec3 {
        DVec3::ZERO
    }
}

impl TextItem {
    /// Create a new text item
    pub fn new(text: impl Into<String>, em_size: f64) -> Self {
        Self {
            text: text.into(),
            em_size,
            font: TextFont::default(),
            fill_rgbas: AlphaColor::WHITE,
            stroke_rgbas: AlphaColor::WHITE,
            stroke_width: 0.0,
            items: RefCell::default(),
            inline_length_em: Cell::default(),
        }
    }

    /// Set font
    pub fn with_font(mut self, font: TextFont) -> Self {
        self.font = font;
        self.items.take();
        self
    }

    /// Get font
    pub fn font(&self) -> &TextFont {
        &self.font
    }

    /// Get intrinsic em size.
    pub fn em_size(&self) -> f64 {
        self.em_size
    }

    /// Get the canonical local basis.
    pub fn basis(&self) -> (DVec3, DVec3) {
        (DVec3::X * self.em_size, DVec3::Y * self.em_size)
    }

    /// Get text
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Get inline length in em units
    pub fn inline_length_em(&self) -> f64 {
        let _ = self.items(); // ensure items are generated
        self.inline_length_em.get().unwrap()
    }

    /// Returns the canonical local text outline box from the baseline origin.
    ///
    /// Positioning and orientation are supplied by `Transformed<TextItem, G>`.
    pub fn text_box(&self) -> Parallelogram {
        let (u, v) = self.basis();
        Parallelogram::from_origin_and_axes(DVec3::ZERO, (u * self.inline_length_em(), v))
    }

    fn generate_items(&self) -> Vec<VItem> {
        let font = &self.font;
        let text = self.text.as_str();

        // font families
        let mut families = String::new();
        for family in font.families.iter() {
            families.push('"');
            families.push_str(family);
            families.push_str("\", ");
        }

        // font weight as an integer between 100 and 900
        let weight = font.variant.weight.to_number();

        // font style
        let style = {
            use FontStyle::*;
            match font.variant.style {
                Normal => "normal",
                Italic => "italic",
                Oblique => "oblique",
            }
        };

        // font stretch
        let stretch = font.variant.stretch.to_ratio().repr();

        // OTF features
        let features = if font.features.is_empty() {
            ":".to_string()
        } else {
            let mut features = String::new();
            for (tag, value) in font.features.iter() {
                features.push('"');
                features.push_str(tag);
                features.push_str("\": ");
                features.push_str(value.to_string().as_str());
                features.push_str(", ");
            }
            features
        };

        let svg_src = typst_svg(
            format!(
                r#"#set text(
    top-edge: 1em,
    font: ({families}),
    weight: {weight},
    style: "{style}",
    stretch: {stretch},
    features: ({features}),
)
#set page(
    width: auto,
    height: auto,
    margin: 0pt,
    background: rect(width: 100%, height: 100%),
)

{text}
"#
            )
            .as_str(),
        );

        let mut items = Vec::<VItem>::from(SvgItem::new(svg_src));
        let baseline_em_box = items[0].aabb();
        let texts = items.split_off(1);

        let (u, v) = self.basis();
        let fill_rgbas = self.fill_rgbas;
        let stroke_rgbas = self.stroke_rgbas;
        let stroke_width = self.stroke_width;
        let [min, max] = baseline_em_box;
        let h = max.y - min.y;
        self.inline_length_em.set(Some((max.x - min.x) / h));
        let mat = DMat3::from_cols(u, v, DVec3::ZERO);
        texts.with(|x| {
            x.shift(-min)
                .scale(DVec3::splat(1. / h)) // Make height = 1.0
                .apply_point_func(|p| *p = mat * *p)
                .set_fill_color(fill_rgbas)
                .set_stroke_color(stroke_rgbas)
                .set_stroke_width(stroke_width)
                .discard()
        })
    }

    fn items(&self) -> Ref<'_, Vec<VItem>> {
        if self.items.borrow().is_none() {
            let items = self.generate_items();
            self.items.replace(Some(items));
        }
        Ref::map(self.items.borrow(), |v| v.as_ref().unwrap())
    }

    fn transform_items(&self, transformation: impl FnOnce(&mut Vec<VItem>)) {
        if let Some(v) = self.items.borrow_mut().as_mut() {
            transformation(v);
        }
    }
}

impl Aabb for TextItem {
    fn aabb(&self) -> [DVec3; 2] {
        self.items().aabb()
    }
}

impl FillColor for TextItem {
    fn fill_color(&self) -> AlphaColor<Srgb> {
        self.fill_rgbas
    }

    fn set_fill_color(&mut self, color: AlphaColor<Srgb>) -> &mut Self {
        self.fill_rgbas = color;
        self.transform_items(|item| item.set_fill_color(color).discard());
        self
    }

    fn set_fill_opacity(&mut self, opacity: f32) -> &mut Self {
        self.fill_rgbas = self.fill_rgbas.with_alpha(opacity);
        self.transform_items(|item| item.set_fill_opacity(opacity).discard());
        self
    }
}

impl StrokeColor for TextItem {
    fn stroke_color(&self) -> AlphaColor<Srgb> {
        self.stroke_rgbas
    }

    fn set_stroke_color(&mut self, color: AlphaColor<Srgb>) -> &mut Self {
        self.stroke_rgbas = color;
        self.transform_items(|item| item.set_stroke_color(color).discard());
        self
    }

    fn set_stroke_opacity(&mut self, opacity: f32) -> &mut Self {
        self.stroke_rgbas = self.stroke_rgbas.with_alpha(opacity);
        self.transform_items(|item| item.set_stroke_opacity(opacity).discard());
        self
    }
}

impl From<TextItem> for Vec<VItem> {
    fn from(item: TextItem) -> Self {
        item.items().clone()
    }
}

impl Extract for TextItem {
    type Target = CoreItem;

    fn extract_into(&self, buf: &mut Vec<Self::Target>) {
        self.items().extract_into(buf);
    }
}

#[cfg(test)]
mod tests {
    use assert_float_eq::assert_float_absolute_eq;

    use super::*;

    #[test]
    fn test_text_item() {
        let item = TextItem::new("Hello, world!", 0.25);
        assert_float_absolute_eq!(item.basis().0.length(), 0.25, 1e-10);
        assert_float_absolute_eq!(Origin.locate(&item).distance(DVec3::ZERO), 0.0, 1e-10);
    }

    #[test]
    fn test_font() {
        let font = TextFont::new(["Arial", "Helvetica"])
            .with_weight(FontWeight::BOLD)
            .with_style(FontStyle::Italic)
            .with_stretch(FontStretch::CONDENSED)
            .with_features([("liga", 1), ("dlig", 1)]);
        dbg!(&font);
    }
}
