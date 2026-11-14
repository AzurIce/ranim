//! Camera orbit parameters and their prepared execution data.

use ranim_core::{
    animation::{definition::AnimRecipe, eval::Eval},
    glam::DVec3,
};
use ranim_items::camera::CameraFrame;

/// Orbit around a target in the XY plane (Z-up).
#[derive(Clone, Copy, Debug, ranim_macros::Animation)]
pub struct Orbit {
    /// Orbit target.
    pub target: DVec3,
    /// Total rotation, in radians.
    pub angle: f64,
}

impl Orbit {
    /// Describe an orbit around `target` through `angle` radians.
    pub fn new(target: DVec3, angle: f64) -> Self {
        Self { target, angle }
    }
}

/// Prepared camera orbit with derived spherical coordinates.
pub struct OrbitEval {
    src: CameraFrame,
    target: DVec3,
    distance: f64,
    phi: f64,
    theta0: f64,
    angle: f64,
}

impl AnimRecipe<CameraFrame> for Orbit {
    type Animation = OrbitEval;
    fn build(&self, input: &CameraFrame) -> Self::Animation {
        let offset = input.pos - self.target;
        let distance = offset.length();
        let phi = if distance > 0.0 {
            (offset.z / distance).acos()
        } else {
            0.0
        };
        OrbitEval {
            src: input.clone(),
            target: self.target,
            distance,
            phi,
            theta0: offset.y.atan2(offset.x),
            angle: self.angle,
        }
    }
}

impl Eval for OrbitEval {
    type Output = CameraFrame;
    fn eval_alpha(&self, alpha: f64) -> CameraFrame {
        let mut result = self.src.clone();
        result.set_spherical(
            self.phi,
            self.theta0 + self.angle * alpha,
            self.distance,
            self.target,
        );
        result
    }
}
