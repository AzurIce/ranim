use itertools::Itertools;

use super::{AnimationSpan, EvalDynamic, ToEvaluator};
use crate::{
    traits::{Alignable, Interpolatable},
    utils::rate_functions::smooth,
};

pub trait TransformRequirement: Alignable + Interpolatable + Clone {}
impl<T: Alignable + Interpolatable + Clone> TransformRequirement for T {}

pub trait GroupTransformAnim<T: TransformRequirement + 'static> {
    type Output;
    fn transform<F: Fn(&mut Self)>(self, func: F) -> Self::Output;
    fn transform_from<I: IntoIterator<Item = T>>(self, srcs: I) -> Self::Output;
    fn transform_to<I: IntoIterator<Item = T>>(self, dsts: I) -> Self::Output;
}

impl<T: TransformRequirement + 'static> GroupTransformAnim<T> for Vec<T> {
    type Output = Vec<AnimationSpan<T>>;
    fn transform<F: Fn(&mut Self)>(self, func: F) -> Vec<AnimationSpan<T>> {
        let mut dsts = self.clone();
        (func)(&mut dsts);
        self.into_iter()
            .zip(dsts)
            .map(|(x, dst)| x.transform_to(dst))
            .collect()
    }
    fn transform_from<I: IntoIterator<Item = T>>(self, srcs: I) -> Vec<AnimationSpan<T>> {
        self.into_iter()
            .zip(srcs)
            .map(|(x, src)| x.transform_from(src))
            .collect()
    }
    fn transform_to<I: IntoIterator<Item = T>>(self, dsts: I) -> Vec<AnimationSpan<T>> {
        self.into_iter()
            .zip(dsts)
            .map(|(x, dst)| x.transform_to(dst))
            .collect()
    }
}

impl<T: TransformRequirement + 'static, const N: usize> GroupTransformAnim<T> for [T; N] {
    type Output = [AnimationSpan<T>; N];
    fn transform<F: Fn(&mut Self)>(self, func: F) -> Self::Output {
        let mut dsts = self.clone();
        (func)(&mut dsts);
        self.into_iter()
            .zip(dsts)
            .map(|(x, dst)| x.transform_to(dst))
            .collect_array()
            .unwrap()
    }
    fn transform_from<I: IntoIterator<Item = T>>(self, srcs: I) -> Self::Output {
        self.into_iter()
            .zip(srcs)
            .map(|(x, src)| x.transform_from(src))
            .collect_array()
            .unwrap()
    }
    fn transform_to<I: IntoIterator<Item = T>>(self, dsts: I) -> Self::Output {
        self.into_iter()
            .zip(dsts)
            .map(|(x, dst)| x.transform_to(dst))
            .collect_array()
            .unwrap()
    }
}

pub trait TransformAnim<T: TransformRequirement + 'static> {
    fn transform<F: Fn(&mut T)>(self, f: F) -> AnimationSpan<T>;
    fn transform_from(self, src: impl Into<T>) -> AnimationSpan<T>;
    fn transform_to(self, dst: impl Into<T>) -> AnimationSpan<T>;
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
