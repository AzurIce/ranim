use super::{AnimationSpan, EvalDynamic, ToEvaluator};
use crate::items::vitem::DEFAULT_STROKE_WIDTH;
use crate::traits::{Empty, Fill, Interpolatable, Partial, Stroke};
use crate::utils::rate_functions::smooth;
use log::warn;

// MARK: Creation

pub trait CreationRequirement: Clone + Partial + Empty + Interpolatable {}
impl<T: Clone + Partial + Empty + Interpolatable> CreationRequirement for T {}

pub trait CreationAnim<T: CreationRequirement + 'static> {
    fn create(self) -> AnimationSpan<T>;
    fn uncreate(self) -> AnimationSpan<T>;
}

impl<T: CreationRequirement + 'static> CreationAnim<T> for T {
    fn create(self) -> AnimationSpan<T> {
        AnimationSpan::from_evaluator(Create::new(self).to_evaluator()).with_rate_func(smooth)
    }
    fn uncreate(self) -> AnimationSpan<T> {
        AnimationSpan::from_evaluator(UnCreate::new(self).to_evaluator()).with_rate_func(smooth)
    }
}

// MARK: Writing
pub trait WritingRequirement: CreationRequirement + Stroke + Fill {}
impl<T: CreationRequirement + Stroke + Fill> WritingRequirement for T {}

pub trait WritingAnim<T: WritingRequirement + 'static> {
    fn write(self) -> AnimationSpan<T>;
    fn unwrite(self) -> AnimationSpan<T>;
}

impl<T: WritingRequirement + 'static> WritingAnim<T> for T {
    fn write(self) -> AnimationSpan<T> {
        AnimationSpan::from_evaluator(Write::new(self).to_evaluator()).with_rate_func(smooth)
    }
    fn unwrite(self) -> AnimationSpan<T> {
        AnimationSpan::from_evaluator(Unwrite::new(self).to_evaluator()).with_rate_func(smooth)
    }
}

// ---------------------------------------------------- //

// MARK: Impl

pub struct Create<T: CreationRequirement> {
    pub original: T,
}

impl<T: CreationRequirement> Create<T> {
    fn new(target: T) -> Self {
        Self { original: target }
    }
}

impl<T: CreationRequirement> EvalDynamic<T> for Create<T> {
    fn eval_alpha(&self, alpha: f64) -> T {
        if alpha == 0.0 {
            T::empty()
        } else if 0.0 < alpha && alpha < 1.0 {
            self.original.get_partial_closed(0.0..alpha)
        } else if alpha == 1.0 {
            self.original.clone()
        } else {
            unreachable!()
        }
    }
}

pub struct UnCreate<T: CreationRequirement> {
    pub original: T,
}

impl<T: CreationRequirement> UnCreate<T> {
    fn new(target: T) -> Self {
        Self { original: target }
    }
}

impl<T: CreationRequirement> EvalDynamic<T> for UnCreate<T> {
    fn eval_alpha(&self, mut alpha: f64) -> T {
        if !(0.0..=1.0).contains(&alpha) {
            warn!(
                "the alpha is out of range: {}, clampped to 0.0..=1.0",
                alpha
            );
            alpha = alpha.clamp(0.0, 1.0)
        }
        // trace!("{alpha}");
        if alpha == 0.0 {
            self.original.clone()
        } else if 0.0 < alpha && alpha < 1.0 {
            self.original.get_partial_closed(0.0..1.0 - alpha)
        } else if alpha == 1.0 {
            T::empty()
        } else {
            panic!("the alpha is out of range: {}", alpha);
        }
    }
}

/// Write
///
/// First update with partial from 0.0..0.0 to 0.0..1.0, then lerp fill_opacity to 1.0
pub struct Write<T: WritingRequirement> {
    pub(crate) original: T,
    pub(crate) outline: T,
}

impl<T: WritingRequirement> Write<T> {
    fn new(target: T) -> Self {
        let mut outline = target.clone();
        outline
            .set_fill_opacity(0.0)
            .set_stroke_width(DEFAULT_STROKE_WIDTH)
            .set_stroke_opacity(1.0);
        Self {
            original: target,
            outline,
        }
    }
}

impl<T: WritingRequirement> EvalDynamic<T> for Write<T> {
    fn eval_alpha(&self, alpha: f64) -> T {
        let alpha = alpha * 2.0;
        if (0.0..1.0).contains(&alpha) {
            self.outline.get_partial(0.0..alpha)
        } else if alpha == 1.0 {
            self.outline.clone()
        } else if (1.0..2.0).contains(&alpha) {
            self.outline.lerp(&self.original, alpha - 1.0)
        } else if alpha == 2.0 {
            self.original.clone()
        } else {
            unreachable!()
        }
    }
}

/// Unwrite
///
/// First lerp fill_opacity to 0.0, then update with partial from 0.0..1.0 to 0.0..0.0
pub struct Unwrite<T: WritingRequirement> {
    pub(crate) original: T,
    pub(crate) outline: T,
}

impl<T: WritingRequirement> Unwrite<T> {
    fn new(target: T) -> Self {
        let mut outline = target.clone();
        outline
            .set_fill_opacity(0.0)
            .set_stroke_width(DEFAULT_STROKE_WIDTH)
            .set_stroke_color(target.fill_color())
            .set_stroke_opacity(1.0);
        Self {
            original: target,
            outline,
        }
    }
}

impl<T: WritingRequirement> EvalDynamic<T> for Unwrite<T> {
    fn eval_alpha(&self, alpha: f64) -> T {
        let alpha = alpha * 2.0;
        if (0.0..1.0).contains(&alpha) {
            self.original.lerp(&self.outline, alpha)
        } else if alpha == 1.0 {
            self.outline.clone()
        } else if (1.0..2.0).contains(&alpha) {
            self.outline.get_partial(0.0..2.0 - alpha)
        } else if alpha == 2.0 {
            T::empty()
        } else if alpha == 0.0 {
            self.original.clone()
        } else {
            panic!("the alpha is out of range: {}", alpha);
        }
    }
}
