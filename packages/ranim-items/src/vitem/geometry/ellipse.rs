use ranim_core::{
    components::vpoint::VPointVec,
    core_item::vitem::Basis2d,
    glam::{DVec2, DVec3},
    traits::{Aabb, RotateTransform, ShiftTransform},
};

use crate::vitem::{VItem, VPath};
use crate::vitem::geometry::{Circle, EllipticArc};

/// An ellipse.
#[derive(Clone, Debug, ranim_macros::Interpolatable)]
pub struct Ellipse {
    /// Basis
    pub basis: Basis2d,
    /// Center
    pub center: DVec3,
    /// Semi-axes in x and y directions
    pub radius: DVec2,
}

impl VItem<Ellipse> {
    /// Creates a new ellipse.
    pub fn new(radius: DVec2) -> Self {
        Self::new_with(Ellipse {
            basis: Basis2d::default(),
            center: DVec3::ZERO,
            radius,
        })
    }
}

impl From<Circle> for Ellipse {
    fn from(value: Circle) -> Self {
        let Circle {
            basis,
            center,
            radius,
        } = value;
        Self {
            basis,
            center,
            radius: DVec2::splat(radius),
        }
    }
}

impl VPath for Ellipse {
    fn normal(&self) -> DVec3 {
        self.basis.normal()
    }
    fn build_vpoint_vec(&self) -> VPointVec {
        EllipticArc::from(self.clone()).build_vpoint_vec()
    }
}

impl Aabb for Ellipse {
    fn aabb(&self) -> [DVec3; 2] {
        let center = self.center;
        let (u, v) = self.basis.uv();
        let DVec2 { x: rx, y: ry } = self.radius;
        let r = u * rx + v * ry;
        [center - r, center + r].aabb()
    }
}

impl ShiftTransform for Ellipse {
    fn shift(&mut self, offset: DVec3) -> &mut Self {
        self.center += offset;
        self
    }
}

impl RotateTransform for Ellipse {
    fn rotate_on_axis(&mut self, axis: DVec3, angle: f64) -> &mut Self {
        self.basis.rotate_on_axis(axis, angle);
        self.center.rotate_on_axis(axis, angle);
        self
    }
}
