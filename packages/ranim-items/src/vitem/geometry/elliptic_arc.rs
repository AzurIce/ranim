use std::f64::consts::TAU;

use ranim_core::{
    components::vpoint::VPointVec,
    core_item::vitem::Basis2d,
    glam::{DVec2, DVec3},
    traits::{Aabb, RotateTransform, ShiftTransform},
};

use crate::vitem::{VItem, VPath};
use crate::vitem::geometry::{Arc, Circle, Ellipse};

/// An elliptic arc.
#[derive(Debug, Clone, ranim_macros::Interpolatable)]
pub struct EllipticArc {
    /// Basis
    pub basis: Basis2d,
    /// Center
    pub center: DVec3,
    /// Semi-axes in the x and y directions
    pub radius: DVec2,
    /// Start angle (measured by the theta parameter in parametric equation of the ellipse) in radians
    pub start_angle: f64,
    /// Span angle in radians
    pub angle: f64,
}

impl VItem<EllipticArc> {
    /// Creates a new elliptic arc.
    pub fn new(start_angle: f64, angle: f64, radius: DVec2) -> Self {
        Self::new_with(EllipticArc {
            basis: Basis2d::default(),
            center: DVec3::ZERO,
            radius,
            start_angle,
            angle,
        })
    }
}

impl EllipticArc {

    fn generate_vpoints(&self) -> Vec<DVec3> {
        const NUM_SEGMENTS: usize = 8;
        let len = 2 * NUM_SEGMENTS + 1;

        let &EllipticArc {
            basis,
            center,
            radius,
            start_angle,
            angle,
        } = self;

        let (u, v) = basis.uv();
        let DVec2 { x: rx, y: ry } = radius;
        let mut vpoints = (0..len)
            .map(|i| i as f64 / NUM_SEGMENTS as f64 / 2. * angle + start_angle)
            .map(|theta| {
                let (mut x, mut y) = (theta.cos(), theta.sin());
                if x.abs() < 1.8e-7 {
                    x = 0.;
                }
                if y.abs() < 1.8e-7 {
                    y = 0.;
                }
                x * rx * u + y * ry * v
            })
            .collect::<Vec<_>>();

        let k = (angle / NUM_SEGMENTS as f64 / 2.).cos();
        vpoints.iter_mut().skip(1).step_by(2).for_each(|p| *p /= k);
        vpoints.shift(center);
        vpoints
    }
}

impl From<Arc> for EllipticArc {
    fn from(value: Arc) -> Self {
        let Arc {
            normal,
            start_dir,
            center,
            radius,
            angle,
        } = value;
        let v_dir = normal.cross(start_dir);
        Self {
            basis: Basis2d::new(start_dir, v_dir),
            center,
            radius: DVec2::splat(radius),
            start_angle: 0.,
            angle,
        }
    }
}

impl From<Circle> for EllipticArc {
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
            start_angle: 0.,
            angle: TAU,
        }
    }
}

impl From<Ellipse> for EllipticArc {
    fn from(value: Ellipse) -> Self {
        let Ellipse {
            basis,
            center,
            radius,
        } = value;
        Self {
            basis,
            center,
            radius,
            start_angle: 0.,
            angle: TAU,
        }
    }
}

impl VPath for EllipticArc {
    fn normal(&self) -> DVec3 {
        self.basis.normal()
    }
    fn build_vpoint_vec(&self) -> VPointVec {
        VPointVec(self.generate_vpoints())
    }
}

impl Aabb for EllipticArc {
    fn aabb(&self) -> [DVec3; 2] {
        VPointVec(self.generate_vpoints()).aabb()
    }
}

impl ShiftTransform for EllipticArc {
    fn shift(&mut self, shift: DVec3) -> &mut Self {
        self.center += shift;
        self
    }
}

impl RotateTransform for EllipticArc {
    fn rotate_on_axis(&mut self, axis: DVec3, angle: f64) -> &mut Self {
        self.basis.rotate_on_axis(axis, angle);
        self.center.rotate_on_axis(axis, angle);
        self
    }
}
