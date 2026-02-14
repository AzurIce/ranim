use std::{
    cell::{Ref, RefCell},
    collections::HashMap,
};

use ranim_core::{
    Extract,
    color::{AlphaColor, Srgb},
    core_item::CoreItem,
    glam::{DAffine3, DMat3, DVec3},
    traits::{
        Aabb, AffineTransform, AffineTransformExt, Discard, FillColor, Locate, Rotate, Scale,
        Shift, With as _,
    },
};
use typst::foundations::Repr;

use crate::vitem::{VItem, geometry::anchor::Origin, svg::SvgItem, typst::typst_svg};
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
    /// Basis
    basis: [DVec3; 2],
    /// Origin
    origin: DVec3,
    /// Text content
    text: String,
    /// Font info
    font: TextFont,
    /// Cached items
    items: RefCell<Option<Vec<VItem>>>,
    /// Fill color
    fill_rbgas: AlphaColor<Srgb>,
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
            basis: [DVec3::X * em_size, DVec3::Y * em_size],
            origin: DVec3::ZERO,
            text: text.into(),
            font: TextFont::default(),
            items: RefCell::default(),
            fill_rbgas: AlphaColor::WHITE,
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

    /// Get basis
    pub fn basis(&self) -> [DVec3; 2] {
        self.basis
    }

    /// Get text
    pub fn text(&self) -> &str {
        &self.text
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

        let svg_src = dbg!(typst_svg(
            format!(
                r#"
#set text(
    top-edge: 1em,
    bottom-edge: "baseline",
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
            .as_str()
        ));
        let items = Vec::<VItem>::from(SvgItem::new(svg_src)).with(|item| {
            let &Self {
                basis: [x_axis, y_axis],
                origin,
                ..
            } = self;
            let [min, max] = item[0].aabb();
            let h = max.y - min.y;
            let mat = DAffine3::from_mat3_translation(
                DMat3::from_cols(x_axis, y_axis, DVec3::ZERO),
                origin,
            );
            item.shift(-min)
                .scale_at_point(DVec3::splat(1. / h), DVec3::ZERO)
                .affine_transform(mat);
        });
        items[1..].into()
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

impl Shift for TextItem {
    fn shift(&mut self, offset: DVec3) -> &mut Self {
        self.origin += offset;
        self.transform_items(|item| item.shift(offset).discard());
        self
    }
}

impl Rotate for TextItem {
    fn rotate_at_point(&mut self, angle: f64, axis: DVec3, point: DVec3) -> &mut Self {
        self.origin.rotate_at_point(angle, axis, point);
        self.basis.rotate_at_point(angle, axis, DVec3::ZERO);
        self.transform_items(|item| item.rotate_at_point(angle, axis, point).discard());
        self
    }
}

impl Scale for TextItem {
    fn scale_at_point(&mut self, scale: DVec3, point: DVec3) -> &mut Self {
        self.origin.scale_at_point(scale, point);
        self.basis.iter_mut().for_each(|v| *v *= scale);
        self.transform_items(|item| item.scale_at_point(scale, point).discard());
        self
    }
}

impl AffineTransform for TextItem {
    fn affine_transform_at_point(&mut self, mat: DAffine3, origin: DVec3) -> &mut Self {
        self.origin.affine_transform_at_point(mat, origin);
        self.basis
            .iter_mut()
            .for_each(|v| *v = mat.transform_vector3(*v));
        self.transform_items(|item| item.affine_transform_at_point(mat, origin).discard());
        self
    }
}

impl FillColor for TextItem {
    fn fill_color(&self) -> AlphaColor<Srgb> {
        self.fill_rbgas
    }

    fn set_fill_color(&mut self, color: AlphaColor<Srgb>) -> &mut Self {
        self.fill_rbgas = color;
        self.transform_items(|item| item.set_fill_color(color).discard());
        self
    }

    fn set_fill_opacity(&mut self, opacity: f32) -> &mut Self {
        self.fill_rbgas = self.fill_rbgas.with_alpha(opacity);
        self.transform_items(|item| item.set_fill_opacity(opacity).discard());
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
        assert_float_absolute_eq!(item.basis[0].length(), 0.25, 1e-10);
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
