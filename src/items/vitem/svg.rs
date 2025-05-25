use glam::DVec3;

use crate::{
    items::Group, render::primitives::{vitem::VItemPrimitive, Extract}, traits::{BoundingBox, FillColor, Opacity, Rotate, Scale, Shift, StrokeColor, StrokeWidth}, utils::svg::vitems_from_tree
};

use super::VItem;

// MARK: ### SvgItem ###
#[derive(Clone)]
pub struct SvgItem(Vec<VItem>);

impl SvgItem {
    pub fn new(svg: impl AsRef<str>) -> Self {
        let svg = svg.as_ref();
        let tree = usvg::Tree::from_str(svg, &usvg::Options::default()).unwrap();

        let mut vitem_group = Self(vitems_from_tree(&tree));
        vitem_group.put_center_on(DVec3::ZERO);
        vitem_group.rotate(std::f64::consts::PI, DVec3::X);
        vitem_group
    }
}

// MARK: Trait impls
impl BoundingBox for SvgItem {
    fn get_bounding_box(&self) -> [glam::DVec3; 3] {
        self.0.get_bounding_box()
    }
}

impl Shift for SvgItem {
    fn shift(&mut self, shift: glam::DVec3) -> &mut Self {
        self.0.shift(shift);
        self
    }
}

impl Rotate for SvgItem {
    fn rotate_by_anchor(
        &mut self,
        angle: f64,
        axis: glam::DVec3,
        anchor: crate::components::Anchor,
    ) -> &mut Self {
        self.0.rotate_by_anchor(angle, axis, anchor);
        self
    }
}

impl Scale for SvgItem {
    fn scale_by_anchor(
        &mut self,
        scale: glam::DVec3,
        anchor: crate::components::Anchor,
    ) -> &mut Self {
        self.0.scale_by_anchor(scale, anchor);
        self
    }
}

impl FillColor for SvgItem {
    fn fill_color(&self) -> color::AlphaColor<color::Srgb> {
        self.0[0].fill_color()
    }
    fn set_fill_color(&mut self, color: color::AlphaColor<color::Srgb>) -> &mut Self {
        self.0.set_fill_color(color);
        self
    }
    fn set_fill_opacity(&mut self, opacity: f32) -> &mut Self {
        self.0.set_fill_opacity(opacity);
        self
    }
}

impl StrokeColor for SvgItem {
    fn stroke_color(&self) -> color::AlphaColor<color::Srgb> {
        self.0[0].fill_color()
    }
    fn set_stroke_color(&mut self, color: color::AlphaColor<color::Srgb>) -> &mut Self {
        self.0.set_stroke_color(color);
        self
    }
    fn set_stroke_opacity(&mut self, opacity: f32) -> &mut Self {
        self.0.set_stroke_opacity(opacity);
        self
    }
}

impl Opacity for SvgItem {
    fn set_opacity(&mut self, opacity: f32) -> &mut Self {
        self.0.set_fill_opacity(opacity);
        self.0.set_stroke_opacity(opacity);
        self
    }
}

impl StrokeWidth for SvgItem {
    fn apply_stroke_func(
        &mut self,
        f: impl for<'a> Fn(&'a mut [crate::components::width::Width]),
    ) -> &mut Self {
        self.0.iter_mut().for_each(|vitem| {
            vitem.apply_stroke_func(&f);
        });
        self
    }
    fn set_stroke_width(&mut self, width: f32) -> &mut Self {
        self.0.set_stroke_width(width);
        self
    }
}

// MARK: Conversions
impl From<SvgItem> for Group<VItem> {
    fn from(value: SvgItem) -> Self {
        Self(value.0)
    }
}

impl Extract for SvgItem {
    type Target = Vec<VItemPrimitive>;
    fn extract(&self) -> Self::Target {
        self.0.iter().map(|vitem| vitem.extract()).collect()
    }
}
