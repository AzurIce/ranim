use ranim_core::{
    animation::Eval,
    glam::DVec3,
    traits::{Aabb, AabbPoint, Locate, RotateTransform, ShiftTransformExt},
};

use crate::pure::{Pure, PureEval};

// MARK: Require Trait
/// The requirement of [`RotatingAnimation`]
pub trait RotatingRequirement: RotateTransform + ShiftTransformExt + Clone {}
impl<T: RotateTransform + ShiftTransformExt + Clone> RotatingRequirement for T {}

// MARK: Anim Trait
/// The methods to create rotation animations for `T` that satisfies [`RotatingRequirement`]
pub trait RotatingAnim: RotatingRequirement + Sized + 'static {
    /// Rotate by a given angle about a given axis at center.
    fn rotating(&mut self, angle: f64, axis: DVec3) -> Pure<RotatingAnimation<Self>>
    where
        Self: Aabb,
    {
        self.rotating_at(angle, axis, AabbPoint::CENTER)
    }

    /// Rotate by a given angle about a given axis at the given anchor.
    fn rotating_at<A: Locate<Self>>(
        &mut self,
        angle: f64,
        axis: DVec3,
        anchor: A,
    ) -> Pure<RotatingAnimation<Self>> {
        Pure(RotatingAnimation::new(
            self.clone(),
            angle,
            axis,
            anchor.locate(self),
        ))
        .apply_to(self)
    }
}

impl<T: RotatingRequirement + 'static> RotatingAnim for T {}

// MARK: Impl

/// Rotation animation.
///
/// Unlike [`Morph`](crate::pure::morph::Morph) which linearly interpolates between
/// start and end states, this animation applies incremental rotation at each frame,
/// producing a true circular arc motion.
pub struct RotatingAnimation<T: RotatingRequirement> {
    src: T,
    angle: f64,
    axis: DVec3,
    point: DVec3,
}

impl<T: RotatingRequirement> RotatingAnimation<T> {
    /// Constructor
    pub fn new(src: T, angle: f64, axis: DVec3, point: DVec3) -> Self {
        Self {
            src,
            angle,
            axis,
            point,
        }
    }
}

impl<T: RotatingRequirement> PureEval for RotatingAnimation<T> {
    type Output = T;

    fn eval_alpha(&self, alpha: f64) -> Self::Output {
        let mut result = self.src.clone();
        result.with_origin(self.point, |x| {
            x.rotate_on_axis(self.axis, self.angle * alpha);
        });
        result
    }
}
