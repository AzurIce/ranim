use std::ops::Range;

use bevy_color::Srgba;

use crate::items::Entity;
use crate::prelude::Interpolatable;
use crate::Rabject;

use crate::animation::{AnimationFunc, entity::EntityAnimation};

pub fn create<T: Entity + Partial + Empty + Interpolatable + 'static>(
    target: Rabject<T>,
) -> EntityAnimation<T> {
    let inner = target.inner.clone();
    EntityAnimation::new(target, Create::new(inner))
}

pub fn uncreate<T: Entity + Partial + Empty + Interpolatable + 'static>(
    target: Rabject<T>,
) -> EntityAnimation<T> {
    let inner = target.inner.clone();
    EntityAnimation::new(target, UnCreate::new(inner))
}

pub fn write<T: Entity + Partial + Empty + Stroke + Fill + Interpolatable + Clone + 'static>(
    target: Rabject<T>,
) -> EntityAnimation<T> {
    let inner = target.inner.clone();
    EntityAnimation::new(target, Write::new(inner))
}

pub fn unwrite<T: Entity + Partial + Empty + Stroke + Fill + Interpolatable + Clone + 'static>(
    target: Rabject<T>,
) -> EntityAnimation<T> {
    let inner = target.inner.clone();
    EntityAnimation::new(target, Unwrite::new(inner))
}

// ---------------------------------------------------- //

pub struct Create<T: Entity + Partial + Empty + Interpolatable> {
    pub original: T,
}

impl<T: Entity + Partial + Empty + Interpolatable> Create<T> {
    fn new(target: T) -> Self {
        Self { original: target }
    }
}

impl<T: Entity + Partial + Empty + Interpolatable> AnimationFunc<T> for Create<T> {
    fn eval_alpha(&mut self, target: &mut T, alpha: f32) {
        if alpha == 0.0 {
            *target = T::empty();
        } else if 0.0 < alpha && alpha < 1.0 {
            *target = self.original.get_partial(0.0..alpha);
        } else if alpha == 1.0 {
            *target = self.original.clone();
        }
    }
}

pub struct UnCreate<T: Entity + Partial + Empty + Interpolatable> {
    pub original: T,
}

impl<T: Entity + Partial + Empty + Interpolatable> UnCreate<T> {
    fn new(target: T) -> Self {
        Self { original: target }
    }
}

impl<T: Entity + Partial + Empty + Interpolatable> AnimationFunc<T> for UnCreate<T> {
    fn eval_alpha(&mut self, target: &mut T, alpha: f32) {
        if alpha == 0.0 {
            *target = self.original.clone();
        } else if 0.0 < alpha && alpha < 1.0 {
            *target = self.original.get_partial(0.0..1.0 - alpha);
        } else if alpha == 1.0 {
            *target = T::empty();
        }
    }
}

/// Write
///
/// First update with partial from 0.0..0.0 to 0.0..1.0, then lerp fill_opacity to 1.0
pub struct Write<T: Entity + Partial + Empty + Stroke + Fill + Interpolatable> {
    pub(crate) original: T,
    pub(crate) outline: T,
    create_anim: Create<T>,
}

impl<T: Entity + Partial + Empty + Stroke + Fill + Interpolatable> Write<T> {
    fn new(target: T) -> Self {
        let mut outline = target.clone();
        outline
            .set_fill_opacity(0.0)
            .set_stroke_width(1.0)
            .set_stroke_opacity(1.0);
        Self {
            original: target,
            create_anim: Create::new(outline.clone()),
            outline,
        }
    }
}

impl<T: Entity + Partial + Empty + Stroke + Fill + Interpolatable + Clone + 'static>
    AnimationFunc<T> for Write<T>
{
    fn eval_alpha(&mut self, target: &mut T, alpha: f32) {
        let alpha = alpha * 2.0;
        if 0.0 <= alpha && alpha <= 1.0 {
            self.create_anim.eval_alpha(target, alpha);
        } else if 1.0 < alpha && alpha < 2.0 {
            *target = self.outline.lerp(&self.original, alpha - 1.0);
        } else if alpha == 2.0 {
            *target = self.original.clone();
        }
    }
}

/// Unwrite
///
/// First lerp fill_opacity to 0.0, then update with partial from 0.0..1.0 to 0.0..0.0
pub struct Unwrite<T: Entity + Partial + Empty + Stroke + Fill + Interpolatable> {
    pub(crate) original: T,
    pub(crate) outline: T,
    uncreate_anim: UnCreate<T>,
}

impl<T: Entity + Partial + Empty + Stroke + Fill + Interpolatable> Unwrite<T> {
    fn new(target: T) -> Self {
        let mut outline = target.clone();
        outline
            .set_fill_opacity(0.0)
            .set_stroke_width(1.0)
            .set_stroke_opacity(1.0);
        Self {
            original: target,
            uncreate_anim: UnCreate::new(outline.clone()),
            outline,
        }
    }
}

impl<T: Entity + Partial + Empty + Stroke + Fill + Interpolatable + 'static> AnimationFunc<T>
    for Unwrite<T>
{
    fn eval_alpha(&mut self, target: &mut T, alpha: f32) {
        let alpha = alpha * 2.0;
        if 0.0 < alpha && alpha < 1.0 {
            *target = self.original.lerp(&self.outline, alpha);
        } else if alpha == 1.0 {
            *target = self.outline.clone();
        } else if 1.0 < alpha && alpha <= 2.0 {
            self.uncreate_anim.eval_alpha(target, alpha - 1.0);
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
    fn set_fill_color(&mut self, color: Srgba) -> &mut Self;
}

pub trait Stroke {
    fn set_stroke_width(&mut self, width: f32) -> &mut Self;
    fn set_stroke_color(&mut self, color: Srgba) -> &mut Self;
    fn set_stroke_opacity(&mut self, opacity: f32) -> &mut Self;
}

pub trait Color: Fill + Stroke {
    fn set_color(&mut self, color: Srgba) -> &mut Self {
        self.set_fill_color(color);
        self.set_stroke_color(color);
        self
    }
}

impl<T: Fill + Stroke> Color for T {}
