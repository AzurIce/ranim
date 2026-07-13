use std::{
    cell::{Cell, Ref, RefCell},
    collections::HashMap,
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
    typst::{CompileOptions, compile_with_options},
};

pub use typst::text::{FontStretch, FontStyle, FontVariant, FontWeight};

/// Font information for text items.
#[derive(Clone, Debug)]
pub struct TextFont {
    families: Vec<String>,
    variant: FontVariant,
    features: HashMap<String, u32>,
}

impl TextFont {
    /// Creates a font configuration.
    pub fn new(families: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            families: families.into_iter().map(Into::into).collect(),
            variant: FontVariant::default(),
            features: HashMap::new(),
        }
    }

    /// Sets the font weight.
    pub fn with_weight(mut self, weight: FontWeight) -> Self {
        self.variant.weight = weight;
        self
    }

    /// Sets the font style.
    pub fn with_style(mut self, style: FontStyle) -> Self {
        self.variant.style = style;
        self
    }

    /// Sets the font stretch.
    pub fn with_stretch(mut self, stretch: FontStretch) -> Self {
        self.variant.stretch = stretch;
        self
    }

    /// Adds OpenType feature values.
    pub fn with_features(
        mut self,
        features: impl IntoIterator<Item = (impl Into<String>, u32)>,
    ) -> Self {
        self.features
            .extend(features.into_iter().map(|(key, value)| (key.into(), value)));
        self
    }
}

impl Default for TextFont {
    fn default() -> Self {
        Self::new(["New Computer Modern", "Libertinus Serif"])
    }
}

/// A simple single-line text item shaped and outlined by Typst.
#[derive(Clone, Debug)]
pub struct TextItem {
    origin: DVec3,
    basis: (DVec3, DVec3),
    text: String,
    font: TextFont,
    fill_rgbas: AlphaColor<Srgb>,
    stroke_rgbas: AlphaColor<Srgb>,
    stroke_width: f32,
    items: RefCell<Option<Vec<VItem>>>,
    inline_length_em: Cell<Option<f64>>,
}

impl Locate<TextItem> for Origin {
    fn locate(&self, target: &TextItem) -> DVec3 {
        target.origin
    }
}

impl TextItem {
    /// Creates a text item with the given em size.
    pub fn new(text: impl Into<String>, em_size: f64) -> Self {
        Self {
            origin: DVec3::ZERO,
            basis: (DVec3::X * em_size, DVec3::Y * em_size),
            text: text.into(),
            font: TextFont::default(),
            fill_rgbas: AlphaColor::WHITE,
            stroke_rgbas: AlphaColor::WHITE,
            stroke_width: 0.0,
            items: RefCell::default(),
            inline_length_em: Cell::default(),
        }
    }

    /// Sets the font configuration.
    pub fn with_font(mut self, font: TextFont) -> Self {
        self.font = font;
        self.items.take();
        self
    }

    /// Returns the font configuration.
    pub fn font(&self) -> &TextFont {
        &self.font
    }

    /// Returns the text basis vectors.
    pub fn basis(&self) -> (DVec3, DVec3) {
        self.basis
    }

    /// Returns the Typst source used as the text body.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Returns the inline length in em units.
    pub fn inline_length_em(&self) -> f64 {
        let _ = self.items();
        self.inline_length_em.get().unwrap_or_default()
    }

    /// Returns the baseline-aligned em box for the text.
    pub fn text_box(&self) -> Parallelogram {
        let (u, v) = self.basis;
        Parallelogram::new(self.origin, (u * self.inline_length_em(), v))
    }

    fn generate_items(&self) -> Vec<VItem> {
        let source = self.typst_source();
        let output = compile_with_options(
            &source,
            CompileOptions {
                include_page_fill: false,
                center_content: false,
            },
        )
        .expect("failed to compile TextItem source");
        let Some(page) = output.document.pages.into_iter().next() else {
            self.inline_length_em.set(Some(0.0));
            return Vec::new();
        };

        let em_height = page.size.y;
        if em_height <= f64::EPSILON {
            self.inline_length_em.set(Some(0.0));
            return Vec::new();
        }
        self.inline_length_em.set(Some(page.size.x / em_height));

        let &Self {
            basis: (u, v),
            origin,
            fill_rgbas,
            stroke_rgbas,
            stroke_width,
            ..
        } = self;
        let affine = DAffine3::from_mat3_translation(DMat3::from_cols(u, v, DVec3::ZERO), origin);
        page.vitems.with(|items| {
            items
                .apply_point_func(|point| {
                    point.x /= em_height;
                    point.y = (point.y + em_height) / em_height;
                    *point = affine.transform_point3(*point);
                })
                .set_fill_color(fill_rgbas)
                .set_stroke_color(stroke_rgbas)
                .set_stroke_width(stroke_width)
                .discard()
        })
    }

    fn typst_source(&self) -> String {
        let mut families = String::new();
        for family in &self.font.families {
            families.push('"');
            families.push_str(family);
            families.push_str("\", ");
        }
        let weight = self.font.variant.weight.to_number();
        let style = match self.font.variant.style {
            FontStyle::Normal => "normal",
            FontStyle::Italic => "italic",
            FontStyle::Oblique => "oblique",
        };
        let stretch = self.font.variant.stretch.to_ratio().repr();
        let features = if self.font.features.is_empty() {
            ":".to_owned()
        } else {
            self.font
                .features
                .iter()
                .map(|(tag, value)| format!("\"{tag}\": {value}"))
                .collect::<Vec<_>>()
                .join(", ")
        };
        let text = &self.text;

        format!(
            r#"#set text(
    top-edge: 1em,
    font: ({families}),
    weight: {weight},
    style: "{style}",
    stretch: {stretch},
    features: ({features}),
)
#set page(width: auto, height: auto, margin: 0pt)

{text}
"#
        )
    }

    fn items(&self) -> Ref<'_, Vec<VItem>> {
        if self.items.borrow().is_none() {
            self.items.replace(Some(self.generate_items()));
        }
        Ref::map(self.items.borrow(), |items| items.as_ref().unwrap())
    }

    fn transform_items(&self, transformation: impl FnOnce(&mut Vec<VItem>)) {
        if let Some(items) = self.items.borrow_mut().as_mut() {
            transformation(items);
        }
    }
}

impl Aabb for TextItem {
    fn aabb(&self) -> [DVec3; 2] {
        self.items().aabb()
    }
}

impl ShiftTransform for TextItem {
    fn shift(&mut self, offset: DVec3) -> &mut Self {
        self.origin += offset;
        self.transform_items(|items| items.shift(offset).discard());
        self
    }
}

impl RotateTransform for TextItem {
    fn rotate_on_axis(&mut self, axis: DVec3, angle: f64) -> &mut Self {
        self.origin.rotate_on_axis(axis, angle);
        self.basis.0.rotate_on_axis(axis, angle);
        self.basis.1.rotate_on_axis(axis, angle);
        self.transform_items(|items| items.rotate_on_axis(axis, angle).discard());
        self
    }
}

impl ScaleTransform for TextItem {
    fn scale(&mut self, scale: DVec3) -> &mut Self {
        self.origin.scale(scale).discard();
        self.basis.0 *= scale;
        self.basis.1 *= scale;
        self.transform_items(|items| items.scale(scale).discard());
        self
    }
}

impl FillColor for TextItem {
    fn fill_color(&self) -> AlphaColor<Srgb> {
        self.fill_rgbas
    }

    fn set_fill_color(&mut self, color: AlphaColor<Srgb>) -> &mut Self {
        self.fill_rgbas = color;
        self.transform_items(|items| items.set_fill_color(color).discard());
        self
    }

    fn set_fill_opacity(&mut self, opacity: f32) -> &mut Self {
        self.fill_rgbas = self.fill_rgbas.with_alpha(opacity);
        self.transform_items(|items| items.set_fill_opacity(opacity).discard());
        self
    }
}

impl StrokeColor for TextItem {
    fn stroke_color(&self) -> AlphaColor<Srgb> {
        self.stroke_rgbas
    }

    fn set_stroke_color(&mut self, color: AlphaColor<Srgb>) -> &mut Self {
        self.stroke_rgbas = color;
        self.transform_items(|items| items.set_stroke_color(color).discard());
        self
    }

    fn set_stroke_opacity(&mut self, opacity: f32) -> &mut Self {
        self.stroke_rgbas = self.stroke_rgbas.with_alpha(opacity);
        self.transform_items(|items| items.set_stroke_opacity(opacity).discard());
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
    use super::*;

    #[test]
    fn text_item_uses_page_metrics() {
        let item = TextItem::new("Hello, world!", 0.25);
        assert!((item.basis.0.length() - 0.25).abs() < 1e-10);
        assert!(item.inline_length_em() > 1.0);
        assert!(!Vec::<VItem>::from(item).is_empty());
    }
}
