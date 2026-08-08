use std::{
    collections::HashMap,
    sync::Mutex,
};

use ranim_core::{
    Extract,
    color::{AlphaColor, Srgb},
    core_item::CoreItem,
    glam::{DAffine3, DMat3, DVec3},
    traits::{
        Aabb, Discard, FillColor, Locate, PointsFunc, RotateTransform, ScaleTransform,
        ShiftTransform, StrokeColor, StrokeWidth, With,
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
#[derive(bevy_ecs::component::Component, Debug)]
pub struct TextItem {
    /// Origin
    origin: DVec3,
    /// Basis
    basis: (DVec3, DVec3),
    /// Text content
    text: String,
    /// Font info
    font: TextFont,
    /// Fill color
    fill_rgbas: AlphaColor<Srgb>,
    /// Stroke color
    stroke_rgbas: AlphaColor<Srgb>,
    /// Stroke width
    stroke_width: f32,
    /// Cached items (Mutex for Send+Sync; the cache itself moves to
    /// `Extracted<T>` in M2 stage 2).
    items: Mutex<Option<Vec<VItem>>>,
    /// cached text inline size
    inline_length_em: Mutex<Option<f64>>,
}

impl Clone for TextItem {
    fn clone(&self) -> Self {
        Self {
            origin: self.origin,
            basis: self.basis,
            text: self.text.clone(),
            font: self.font.clone(),
            fill_rgbas: self.fill_rgbas,
            stroke_rgbas: self.stroke_rgbas,
            stroke_width: self.stroke_width,
            items: Mutex::new(self.items.lock().unwrap().clone()),
            inline_length_em: Mutex::new(*self.inline_length_em.lock().unwrap()),
        }
    }
}

impl Locate<TextItem> for Origin {
    fn locate(&self, target: &TextItem) -> DVec3 {
        target.origin
    }
}

impl TextItem {
    /// Create a new text item
    pub fn new(text: impl Into<String>, em_size: f64) -> Self {
        Self {
            origin: DVec3::ZERO,
            basis: (DVec3::X * em_size, DVec3::Y * em_size),
            text: text.into(),
            font: TextFont::default(),
            fill_rgbas: AlphaColor::WHITE,
            stroke_rgbas: AlphaColor::WHITE,
            stroke_width: 0.0,
            items: Mutex::default(),
            inline_length_em: Mutex::default(),
        }
    }

    /// Set font
    pub fn with_font(mut self, font: TextFont) -> Self {
        self.font = font;
        *self.items.lock().unwrap() = None;
        self
    }

    /// Get font
    pub fn font(&self) -> &TextFont {
        &self.font
    }

    /// Get basis
    pub fn basis(&self) -> (DVec3, DVec3) {
        self.basis
    }

    /// Get text
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Get inline length in em units
    pub fn inline_length_em(&self) -> f64 {
        drop(self.items()); // ensure items are generated
        self.inline_length_em.lock().unwrap().unwrap()
    }

    /// Returns the text outline box starting from baseline origin to the width of last character and em height.
    pub fn text_box(&self) -> Parallelogram {
        let (u, v) = self.basis;
        Parallelogram::new(self.origin, (u * self.inline_length_em(), v))
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

        let &Self {
            basis: (u, v),
            origin,
            fill_rgbas,
            stroke_rgbas,
            stroke_width,
            ..
        } = self;
        let [min, max] = baseline_em_box;
        let h = max.y - min.y;
        *self.inline_length_em.lock().unwrap() = Some((max.x - min.x) / h);
        let mat = DAffine3::from_mat3_translation(DMat3::from_cols(u, v, DVec3::ZERO), origin);
        texts.with(|x| {
            x.shift(-min)
                .scale(DVec3::splat(1. / h)) // Make height = 1.0
                .apply_point_func(|p| *p = mat.transform_point3(*p))
                .set_fill_color(fill_rgbas)
                .set_stroke_color(stroke_rgbas)
                .set_stroke_width(stroke_width)
                .discard()
        })
    }

    fn items(&self) -> std::sync::MutexGuard<'_, Option<Vec<VItem>>> {
        if self.items.lock().unwrap().is_none() {
            *self.items.lock().unwrap() = Some(self.generate_items());
        }
        self.items.lock().unwrap()
    }

    fn transform_items(&self, transformation: impl FnOnce(&mut Vec<VItem>)) {
        if let Some(v) = self.items.lock().unwrap().as_mut() {
            transformation(v);
        }
    }
}

impl Aabb for TextItem {
    fn aabb(&self) -> [DVec3; 2] {
        self.items().as_ref().unwrap().aabb()
    }
}

impl ShiftTransform for TextItem {
    fn shift(&mut self, offset: DVec3) -> &mut Self {
        self.origin += offset;
        self.transform_items(|item| item.shift(offset).discard());
        self
    }
}

impl RotateTransform for TextItem {
    fn rotate_on_axis(&mut self, axis: DVec3, angle: f64) -> &mut Self {
        self.origin.rotate_on_axis(axis, angle);
        self.basis.0.rotate_on_axis(axis, angle);
        self.basis.1.rotate_on_axis(axis, angle);
        self.transform_items(|item| item.rotate_on_axis(axis, angle).discard());
        self
    }
}

impl ScaleTransform for TextItem {
    fn scale(&mut self, scale: DVec3) -> &mut Self {
        self.origin.scale(scale).discard();
        self.basis.0 *= scale;
        self.basis.1 *= scale;
        self.transform_items(|item| item.scale(scale).discard());
        self
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
        item.items().as_ref().unwrap().clone()
    }
}

impl Extract for TextItem {
    type Target = CoreItem;

    fn extract_into(&self, buf: &mut Vec<Self::Target>) {
        self.items().as_ref().unwrap().extract_into(buf);
    }
}

#[cfg(test)]
mod tests {
    use assert_float_eq::assert_float_absolute_eq;

    use super::*;

    #[test]
    fn test_text_item() {
        let item = TextItem::new("Hello, world!", 0.25);
        assert_float_absolute_eq!(item.basis.0.length(), 0.25, 1e-10);
        assert_float_absolute_eq!(item.origin.distance(DVec3::ZERO), 0.0, 1e-10);
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
