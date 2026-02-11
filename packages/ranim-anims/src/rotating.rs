use ranim_core::{
    animation::{AnimationCell, Eval},
    glam::DVec3,
    traits::{Aabb, AabbPoint, Locate, Rotate},
    utils::rate_functions::smooth,
};

// MARK: Require Trait
/// The requirement of [`RotatingAnimation`]
pub trait RotatingRequirement: Rotate + Clone {}
impl<T: Rotate + Clone> RotatingRequirement for T {}

// MARK: Anim Trait
/// The methods to create rotation animations for `T` that satisfies [`RotatingRequirement`]
pub trait RotatingAnim: RotatingRequirement + Sized + 'static {
    /// Rotate by a given angle about a given axis at center.
    fn rotating(&mut self, angle: f64, axis: DVec3) -> AnimationCell<Self>
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
    ) -> AnimationCell<Self> {
        RotatingAnimation::new(self.clone(), angle, axis, anchor.locate(self))
            .into_animation_cell()
            .with_rate_func(smooth)
            .apply_to(self)
    }
}

impl<T: RotatingRequirement + 'static> RotatingAnim for T {}

// MARK: Impl

/// Rotation animation.
///
/// Unlike [`Transform`](crate::transform::Transform) which linearly interpolates between
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

impl<T: RotatingRequirement> Eval<T> for RotatingAnimation<T> {
    fn eval_alpha(&self, alpha: f64) -> T {
        let mut result = self.src.clone();
        result.rotate_at_point(self.angle * alpha, self.axis, self.point);
        result
    }
}
