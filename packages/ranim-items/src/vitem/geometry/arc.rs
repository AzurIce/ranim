use ranim_core::{
    Extract,
    anchor::{Aabb, AabbPoint, Locate},
    core_item::CoreItem,
    traits::{ApplyTransform, Opacity, ScaleTransform, ShiftTransform, StrokeColor},
};
use ranim_core::{
    color::{AlphaColor, Srgb},
    glam::{self, DVec3},
};

use crate::vitem::geometry::EllipticArc;
use crate::vitem::{DEFAULT_STROKE_WIDTH, VItem};

/// An arc centered at the origin in the XY plane.
#[derive(Clone, Debug, ranim_macros::Interpolatable)]
pub struct Arc {
    /// Radius.
    pub radius: f64,
    /// Span angle in radians.
    pub angle: f64,
    /// Stroke rgba.
    pub stroke_rgba: AlphaColor<Srgb>,
    /// Stroke width.
    pub stroke_width: f32,
}

impl Arc {
    /// Creates an arc with the given angle and radius.
    pub fn new(angle: f64, radius: f64) -> Self {
        Self {
            radius,
            angle,
            stroke_rgba: AlphaColor::WHITE,
            stroke_width: DEFAULT_STROKE_WIDTH,
        }
    }

    /// Scales the intrinsic radius.
    pub fn scale(&mut self, scale: f64) -> &mut Self {
        self.radius *= scale;
        self
    }

    /// The start point in canonical local coordinates.
    pub fn start(&self) -> DVec3 {
        DVec3::X * self.radius
    }

    /// The end point in canonical local coordinates.
    pub fn end(&self) -> DVec3 {
        DVec3::new(self.angle.cos(), self.angle.sin(), 0.0) * self.radius
    }
}

impl Aabb for Arc {
    fn aabb(&self) -> [DVec3; 2] {
        VItem::from(self.clone()).aabb()
    }
}

impl Opacity for Arc {
    fn set_opacity(&mut self, opacity: f32) -> &mut Self {
        self.stroke_rgba = self.stroke_rgba.with_alpha(opacity);
        self
    }
}

impl StrokeColor for Arc {
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

impl From<Arc> for VItem {
    fn from(value: Arc) -> Self {
        EllipticArc::from(value).into()
    }
}

impl Extract for Arc {
    type Target = CoreItem;
    fn extract_into(&self, buf: &mut Vec<Self::Target>) {
        VItem::from(self.clone()).extract_into(buf);
    }
}

/// An arc whose local geometry is defined by its start and end points.
#[derive(Clone, Debug, ranim_macros::Interpolatable)]
pub struct ArcBetweenPoints {
    /// Start point.
    pub start: DVec3,
    /// End point.
    pub end: DVec3,
    /// Arc angle.
    pub angle: f64,
    /// Stroke rgba.
    pub stroke_rgba: AlphaColor<Srgb>,
    /// Stroke width.
    pub stroke_width: f32,
}

impl ArcBetweenPoints {
    /// Creates an arc between two local points.
    pub fn new(start: DVec3, end: DVec3, angle: f64) -> Self {
        Self {
            start,
            end,
            angle,
            stroke_rgba: AlphaColor::WHITE,
            stroke_width: DEFAULT_STROKE_WIDTH,
        }
    }

    /// Returns the circle center in local coordinates.
    pub fn center(&self) -> DVec3 {
        let chord = self.end - self.start;
        let midpoint = (self.start + self.end) / 2.0;
        let perpendicular = DVec3::Z.cross(chord).normalize_or_zero();
        midpoint + perpendicular * (chord.length() / (2.0 * (self.angle / 2.0).tan()))
    }

    /// Scales the intrinsic start and end points about their AABB center.
    pub fn scale(&mut self, scale: f64) -> &mut Self {
        self.scale_at(scale, AabbPoint::CENTER)
    }

    /// Scales the intrinsic start and end points about an anchor.
    pub fn scale_at<T>(&mut self, scale: f64, anchor: T) -> &mut Self
    where
        T: Locate<Self>,
    {
        let point = anchor.locate(self);
        self.start
            .shift(-point)
            .scale(DVec3::splat(scale))
            .shift(point);
        self.end
            .shift(-point)
            .scale(DVec3::splat(scale))
            .shift(point);
        self
    }
}

impl Aabb for ArcBetweenPoints {
    fn aabb(&self) -> [DVec3; 2] {
        VItem::from(self.clone()).aabb()
    }
}

impl<G: Into<ranim_core::prelude::Similarity>> ApplyTransform<G> for ArcBetweenPoints {
    fn apply(&mut self, transform: G) -> &mut Self {
        let transform = transform.into();
        self.start = transform.transform_point(self.start);
        self.end = transform.transform_point(self.end);
        self
    }
}

impl Opacity for ArcBetweenPoints {
    fn set_opacity(&mut self, opacity: f32) -> &mut Self {
        self.stroke_rgba = self.stroke_rgba.with_alpha(opacity);
        self
    }
}

impl StrokeColor for ArcBetweenPoints {
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

impl From<ArcBetweenPoints> for VItem {
    fn from(value: ArcBetweenPoints) -> Self {
        let center = value.center();
        let radius = value.start.distance(center);
        let start = value.start - center;
        let start_angle = start.y.atan2(start.x);
        let mut item = VItem::from(EllipticArc {
            radius: glam::DVec2::splat(radius),
            start_angle,
            angle: value.angle,
            stroke_rgba: value.stroke_rgba,
            stroke_width: value.stroke_width,
        });
        item.shift(center);
        item
    }
}

impl Extract for ArcBetweenPoints {
    type Target = CoreItem;
    fn extract_into(&self, buf: &mut Vec<Self::Target>) {
        VItem::from(self.clone()).extract_into(buf);
    }
}

#[cfg(test)]
mod tests {
    use std::f64::consts::PI;

    use assert_float_eq::assert_float_absolute_eq;
    use glam::dvec3;

    use super::*;

    #[test]
    fn arc_is_canonical_local() {
        let arc = Arc::new(PI / 2.0, 2.0);
        assert_float_absolute_eq!(
            arc.start().distance_squared(dvec3(2.0, 0.0, 0.0)),
            0.0,
            1e-10
        );
        assert_float_absolute_eq!(arc.end().distance_squared(dvec3(0.0, 2.0, 0.0)), 0.0, 1e-10);
    }

    #[test]
    fn arc_between_points_preserves_endpoints() {
        let arc = ArcBetweenPoints::new(dvec3(2.0, 0.0, 0.0), dvec3(0.0, 2.0, 0.0), PI / 2.0);
        assert_float_absolute_eq!(arc.center().distance_squared(DVec3::ZERO), 0.0, 1e-10);
        let item = VItem::from(arc);
        assert_float_absolute_eq!(
            item.vpoints[0].distance_squared(dvec3(2.0, 0.0, 0.0)),
            0.0,
            1e-10
        );
        assert_float_absolute_eq!(
            item.vpoints[item.vpoints.len() - 1].distance_squared(dvec3(0.0, 2.0, 0.0)),
            0.0,
            1e-10
        );
    }
}
