use std::f64::consts::PI;

use color::{AlphaColor, Srgb};
use glam::DVec3;
use ranim_macros::Interpolatable;

use crate::{
    components::Anchor,
    items::vitem::{VItem, DEFAULT_STROKE_WIDTH},
    render::primitives::{vitem::VItemPrimitive, Extract},
    traits::{BoundingBox, FillColor, Opacity, Rotate, Scale, Shift, StrokeColor, With},
};

use super::Arc;

// MARK: ### Circle ###
#[derive(Clone, Debug, Interpolatable)]
pub struct Circle {
    pub center: DVec3,
    pub radius: f64,
    up: DVec3,
    pub normal: DVec3,

    pub stroke_rgba: AlphaColor<Srgb>,
    pub stroke_width: f32,
    pub fill_rgba: AlphaColor<Srgb>,
}

impl Circle {
    pub fn new(radius: f64) -> Self {
        Self {
            center: DVec3::ZERO,
            radius,
            up: DVec3::Y,
            normal: DVec3::Z,

            stroke_rgba: AlphaColor::WHITE,
            stroke_width: DEFAULT_STROKE_WIDTH,
            fill_rgba: AlphaColor::TRANSPARENT,
        }
    }
    /// Scale the circle by the given scale, with the given anchor as the center.
    ///
    /// Note that this accepts a `f64` scale dispite of [`Scale`]'s `DVec3`,
    /// because this keeps the circle a circle.
    pub fn scale(&mut self, scale: f64) -> &mut Self {
        self.scale_by_anchor(scale, Anchor::CENTER)
    }
    /// Scale the circle by the given scale, with the given anchor as the center.
    ///
    /// Note that this accepts a `f64` scale dispite of [`Scale`]'s `DVec3`,
    /// because this keeps the circle a circle.
    pub fn scale_by_anchor(&mut self, scale: f64, anchor: Anchor) -> &mut Self {
        let anchor = Anchor::Point(match anchor {
            Anchor::Point(point) => point,
            Anchor::Edge(edge) => self.get_bounding_box_point(edge),
        });
        self.radius *= scale;
        self.center.scale_by_anchor(DVec3::splat(scale), anchor);
        self
    }
}

// MARK: Traits impl
impl BoundingBox for Circle {
    fn get_bounding_box(&self) -> [DVec3; 3] {
        let right = -self.normal.cross(self.up).normalize();
        [
            self.center - self.radius * right + self.radius * self.up,
            self.center + self.radius * right - self.radius * self.up,
        ]
        .get_bounding_box()
    }
}

impl Shift for Circle {
    fn shift(&mut self, shift: DVec3) -> &mut Self {
        self.center.shift(shift);
        self
    }
}

impl Rotate for Circle {
    fn rotate_by_anchor(&mut self, angle: f64, axis: DVec3, anchor: Anchor) -> &mut Self {
        let anchor = Anchor::Point(anchor.get_pos(self));
        self.center.rotate_by_anchor(angle, axis, anchor);
        self.up.rotate_by_anchor(angle, axis, Anchor::ORIGIN);
        self.normal.rotate_by_anchor(angle, axis, Anchor::ORIGIN);
        self
    }
}

impl Opacity for Circle {
    fn set_opacity(&mut self, opacity: f32) -> &mut Self {
        self.stroke_rgba = self.stroke_rgba.with_alpha(opacity);
        self.fill_rgba = self.fill_rgba.with_alpha(opacity);
        self
    }
}

impl StrokeColor for Circle {
    fn stroke_color(&self) -> AlphaColor<Srgb> {
        self.stroke_rgba
    }
    fn set_stroke_color(&mut self, color: AlphaColor<Srgb>) -> &mut Self {
        self.stroke_rgba = color;
        self
    }
    fn set_stroke_opacity(&mut self, opacity: f32) -> &mut Self {
        self.stroke_rgba = self.stroke_rgba.with_alpha(opacity);
        self
    }
}

impl FillColor for Circle {
    fn fill_color(&self) -> AlphaColor<Srgb> {
        self.fill_rgba
    }
    fn set_fill_color(&mut self, color: AlphaColor<Srgb>) -> &mut Self {
        self.fill_rgba = color;
        self
    }
    fn set_fill_opacity(&mut self, opacity: f32) -> &mut Self {
        self.fill_rgba = self.fill_rgba.with_alpha(opacity);
        self
    }
}

// MARK: Conversions
impl From<Circle> for Arc {
    fn from(value: Circle) -> Self {
        let Circle {
            center,
            radius,
            up,
            normal,
            stroke_rgba,
            stroke_width,
            ..
        } = value;
        Self {
            center,
            radius,
            up,
            normal,
            angle: 2.0 * PI,
            stroke_rgba,
            stroke_width,
        }
    }
}

impl From<Circle> for VItem {
    fn from(value: Circle) -> Self {
        let fill_rgba = value.fill_rgba;
        VItem::from(Arc::from(value)).with(|item| {
            item.set_fill_color(fill_rgba);
        })
    }
}

impl Extract for Circle {
    type Target = VItemPrimitive;
    fn extract(&self) -> Self::Target {
        VItem::from(self.clone()).extract()
    }
}
