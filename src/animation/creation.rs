use super::{AnimScheduler, EntityAnim, PureEvaluator};
use crate::items::{Entity, Rabject};
use crate::prelude::Interpolatable;
use color::{AlphaColor, Srgb};
use std::ops::Range;

// MARK: Creation

pub trait Creation: Entity + Partial + Empty + Interpolatable {}
impl<T: Entity + Partial + Empty + Interpolatable> Creation for T {}

pub trait CreationAnim<'r, 't, T: Creation + 'static> {
    fn create(&'r mut self) -> AnimScheduler<'r, 't, T, EntityAnim<T>>;
    fn uncreate(&'r mut self) -> AnimScheduler<'r, 't, T, EntityAnim<T>>;
}

impl<'r, 't, T: Creation + 'static> CreationAnim<'r, 't, T> for Rabject<'t, T> {
    fn create(&'r mut self) -> AnimScheduler<'r, 't, T, EntityAnim<T>> {
        let func = Create::new(self.data.clone());
        AnimScheduler::new(self, EntityAnim::new(self.id, self.data.clone(), func))
    }
    fn uncreate(&'r mut self) -> AnimScheduler<'r, 't, T, EntityAnim<T>> {
        let func = UnCreate::new(self.data.clone());
        AnimScheduler::new(self, EntityAnim::new(self.id, self.data.clone(), func))
    }
}

// MARK: Writing
pub trait Writing: Creation + Stroke + Fill {}
impl<T: Creation + Stroke + Fill> Writing for T {}

pub trait WritingAnim<'r, 't, T: Writing + 'static> {
    fn write(&'r mut self) -> AnimScheduler<'r, 't, T, EntityAnim<T>>;
    fn unwrite(&'r mut self) -> AnimScheduler<'r, 't, T, EntityAnim<T>>;
}

impl<'r, 't, T: Writing + 'static> WritingAnim<'r, 't, T> for Rabject<'t, T> {
    fn write(&'r mut self) -> AnimScheduler<'r, 't, T, EntityAnim<T>> {
        let func = Write::new(self.data.clone());
        AnimScheduler::new(self, EntityAnim::new(self.id, self.data.clone(), func))
    }
    fn unwrite(&'r mut self) -> AnimScheduler<'r, 't, T, EntityAnim<T>> {
        let func = Unwrite::new(self.data.clone());
        AnimScheduler::new(self, EntityAnim::new(self.id, self.data.clone(), func))
    }
}

// ---------------------------------------------------- //

pub struct Create<T: Creation> {
    pub original: T,
}

impl<T: Creation> Create<T> {
    fn new(target: T) -> Self {
        Self { original: target }
    }
}

impl<T: Creation> PureEvaluator<T> for Create<T> {
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

pub struct UnCreate<T: Creation> {
    pub original: T,
}

impl<T: Creation> UnCreate<T> {
    fn new(target: T) -> Self {
        Self { original: target }
    }
}

impl<T: Creation> PureEvaluator<T> for UnCreate<T> {
    fn eval_alpha(&self, alpha: f32) -> T {
        // trace!("{alpha}");
        if alpha == 0.0 {
            self.original.clone()
        } else if 0.0 < alpha && alpha < 1.0 {
            self.original.get_partial(0.0..1.0 - alpha)
        } else if alpha == 1.0 {
            T::empty()
        } else {
            unreachable!()
        }
    }
}

/// Write
///
/// First update with partial from 0.0..0.0 to 0.0..1.0, then lerp fill_opacity to 1.0
pub struct Write<T: Writing> {
    pub(crate) original: T,
    pub(crate) outline: T,
    create_anim: Create<T>,
}

impl<T: Writing> Write<T> {
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

impl<T: Writing> PureEvaluator<T> for Write<T> {
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
pub struct Unwrite<T: Writing> {
    pub(crate) original: T,
    pub(crate) outline: T,
    uncreate_anim: UnCreate<T>,
}

impl<T: Writing> Unwrite<T> {
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

impl<T: Writing> PureEvaluator<T> for Unwrite<T> {
    fn eval_alpha(&self, alpha: f32) -> T {
        let alpha = alpha * 2.0;
        if 0.0 < alpha && alpha < 1.0 {
            self.original.lerp(&self.outline, alpha)
        } else if alpha == 1.0 {
            self.outline.clone()
        } else if 1.0 < alpha && alpha <= 2.0 {
            self.uncreate_anim.eval_alpha(alpha - 1.0)
        } else {
            unreachable!()
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
