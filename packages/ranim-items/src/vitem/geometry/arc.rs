use glam::DVec3;
use ranim_core::anchor::AabbPoint;
use ranim_core::anchor::{Aabb, Anchor};
use ranim_core::components::vpoint::VPointVec;
use ranim_core::core_item::vitem::Basis2d;
use ranim_core::{
    glam,
    traits::{RotateTransform, ScaleTransform, ShiftTransform},
};

use crate::vitem::geometry::EllipticArc;
use crate::vitem::{VItem, VPath};

// MARK: ### Arc ###
/// An arc
#[derive(Clone, Debug, ranim_macros::Interpolatable)]
pub struct Arc {
    /// Normalized normal vector
    pub normal: DVec3,
    /// Normalized start direction vector
    pub start_dir: DVec3,
    /// Center
    pub center: DVec3,
    /// Radius
    pub radius: f64,
    /// Angle
    pub angle: f64,
}

impl Arc {
    /// Constructor
    pub fn new(angle: f64, radius: f64) -> Self {
        Self {
            normal: DVec3::Z,
            center: DVec3::ZERO,
            start_dir: DVec3::X,
            radius,
            angle,
        }
    }
}

impl VItem<Arc> {
    /// Constructor
    pub fn new(angle: f64, radius: f64) -> Self {
        Self::new_with(Arc::new(angle, radius))
    }
    /// Scale the arc by the given scale, with the given anchor as the center.
    pub fn scale(&mut self, scale: f64) -> &mut Self {
        self.scale_by_anchor(scale, AabbPoint::CENTER)
    }
    /// Scale the arc by the given scale, with the given anchor as the center.
    pub fn scale_by_anchor<T>(&mut self, scale: f64, anchor: T) -> &mut Self
    where
        T: Anchor<Self>,
    {
        let anchor = anchor.locate_on(self);
        self.with_inner_mut(|arc| {
            arc.radius *= scale;
            arc.center
                .shift(-anchor)
                .scale(DVec3::splat(scale))
                .shift(anchor);
        });
        self
    }
    /// The start point
    pub fn start(&self) -> DVec3 {
        self.with_inner(|arc| arc.center + arc.start_dir * arc.radius)
    }
    /// The end point
    pub fn end(&self) -> DVec3 {
        self.with_inner(|arc| {
            let v_dir = arc.normal.cross(arc.start_dir);
            arc.center + arc.radius * (arc.angle.cos() * arc.start_dir + arc.angle.sin() * v_dir)
        })
    }
}

// MARK: Traits impl
impl ShiftTransform for Arc {
    fn shift(&mut self, shift: DVec3) -> &mut Self {
        self.center.shift(shift);
        self
    }
}

impl RotateTransform for Arc {
    fn rotate_on_axis(&mut self, axis: DVec3, angle: f64) -> &mut Self {
        self.center.rotate_on_axis(axis, angle);
        self.start_dir.rotate_on_axis(axis, angle);
        self.normal.rotate_on_axis(axis, angle);
        self
    }
}

impl VPath for Arc {
    fn normal(&self) -> DVec3 {
        self.normal
    }
}

impl Aabb for Arc {
    fn aabb(&self) -> [DVec3; 2] {
        VPointVec::from(self.clone()).aabb()
    }
}

impl From<Arc> for VPointVec {
    fn from(value: Arc) -> Self {
        EllipticArc::from(value).into()
    }
}

// MARK: ### ArcBetweenPoints ###
/// An arc between points
#[derive(Clone, Debug, ranim_macros::Interpolatable)]
pub struct ArcBetweenPoints {
    /// Projection
    pub basis: Basis2d,
    /// Start point
    pub start: DVec3,
    /// End point
    pub end: DVec3,
    /// Arc angle
    pub angle: f64,
}

impl ArcBetweenPoints {
    /// Constructor
    pub fn new(start: DVec3, end: DVec3, angle: f64) -> Self {
        Self {
            basis: Basis2d::default(),
            start,
            end,
            angle,
        }
    }
    /// Scale the arc by the given scale, with the given anchor as the center.
    pub fn scale(&mut self, scale: f64) -> &mut Self {
        self.scale_at(scale, AabbPoint::CENTER)
    }
    /// Scale the arc by the given scale, with the given anchor as the center.
    pub fn scale_at<T>(&mut self, scale: f64, anchor: T) -> &mut Self
    where
        T: Anchor<Self>,
    {
        let point = anchor.locate_on(self);
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

// MARK: Traits impl
impl ShiftTransform for ArcBetweenPoints {
    fn shift(&mut self, shift: DVec3) -> &mut Self {
        self.start.shift(shift);
        self.end.shift(shift);
        self
    }
}

impl RotateTransform for ArcBetweenPoints {
    fn rotate_on_axis(&mut self, axis: DVec3, angle: f64) -> &mut Self {
        self.start.rotate_on_axis(axis, angle);
        self.end.rotate_on_axis(axis, angle);
        self.basis.rotate_on_axis(axis, angle);
        self
    }
}

impl VPath for ArcBetweenPoints {
    fn normal(&self) -> DVec3 {
        self.basis.normal()
    }
}

impl From<ArcBetweenPoints> for VPointVec {
    fn from(value: ArcBetweenPoints) -> Self {
        Arc::from(value).into()
    }
}

// MARK: Conversions
impl From<ArcBetweenPoints> for Arc {
    fn from(value: ArcBetweenPoints) -> Arc {
        let ArcBetweenPoints {
            basis,
            start,
            end,
            angle,
        } = value;
        let normal = basis.normal();

        let chord = end - start;
        let chord_dir = chord.normalize();
        let chord_mid = (start + end) * 0.5;

        let radius = (chord.length() * 0.5) / (angle * 0.5).sin();
        let center_dist = radius * (angle * 0.5).cos();
        let perp_dir = normal.cross(chord_dir).normalize();
        let center = chord_mid + center_dist * perp_dir;
        let start_dir = (start - center).normalize();

        Arc {
            normal,
            start_dir,
            center,
            radius,
            angle,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::f64::consts::PI;

    use assert_float_eq::assert_float_absolute_eq;
    use glam::dvec3;
    use ranim_core::traits::{ShiftTransformExt, With};

    use crate::vitem::{VItem, geometry::anchor::Origin};

    use super::*;

    #[test]
    fn test_arc() {
        let arc = VItem::<Arc>::new(PI / 2.0, 2.0);
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
            arc.inner.center.distance_squared(arc_between_points.center),
            0.0,
            1e-10
        );
        assert_float_absolute_eq!(arc.inner.radius - arc_between_points.radius, 0.0, 1e-10);
        assert_float_absolute_eq!(arc.inner.angle - arc_between_points.angle, 0.0, 1e-10);

        let arc_between_points =
            ArcBetweenPoints::new(dvec3(0.0, 2.0, 0.0), dvec3(2.0, 0.0, 0.0), PI / 2.0);
        let arc_between_points = Arc::from(arc_between_points);
        let arc = VItem::<Arc>::new(PI / 2.0, 2.0).with(|arc| {
            arc.with_origin(Origin, |x| {
                x.rotate_on_axis(DVec3::NEG_Z, PI);
            })
            .shift(dvec3(2.0, 2.0, 0.0));
        });
        assert_float_absolute_eq!(
            arc.inner.center.distance_squared(arc_between_points.center),
            0.0,
            1e-10
        );
        assert_float_absolute_eq!(arc.inner.radius - arc_between_points.radius, 0.0, 1e-10);
        assert_float_absolute_eq!(arc.inner.angle - arc_between_points.angle, 0.0, 1e-10);
    }
}
