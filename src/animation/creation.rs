use super::{AnimSchedule, Animation, EvalDynamic, ToEvaluator};
use crate::items::Rabject;
use crate::prelude::Interpolatable;
use crate::utils::rate_functions::smooth;
use color::{AlphaColor, Srgb};
use log::warn;
use std::ops::Range;

// MARK: Creation

pub trait CreationRequirement: Clone + Partial + Empty + Interpolatable {}
impl<T: Clone + Partial + Empty + Interpolatable> CreationRequirement for T {}

pub trait CreationAnim<T: CreationRequirement + 'static> {
    fn create(&self) -> Animation<T>;
    fn uncreate(&self) -> Animation<T>;
}

pub trait CreationAnimSchedule<'r, 't, T: CreationRequirement + 'static> {
    fn create(&'r mut self) -> AnimSchedule<'r, 't, T>;
    fn uncreate(&'r mut self) -> AnimSchedule<'r, 't, T>;
}

impl<T: CreationRequirement + 'static> CreationAnim<T> for T {
    fn create(&self) -> Animation<T> {
        Animation::from_evaluator(Create::new(self.clone()).to_evaluator()).with_rate_func(smooth)
    }
    fn uncreate(&self) -> Animation<T> {
        Animation::from_evaluator(UnCreate::new(self.clone()).to_evaluator()).with_rate_func(smooth)
    }
}

impl<'r, 't, T: CreationRequirement + 'static> CreationAnimSchedule<'r, 't, T> for Rabject<'t, T> {
    fn create(&'r mut self) -> AnimSchedule<'r, 't, T> {
        AnimSchedule::new(self, self.data.create())
    }
    fn uncreate(&'r mut self) -> AnimSchedule<'r, 't, T> {
        AnimSchedule::new(self, self.data.uncreate())
    }
}

// MARK: Writing
pub trait WritingRequirement: CreationRequirement + Stroke + Fill {}
impl<T: CreationRequirement + Stroke + Fill> WritingRequirement for T {}

pub trait WritingAnim<T: WritingRequirement + 'static> {
    fn write(&self) -> Animation<T>;
    fn unwrite(&self) -> Animation<T>;
}

pub trait WritingAnimSchedule<'r, 't, T: WritingRequirement + 'static> {
    fn write(&'r mut self) -> AnimSchedule<'r, 't, T>;
    fn unwrite(&'r mut self) -> AnimSchedule<'r, 't, T>;
}

impl<T: WritingRequirement + 'static> WritingAnim<T> for T {
    fn write(&self) -> Animation<T> {
        Animation::from_evaluator(Write::new(self.clone()).to_evaluator()).with_rate_func(smooth)
    }
    fn unwrite(&self) -> Animation<T> {
        Animation::from_evaluator(Unwrite::new(self.clone()).to_evaluator()).with_rate_func(smooth)
    }
}

impl<'r, 't, T: WritingRequirement + 'static> WritingAnimSchedule<'r, 't, T> for Rabject<'t, T> {
    fn write(&'r mut self) -> AnimSchedule<'r, 't, T> {
        AnimSchedule::new(self, self.data.write())
    }
    fn unwrite(&'r mut self) -> AnimSchedule<'r, 't, T> {
        AnimSchedule::new(self, self.data.unwrite())
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
    fn eval_alpha(&self, alpha: f32) -> T {
        if alpha == 0.0 {
            T::empty()
        } else if 0.0 < alpha && alpha < 1.0 {
            self.original.get_partial(0.0..alpha)
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
    fn eval_alpha(&self, mut alpha: f32) -> T {
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
            self.original.get_partial(0.0..1.0 - alpha)
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
    create_anim: Create<T>,
}

impl<T: WritingRequirement> Write<T> {
    fn new(target: T) -> Self {
        let mut outline = target.clone();
        outline
            .set_fill_opacity(0.0)
            .set_stroke_width(1.0)
            .set_stroke_color(target.fill_color())
            .set_stroke_opacity(1.0);
        Self {
            original: target,
            create_anim: Create::new(outline.clone()),
            outline,
        }
    }
}

impl<T: WritingRequirement> EvalDynamic<T> for Write<T> {
    fn eval_alpha(&self, alpha: f32) -> T {
        let alpha = alpha * 2.0;
        if (0.0..=1.0).contains(&alpha) {
            self.create_anim.eval_alpha(alpha)
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
    uncreate_anim: UnCreate<T>,
}

impl<T: WritingRequirement> Unwrite<T> {
    fn new(target: T) -> Self {
        let mut outline = target.clone();
        outline
            .set_fill_opacity(0.0)
            .set_stroke_width(1.0)
            .set_stroke_color(target.fill_color())
            .set_stroke_opacity(1.0);
        Self {
            original: target,
            uncreate_anim: UnCreate::new(outline.clone()),
            outline,
        }
    }
}

impl<T: WritingRequirement> EvalDynamic<T> for Unwrite<T> {
    fn eval_alpha(&self, alpha: f32) -> T {
        let alpha = alpha * 2.0;
        if (1.0..=2.0).contains(&alpha) {
            self.uncreate_anim.eval_alpha(alpha - 1.0)
        } else if alpha == 0.0 {
            self.original.clone()
        } else if (0.0..1.0).contains(&alpha) {
            self.original.lerp(&self.outline, alpha)
        } else if alpha == 1.0 {
            self.outline.clone()
        } else {
            panic!("the alpha is out of range: {}", alpha);
        }
    }
}

// MARK: Traits

pub trait Partial {
    fn get_partial(&self, range: Range<f32>) -> Self;
}

pub trait Empty {
    fn empty() -> Self;
}

pub trait Fill {
    fn set_fill_opacity(&mut self, opacity: f32) -> &mut Self;
    fn fill_color(&self) -> AlphaColor<Srgb>;
    fn set_fill_color(&mut self, color: AlphaColor<Srgb>) -> &mut Self;
}

pub trait Stroke {
    fn set_stroke_width(&mut self, width: f32) -> &mut Self;
    fn set_stroke_color(&mut self, color: AlphaColor<Srgb>) -> &mut Self;
    fn set_stroke_opacity(&mut self, opacity: f32) -> &mut Self;
}

pub trait Color: Fill + Stroke {
    fn set_color(&mut self, color: AlphaColor<Srgb>) -> &mut Self {
        self.set_fill_color(color);
        self.set_stroke_color(color);
        self
    }
}

impl<T: Fill + Stroke> Color for T {}
