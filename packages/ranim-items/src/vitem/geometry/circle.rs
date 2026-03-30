use std::f64::consts::PI;

use glam::DVec3;
use ranim_core::anchor::AabbPoint;
use ranim_core::{
    anchor::{Aabb, Anchor},
    components::vpoint::VPointVec,
    core_item::vitem::Basis2d,
    glam,
    traits::{RotateTransform, ScaleTransform, ShiftTransform},
};

use super::Arc;
use crate::vitem::{VItem, VPath};

// MARK: ### Circle ###
/// An circle
#[derive(Clone, Debug, ranim_macros::Interpolatable)]
pub struct Circle {
    /// Basis
    pub basis: Basis2d,
    /// Center
    pub center: DVec3,
    /// Radius
    pub radius: f64,
}

impl VItem<Circle> {
    /// Constructor
    pub fn new(radius: f64) -> Self {
        Self::new_with(Circle {
            basis: Basis2d::default(),
            center: DVec3::ZERO,
            radius,
        })
    }
    /// Scale the circle by the given scale, with the given anchor as the center.
    pub fn scale(&mut self, scale: f64) -> &mut Self {
        self.scale_by_anchor(scale, AabbPoint::CENTER)
    }
    /// Scale the circle by the given scale, with the given anchor as the center.
    pub fn scale_by_anchor<T>(&mut self, scale: f64, anchor: T) -> &mut Self
    where
        T: Anchor<Self>,
    {
        let point = anchor.locate_on(self);
        self.with_inner_mut(|circle| {
            circle.radius *= scale;
            circle.center
                .shift(-point)
                .scale(DVec3::splat(scale))
                .shift(point);
        });
        self
    }
}

impl Circle {
    /// Constructor
    pub fn new(radius: f64) -> Self {
        Self {
            basis: Basis2d::default(),
            center: DVec3::ZERO,
            radius,
        }
    }
}

// MARK: Traits impl
impl Aabb for Circle {
    fn aabb(&self) -> [DVec3; 2] {
        let (u, v) = self.basis.uv();
        let r = self.radius * (u + v);
        [self.center + r, self.center - r].aabb()
    }
}

impl ShiftTransform for Circle {
    fn shift(&mut self, shift: DVec3) -> &mut Self {
        self.center.shift(shift);
        self
    }
}

impl RotateTransform for Circle {
    fn rotate_on_axis(&mut self, axis: DVec3, angle: f64) -> &mut Self {
        self.center.rotate_on_axis(axis, angle);
        self.basis.rotate_on_axis(axis, angle);
        self
    }
}

impl From<Circle> for Arc {
    fn from(value: Circle) -> Self {
        let Circle {
            basis,
            center,
            radius,
        } = value;
        Self {
            normal: basis.normal(),
            start_dir: basis.u(),
            center,
            radius,
            angle: 2.0 * PI,
        }
    }
}

impl VPath for Circle {
    fn normal(&self) -> DVec3 {
        self.basis.normal()
    }
    fn build_vpoint_vec(&self) -> VPointVec {
        Arc::from(self.clone()).build_vpoint_vec()
    }
}
