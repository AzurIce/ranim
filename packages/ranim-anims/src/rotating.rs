//! Rotation parameters, bound to an input through `ItemAnimExt`.

use ranim_core::{
    animation::{AnimRecipe, AnimWriteBack, eval::Eval, node::AnimNode},
    glam::DVec3,
    traits::{
        AabbPoint, Locate,
        transform::{RotateTransform, ShiftTransformExt},
    },
};

/// Item capabilities needed for rotation around a located point.
pub trait RotatingRequirement: RotateTransform + ShiftTransformExt + Clone {}
impl<T: RotateTransform + ShiftTransformExt + Clone> RotatingRequirement for T {}

/// Rotation inputs. The pivot is resolved against the original input at build time.
#[derive(Clone, ranim_macros::Animation)]
pub struct Rotate<A = AabbPoint> {
    /// Rotation angle, in radians.
    pub angle: f64,
    /// Rotation axis.
    pub axis: DVec3,
    /// Pivot locator, such as `AabbPoint::CENTER` or an explicit `DVec3`.
    pub point: A,
}

impl Rotate {
    /// Rotate around the input's bounding-box center.
    pub fn new(angle: f64, axis: DVec3) -> Self {
        Self {
            angle,
            axis,
            point: AabbPoint::CENTER,
        }
    }
}

impl<A> Rotate<A> {
    /// Rotate around an explicit point or locator.
    pub fn at(angle: f64, axis: DVec3, point: A) -> Self {
        Self { angle, axis, point }
    }
}

/// Prepared rotation with a resolved pivot.
pub struct RotateEval<T> {
    src: T,
    angle: f64,
    axis: DVec3,
    point: DVec3,
}

impl<A> Rotate<A> {
    fn prepare<T: RotatingRequirement>(&self, input: &T) -> RotateEval<T>
    where
        A: Locate<T>,
    {
        RotateEval {
            src: input.clone(),
            angle: self.angle,
            axis: self.axis,
            point: self.point.locate(input),
        }
    }
}

impl<T: RotatingRequirement + Send + Sync + 'static, A: Locate<T>> AnimRecipe<T> for Rotate<A> {
    fn build(&self, input: &T) -> AnimNode {
        AnimNode::from_eval(self.prepare(input), 1.0)
    }
}

impl<T: RotatingRequirement + Send + Sync + 'static, A: Locate<T>> AnimWriteBack<T> for Rotate<A> {
    fn end_value(&self, input: &T) -> T {
        self.prepare(input).eval_alpha(1.0)
    }
}

impl<T: RotatingRequirement> Eval for RotateEval<T> {
    type Output = T;
    fn eval_alpha(&self, alpha: f64) -> T {
        let mut result = self.src.clone();
        result.with_origin(self.point, |x| {
            x.rotate_on_axis(self.axis, self.angle * alpha);
        });
        result
    }
}
