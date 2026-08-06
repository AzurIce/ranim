//! Camera frame animations.

use ranim_core::{
    animation::Eval, core_item::camera_frame::CameraFrame, glam::DVec3,
};

use crate::pure::{Pure, PureEval};

// MARK: Anim Trait
/// The methods to create animations for [`CameraFrame`].
pub trait CameraFrameAnim {
    /// Create an orbit animation that rotates the camera around `target`
    /// by `total_angle` radians in the XY plane (Z-up).
    ///
    /// The camera's current position is used to derive the spherical
    /// coordinates (distance, elevation) which are kept constant during the orbit.
    ///
    /// # Example
    /// ```ignore
    /// use std::f64::consts::TAU;
    ///
    /// let mut cam = CameraFrame::from_spherical(phi, theta, distance);
    /// r.play(
    ///     cam.orbit(DVec3::ZERO, TAU)
    ///         .with_duration(8.0)
    ///         .with_rate_func(linear),
    /// );
    /// ```
    fn orbit(&mut self, target: DVec3, total_angle: f64) -> Pure<Orbit>;
}

impl CameraFrameAnim for CameraFrame {
    fn orbit(&mut self, target: DVec3, total_angle: f64) -> Pure<Orbit> {
        let offset = self.pos - target;
        let distance = offset.length();
        let phi = if distance > 0.0 {
            (offset.z / distance).acos()
        } else {
            0.0
        };
        let theta0 = offset.y.atan2(offset.x);

        Pure(Orbit {
            src: self.clone(),
            target,
            distance,
            phi,
            theta0,
            total_angle,
        })
        .apply_to(self)
    }
}

// MARK: Impl
/// An orbit animation rotating the camera around a target.
pub struct Orbit {
    /// The camera state at the start of the orbit.
    pub src: CameraFrame,
    /// The orbit target.
    pub target: DVec3,
    /// Distance from the target.
    pub distance: f64,
    /// Elevation angle kept constant during the orbit.
    pub phi: f64,
    /// Initial azimuth angle.
    pub theta0: f64,
    /// Total angle to rotate, in radians.
    pub total_angle: f64,
}

impl PureEval for Orbit {
    type Output = CameraFrame;

    fn eval_alpha(&self, alpha: f64) -> Self::Output {
        let theta = self.theta0 + self.total_angle * alpha;
        let mut result = self.src.clone();
        result.set_spherical(self.phi, theta, self.distance, self.target);
        result
    }
}
