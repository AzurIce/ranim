use std::ops::Range;

use bevy_color::Srgba;

use crate::animation::AnimWithParams;
use crate::items::{Entity, Rabject};
use crate::prelude::Interpolatable;

use crate::animation::entity::{EntityAnim, PureEvaluator};

pub fn create<T: Entity + Partial + Empty + Interpolatable + 'static>(
    rabject: &Rabject<T>,
) -> AnimWithParams<EntityAnim<T>> {
    let func = Create::new(rabject.data.clone());
    AnimWithParams::new(EntityAnim::new(rabject.clone(), func))
}

pub fn uncreate<T: Entity + Partial + Empty + Interpolatable + 'static>(
    rabject: &Rabject<T>,
) -> AnimWithParams<EntityAnim<T>> {
    let func = UnCreate::new(rabject.data.clone());
    AnimWithParams::new(EntityAnim::new(rabject.clone(), func))
}

pub fn write<T: Entity + Partial + Empty + Stroke + Fill + Interpolatable + Clone + 'static>(
    rabject: &Rabject<T>,
) -> AnimWithParams<EntityAnim<T>> {
    let func = Write::new(rabject.data.clone());
    AnimWithParams::new(EntityAnim::new(rabject.clone(), func))
}

pub fn unwrite<T: Entity + Partial + Empty + Stroke + Fill + Interpolatable + Clone + 'static>(
    rabject: &Rabject<T>,
) -> AnimWithParams<EntityAnim<T>> {
    let func = Unwrite::new(rabject.data.clone());
    AnimWithParams::new(EntityAnim::new(rabject.clone(), func))
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

impl<T: Entity + Partial + Empty + Interpolatable> PureEvaluator<T> for Create<T> {
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

pub struct UnCreate<T: Entity + Partial + Empty + Interpolatable> {
    pub original: T,
}

impl<T: Entity + Partial + Empty + Interpolatable> UnCreate<T> {
    fn new(target: T) -> Self {
        Self { original: target }
    }
}

impl<T: Entity + Partial + Empty + Interpolatable> PureEvaluator<T> for UnCreate<T> {
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
            .set_stroke_color(target.fill_color())
            .set_stroke_opacity(1.0);
        Self {
            original: target,
            create_anim: Create::new(outline.clone()),
            outline,
        }
    }
}

impl<T: Entity + Partial + Empty + Stroke + Fill + Interpolatable + Clone + 'static>
    PureEvaluator<T> for Write<T>
{
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
            .set_stroke_color(target.fill_color())
            .set_stroke_opacity(1.0);
        Self {
            original: target,
            uncreate_anim: UnCreate::new(outline.clone()),
            outline,
        }
    }
}

impl<T: Entity + Partial + Empty + Stroke + Fill + Interpolatable + 'static> PureEvaluator<T>
    for Unwrite<T>
{
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
    fn fill_color(&self) -> Srgba;
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
