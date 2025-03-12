use super::{AnimSchedule, Animation, EvalDynamic, Rabject, ToEvaluator};
use crate::{interpolate::Interpolatable, utils::rate_functions::smooth};

pub trait TransformRequirement: Alignable + Interpolatable + Clone {}
impl<T: Alignable + Interpolatable + Clone> TransformRequirement for T {}

pub trait TransformAnim<T: TransformRequirement + 'static> {
    fn transform_between(&self, dst: T) -> Animation<T>;
}

pub trait TransformAnimSchedule<'r, 't, T: TransformRequirement + 'static> {
    fn transform<F: Fn(&mut T)>(&'r mut self, f: F) -> AnimSchedule<'r, 't, T>;
    fn transform_from(&'r mut self, s: impl Into<T>) -> AnimSchedule<'r, 't, T>;
    fn transform_to(&'r mut self, d: impl Into<T>) -> AnimSchedule<'r, 't, T>;
}

impl<T: TransformRequirement + 'static> TransformAnim<T> for T {
    fn transform_between(&self, dst: T) -> Animation<T> {
        Animation::from_evaluator(Transform::new(self.clone(), dst).to_evaluator())
            .with_rate_func(smooth)
    }
}

impl<'r, 't, T: TransformRequirement + 'static> TransformAnimSchedule<'r, 't, T>
    for Rabject<'t, T>
{
    /// Play an animation interpolates from the given src to current rabject
    fn transform_from(&'r mut self, src: impl Into<T>) -> AnimSchedule<'r, 't, T> {
        let src = src.into();
        AnimSchedule::new(self, src.transform_between(self.data.clone()))
    }

    /// Play an animation interpolates current rabject with a given transform func
    fn transform<F: Fn(&mut T)>(&'r mut self, f: F) -> AnimSchedule<'r, 't, T> {
        let mut dst = self.data.clone();
        (f)(&mut dst);
        AnimSchedule::new(self, self.data.transform_between(dst))
    }

    /// Play an animation interpolates from the given src to current rabject
    fn transform_to(&'r mut self, dst: impl Into<T>) -> AnimSchedule<'r, 't, T> {
        let dst = dst.into();
        AnimSchedule::new(self, self.data.transform_between(dst))
    }
}

/// A transform animation func
pub struct Transform<T: TransformRequirement> {
    src: T,
    dst: T,
    aligned_src: T,
    aligned_dst: T,
}

/// A trait for aligning two items
///
/// Alignment is actually the meaning of preparation for interpolation.
///
/// For example, if we want to interpolate two VItems, we need to
/// align all their inner components like `ComponentVec<VPoint>` to the same length.
pub trait Alignable {
    fn is_aligned(&self, other: &Self) -> bool;
    fn align_with(&mut self, other: &mut Self);
}

impl<T: TransformRequirement> Transform<T> {
    pub fn new(src: T, dst: T) -> Self {
        let mut aligned_src = src.clone();
        let mut aligned_dst = dst.clone();
        if !aligned_src.is_aligned(&aligned_dst) {
            aligned_src.align_with(&mut aligned_dst);
        }
        Self {
            src,
            dst,
            aligned_src,
            aligned_dst,
        }
    }
}

impl<T: TransformRequirement> EvalDynamic<T> for Transform<T> {
    fn eval_alpha(&self, alpha: f32) -> T {
        if alpha == 0.0 {
            self.src.clone()
        } else if 0.0 < alpha && alpha < 1.0 {
            self.aligned_src.lerp(&self.aligned_dst, alpha)
        } else if alpha == 1.0 {
            self.dst.clone()
        } else {
            unreachable!()
        }
    }
}
