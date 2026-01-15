use std::f64::consts::PI;

use color::{AlphaColor, Srgb};
use glam::DVec3;
use ranim_core::{Extract, color, core_item::CoreItem, glam, traits::Anchor};

use crate::vitem::{DEFAULT_STROKE_WIDTH, Proj};
use ranim_core::traits::{
    BoundingBox, FillColor, Opacity, Rotate, Scale, Shift, StrokeColor, With,
};

use crate::vitem::VItem;

use super::Arc;

// MARK: ### Circle ###
/// An circle
#[derive(Clone, Debug, ranim_macros::Interpolatable)]
pub struct Circle {
    /// Proj
    pub proj: Proj,
    /// Center
    pub center: DVec3,
    /// Radius
    pub radius: f64,

    /// Stroke rgba
    pub stroke_rgba: AlphaColor<Srgb>,
    /// Stroke width
    pub stroke_width: f32,
    /// Fill rgba
    pub fill_rgba: AlphaColor<Srgb>,
}

impl Circle {
    /// Constructor
    pub fn new(radius: f64) -> Self {
        Self {
            proj: Proj::default(),
            center: DVec3::ZERO,
            radius,

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
        let (u, v) = self.proj.basis();
        let r = self.radius * (u + v);
        [self.center + r, self.center - r].get_bounding_box()
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
        self.proj.rotate(angle, axis);
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
            proj,
            center,
            radius,
            stroke_rgba,
            stroke_width,
            ..
        } = value;
        Self {
            proj,
            center,
            radius,
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
    type Target = CoreItem;
    fn extract_into(&self, buf: &mut Vec<Self::Target>) {
        VItem::from(self.clone()).extract_into(buf);
    }
}
