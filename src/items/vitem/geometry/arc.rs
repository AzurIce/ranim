use color::{AlphaColor, Srgb};
use glam::DVec3;
use ranim_macros::Interpolatable;

use crate::{
    components::Anchor,
    items::vitem::{DEFAULT_STROKE_WIDTH, VItem},
    render::primitives::{Extract, vitem::VItemPrimitive},
    traits::{BoundingBox, Opacity, Rotate, Scale, Shift, StrokeColor, StrokeWidth, With},
};

// MARK: ### Arc ###
/// An arc
#[derive(Clone, Debug, Interpolatable)]
pub struct Arc {
    /// Center
    pub center: DVec3,
    /// Radius
    pub radius: f64,
    /// Angle
    pub angle: f64,
    pub(super) up: DVec3,
    /// The normal vec of the arc plane
    pub normal: DVec3,

    /// Stroke rgba
    pub stroke_rgba: AlphaColor<Srgb>,
    /// Stroke width
    pub stroke_width: f32,
}

impl Arc {
    /// Constructor
    pub fn new(angle: f64, radius: f64) -> Self {
        Self {
            center: DVec3::ZERO,
            radius,
            angle,
            up: DVec3::Y,
            normal: DVec3::Z,
            stroke_rgba: AlphaColor::WHITE,
            stroke_width: DEFAULT_STROKE_WIDTH,
        }
    }
    /// Scale the arc by the given scale, with the given anchor as the center.
    ///
    /// Note that this accepts a `f64` scale dispite of [`Scale`]'s `DVec3`,
    /// because this keeps the arc a arc.
    pub fn scale(&mut self, scale: f64) -> &mut Self {
        self.scale_by_anchor(scale, Anchor::CENTER)
    }
    /// Scale the arc by the given scale, with the given anchor as the center.
    ///
    /// Note that this accepts a `f64` scale dispite of [`Scale`]'s `DVec3`,
    /// because this keeps the arc a arc.
    pub fn scale_by_anchor(&mut self, scale: f64, anchor: Anchor) -> &mut Self {
        let anchor = Anchor::Point(anchor.get_pos(self));
        self.radius *= scale;
        self.center.scale_by_anchor(DVec3::splat(scale), anchor);
        self
    }
    /// The start point
    pub fn start(&self) -> DVec3 {
        let right = self.up.cross(self.normal).normalize();
        self.center + self.radius * right
    }
    /// The end point
    pub fn end(&self) -> DVec3 {
        let right = self.up.cross(self.normal).normalize();
        self.center
            + self.radius * self.angle.cos() * right
            + self.radius * self.angle.sin() * self.up
    }
}

// MARK: Traits impl
impl BoundingBox for Arc {
    /// Note that the arc's bounding box is actually same as the circle's bounding box.
    fn get_bounding_box(&self) -> [DVec3; 3] {
        let right = -self.normal.cross(self.up).normalize();
        [
            self.center - self.radius * right + self.radius * self.up,
            self.center + self.radius * right - self.radius * self.up,
        ]
        .get_bounding_box()
    }
}

impl Shift for Arc {
    fn shift(&mut self, shift: DVec3) -> &mut Self {
        self.center.shift(shift);
        self
    }
}

impl Rotate for Arc {
    fn rotate_by_anchor(&mut self, angle: f64, axis: DVec3, anchor: Anchor) -> &mut Self {
        let anchor = Anchor::Point(match anchor {
            Anchor::Point(point) => point,
            Anchor::Edge(edge) => self.get_bounding_box_point(edge),
        });
        self.center.rotate_by_anchor(angle, axis, anchor);
        self.up.rotate_by_anchor(angle, axis, Anchor::ORIGIN);
        self.normal.rotate_by_anchor(angle, axis, Anchor::ORIGIN);
        self
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

// MARK: Conversions
impl From<Arc> for VItem {
    fn from(value: Arc) -> Self {
        const NUM_SEGMENTS: usize = 8;
        let len = 2 * NUM_SEGMENTS + 1;

        let Arc {
            center,
            radius,
            angle,
            up,
            normal,
            stroke_rgba,
            stroke_width,
        } = value;

        let right = -normal.cross(up).normalize();

        let mut vpoints = (0..len)
            .map(|i| {
                let angle = angle * i as f64 / (len - 1) as f64;
                let (mut x, mut y) = (angle.cos(), angle.sin());
                if x.abs() < 1.8e-7 {
                    x = 0.0;
                }
                if y.abs() < 1.8e-7 {
                    y = 0.0;
                }
                (x * right + y * up) * radius
            })
            .collect::<Vec<_>>();

        let theta = angle / NUM_SEGMENTS as f64;
        vpoints.iter_mut().skip(1).step_by(2).for_each(|p| {
            *p /= (theta / 2.0).cos();
        });
        VItem::from_vpoints(vpoints).with(|vitem| {
            vitem
                .set_stroke_color(stroke_rgba)
                .set_stroke_width(stroke_width)
                .shift(center);
        })
    }
}

impl Extract for Arc {
    type Target = VItemPrimitive;
    fn extract(&self) -> Self::Target {
        VItem::from(self.clone()).extract()
    }
}

// MARK: ### ArcBetweenPoints ###
/// An arc between points
#[derive(Clone, Debug, Interpolatable)]
pub struct ArcBetweenPoints {
    /// Start point
    pub start: DVec3,
    /// End point
    pub end: DVec3,
    /// Arc angle
    pub angle: f64,
    up: DVec3,
    /// Arc plane normal vec
    pub normal: DVec3,

    /// Stroke rgba
    pub stroke_rgba: AlphaColor<Srgb>,
    /// Stroke width
    pub stroke_width: f32,
}
impl ArcBetweenPoints {
    /// Constructor
    pub fn new(start: DVec3, end: DVec3, angle: f64) -> Self {
        Self {
            start,
            end,
            angle,
            up: DVec3::Y,
            normal: DVec3::Z,

            stroke_rgba: AlphaColor::WHITE,
            stroke_width: DEFAULT_STROKE_WIDTH,
        }
    }
    /// Scale the arc by the given scale, with the given anchor as the center.
    ///
    /// Note that this accepts a `f64` scale dispite of [`Scale`]'s `DVec3`,
    /// because this keeps the arc a arc.
    pub fn scale(&mut self, scale: f64) -> &mut Self {
        self.scale_by_anchor(scale, Anchor::CENTER)
    }
    /// Scale the arc by the given scale, with the given anchor as the center.
    ///
    /// Note that this accepts a `f64` scale dispite of [`Scale`]'s `DVec3`,
    /// because this keeps the arc a arc.
    pub fn scale_by_anchor(&mut self, scale: f64, anchor: Anchor) -> &mut Self {
        let anchor = Anchor::Point(match anchor {
            Anchor::Point(point) => point,
            Anchor::Edge(edge) => self.get_bounding_box_point(edge),
        });
        self.start.scale_by_anchor(DVec3::splat(scale), anchor);
        self.end.scale_by_anchor(DVec3::splat(scale), anchor);
        self
    }
}

// MARK: Traits impl
impl BoundingBox for ArcBetweenPoints {
    /// Note that the arc's bounding box is actually same as the circle's bounding box.
    fn get_bounding_box(&self) -> [DVec3; 3] {
        // TODO: optimize this
        Arc::from(self.clone()).get_bounding_box()
    }
}

impl Shift for ArcBetweenPoints {
    fn shift(&mut self, shift: DVec3) -> &mut Self {
        self.start.shift(shift);
        self.end.shift(shift);
        self
    }
}

impl Rotate for ArcBetweenPoints {
    fn rotate_by_anchor(&mut self, angle: f64, axis: DVec3, anchor: Anchor) -> &mut Self {
        let anchor = Anchor::Point(match anchor {
            Anchor::Point(point) => point,
            Anchor::Edge(edge) => self.get_bounding_box_point(edge),
        });
        self.start.rotate_by_anchor(angle, axis, anchor);
        self.end.rotate_by_anchor(angle, axis, anchor);
        self.up.rotate_by_anchor(angle, axis, Anchor::ORIGIN);
        self.normal.rotate_by_anchor(angle, axis, Anchor::ORIGIN);
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

// MARK: Conversions
impl From<ArcBetweenPoints> for Arc {
    fn from(value: ArcBetweenPoints) -> Arc {
        let ArcBetweenPoints {
            start,
            end,
            angle,
            up,
            normal,
            stroke_rgba,
            stroke_width,
        } = value;
        let radius = (start.distance(end) / 2.0) / (angle / 2.0).sin();

        Arc {
            angle,
            radius,
            center: DVec3::ZERO,
            up,
            normal,
            stroke_rgba,
            stroke_width,
        }
        .with(|arc| {
            let cur_start = arc.start();

            let v1 = arc.end() - arc.start();
            let v2 = end - start;

            let rot_angle = v1.angle_between(v2);
            let mut rot_axis = v1.cross(v2);
            if rot_axis.length_squared() <= f64::EPSILON {
                rot_axis = DVec3::NEG_Z;
            }
            rot_axis = rot_axis.normalize();
            arc.shift(start - cur_start).rotate_by_anchor(
                rot_angle,
                rot_axis,
                Anchor::Point(start),
            );
        })
    }
}

impl From<ArcBetweenPoints> for VItem {
    fn from(value: ArcBetweenPoints) -> Self {
        Arc::from(value).into()
    }
}

impl Extract for ArcBetweenPoints {
    type Target = VItemPrimitive;
    fn extract(&self) -> Self::Target {
        Arc::from(self.clone()).extract()
    }
}

#[cfg(test)]
mod tests {
    use std::f64::consts::PI;

    use assert_float_eq::assert_float_absolute_eq;
    use glam::dvec3;

    use super::*;

    #[test]
    fn test_arc() {
        let arc = Arc::new(PI / 2.0, 2.0);
        assert_float_absolute_eq!(
            arc.start().distance_squared(dvec3(2.0, 0.0, 0.0)),
            0.0,
            1e-10
        );
        assert_float_absolute_eq!(arc.end().distance_squared(dvec3(0.0, 2.0, 0.0)), 0.0, 1e-10);

        let arc_between_points =
            ArcBetweenPoints::new(dvec3(2.0, 0.0, 0.0), dvec3(0.0, 2.0, 0.0), PI / 2.0);
        let arc_between_points = Arc::from(arc_between_points);
        assert_float_absolute_eq!(
            arc.center.distance_squared(arc_between_points.center),
            0.0,
            1e-10
        );
        assert_float_absolute_eq!(arc.radius - arc_between_points.radius, 0.0, 1e-10);
        assert_float_absolute_eq!(arc.angle - arc_between_points.angle, 0.0, 1e-10);

        let arc_between_points =
            ArcBetweenPoints::new(dvec3(0.0, 2.0, 0.0), dvec3(2.0, 0.0, 0.0), PI / 2.0);
        let arc_between_points = Arc::from(arc_between_points);
        let arc = Arc::new(PI / 2.0, 2.0).with(|arc| {
            arc.rotate(PI, DVec3::NEG_Z).shift(dvec3(2.0, 2.0, 0.0));
        });
        assert_float_absolute_eq!(
            arc.center.distance_squared(arc_between_points.center),
            0.0,
            1e-10
        );
        assert_float_absolute_eq!(arc.radius - arc_between_points.radius, 0.0, 1e-10);
        assert_float_absolute_eq!(arc.angle - arc_between_points.angle, 0.0, 1e-10);
    }
}
