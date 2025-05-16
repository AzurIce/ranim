use super::{AnimSchedule, AnimationSpan, EvalDynamic, PinnedItem, ToEvaluator};
use crate::{
    items::group::Group,
    traits::{Alignable, Interpolatable},
    utils::rate_functions::smooth,
};

pub trait TransformRequirement: Alignable + Interpolatable + Clone {}
impl<T: Alignable + Interpolatable + Clone> TransformRequirement for T {}

pub trait GroupTransformAnim<T: TransformRequirement + 'static> {
    fn transform<F: Fn(&mut Self)>(self, f: F) -> Group<AnimationSpan<T>>;
    fn transform_from<E: Into<T>>(self, s: Group<E>) -> Group<AnimationSpan<T>>;
    fn transform_to<E: Into<T>>(self, d: Group<E>) -> Group<AnimationSpan<T>>;
}

impl<T: TransformRequirement + 'static> GroupTransformAnim<T> for Group<T> {
    fn transform<F: Fn(&mut Self)>(self, f: F) -> Group<AnimationSpan<T>> {
        let mut dsts = self.clone();
        (f)(&mut dsts);
        self.into_iter()
            .zip(dsts)
            .map(|(x, dst)| x.transform_to(dst))
            .collect()
    }
    fn transform_from<E: Into<T>>(self, s: Group<E>) -> Group<AnimationSpan<T>> {
        let ss: Group<T> = s.into_iter().map(|x| x.into()).collect();
        self.into_iter()
            .zip(ss)
            .map(|(x, s)| x.transform_from(s))
            .collect()
    }
    fn transform_to<E: Into<T>>(self, d: Group<E>) -> Group<AnimationSpan<T>> {
        let dd: Group<T> = d.into_iter().map(|x| x.into()).collect();
        self.into_iter()
            .zip(dd)
            .map(|(x, d)| x.transform_to(d))
            .collect()
    }
}

pub trait TransformAnim<T: TransformRequirement + 'static> {
    fn transform<F: Fn(&mut T)>(self, f: F) -> AnimationSpan<T>;
    fn transform_from(self, src: impl Into<T>) -> AnimationSpan<T>;
    fn transform_to(self, dst: impl Into<T>) -> AnimationSpan<T>;
}

pub trait TransformAnimSchedule<T: TransformRequirement + 'static> {
    fn transform<F: Fn(&mut T)>(self, f: F) -> AnimSchedule<T>;
    fn transform_from(self, s: impl Into<T>) -> AnimSchedule<T>;
    fn transform_to(self, d: impl Into<T>) -> AnimSchedule<T>;
}

impl<T: TransformRequirement + 'static> TransformAnim<T> for T {
    fn transform<F: Fn(&mut T)>(self, f: F) -> AnimationSpan<T> {
        let mut dst = self.clone();
        (f)(&mut dst);
        AnimationSpan::from_evaluator(Transform::new(self.clone(), dst).to_evaluator())
            .with_rate_func(smooth)
    }
    fn transform_from(self, s: impl Into<T>) -> AnimationSpan<T> {
        AnimationSpan::from_evaluator(Transform::new(s.into(), self.clone()).to_evaluator())
            .with_rate_func(smooth)
    }
    fn transform_to(self, d: impl Into<T>) -> AnimationSpan<T> {
        AnimationSpan::from_evaluator(Transform::new(self.clone(), d.into()).to_evaluator())
            .with_rate_func(smooth)
    }
}

/// A transform animation func
pub struct Transform<T: TransformRequirement> {
    src: T,
    dst: T,
    aligned_src: T,
    aligned_dst: T,
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
    fn eval_alpha(&self, alpha: f64) -> T {
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
