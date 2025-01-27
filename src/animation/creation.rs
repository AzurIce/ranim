use std::ops::Range;

use bevy_color::Srgba;

use crate::items::Updatable;
use crate::prelude::Interpolatable;

use super::{Animation, AnimationFunc};

pub fn create<T: Partial + Empty + Interpolatable + Clone + 'static>() -> Animation<T> {
    Animation::new(Create::default())
}
pub fn uncreate<T: Partial + Empty + Interpolatable + Clone + 'static>() -> Animation<T> {
    Animation::new(Uncreate::default())
}

pub fn write<T: Partial + Empty + Stroke + Fill + Interpolatable + Clone + 'static>() -> Animation<T>
{
    Animation::new(Write::default())
}

pub fn unwrite<T: Partial + Empty + Stroke + Fill + Interpolatable + Clone + 'static>(
) -> Animation<T> {
    Animation::new(Unwrite::default())
}

// ---------------------------------------------------- //

pub struct Create<T: Partial + Empty + Interpolatable + Clone> {
    pub original: Option<T>,
}

impl<T: Partial + Empty + Interpolatable + Clone> Default for Create<T> {
    fn default() -> Self {
        Self { original: None }
    }
}

impl<T: Partial + Empty + Interpolatable + Clone + 'static> AnimationFunc<T> for Create<T> {
    fn init(&mut self, entity: &mut T) {
        self.original = Some(entity.clone());
    }

    fn interpolate(&mut self, entity: &mut T, alpha: f32) {
        if alpha == 0.0 {
            entity.update_from(&T::empty());
        } else if 0.0 < alpha && alpha < 1.0 {
            entity.update_from(&self.original.as_ref().unwrap().get_partial(0.0..alpha));
        } else if alpha == 1.0 {
            entity.update_from(&self.original.as_ref().unwrap());
        }
    }
}

pub struct Uncreate<T: Partial + Empty + Interpolatable + Clone> {
    pub original: Option<T>,
}

impl<T: Partial + Empty + Interpolatable + Clone> Default for Uncreate<T> {
    fn default() -> Self {
        Self { original: None }
    }
}

impl<T: Partial + Empty + Interpolatable + Clone + 'static> AnimationFunc<T> for Uncreate<T> {
    fn init(&mut self, entity: &mut T) {
        self.original = Some(entity.clone());
    }

    fn interpolate(&mut self, entity: &mut T, alpha: f32) {
        if alpha == 0.0 {
            // entity.update_from(&self.original.as_ref().unwrap());
        } else if 0.0 < alpha && alpha < 1.0 {
            entity.update_from(
                &self
                    .original
                    .as_ref()
                    .unwrap()
                    .get_partial(0.0..1.0 - alpha),
            );
        } else if alpha == 1.0 {
            entity.update_from(&T::empty());
        }
    }
}

/// Write
///
/// First update with partial from 0.0..0.0 to 0.0..1.0, then lerp fill_opacity to 1.0
pub struct Write<T: Partial + Empty + Stroke + Fill + Interpolatable + Clone> {
    pub(crate) original: Option<T>,
    pub(crate) outline: Option<T>,
    create_anim: Create<T>,
}

impl<T: Partial + Empty + Stroke + Fill + Interpolatable + Clone> Default for Write<T> {
    fn default() -> Self {
        Self {
            original: None,
            outline: None,
            create_anim: Create::default(),
        }
    }
}

impl<T: Partial + Empty + Stroke + Fill + Interpolatable + Clone + 'static> AnimationFunc<T>
    for Write<T>
{
    fn init(&mut self, entity: &mut T) {
        self.original = Some(entity.clone());

        // We assert that every entity has both stroke and fill
        let mut outline = entity.clone();
        outline.set_fill_opacity(0.0);
        outline.set_stroke_width(1.0);
        outline.set_stroke_opacity(1.0);
        self.outline = Some(outline);

        self.create_anim.init(self.outline.as_mut().unwrap());
    }

    fn interpolate(&mut self, entity: &mut T, alpha: f32) {
        let alpha = alpha * 2.0;
        if 0.0 <= alpha && alpha <= 1.0 {
            self.create_anim.interpolate(entity, alpha);
        } else if 1.0 < alpha && alpha < 2.0 {
            entity.update_from(
                &self
                    .outline
                    .as_ref()
                    .unwrap()
                    .lerp(self.original.as_ref().unwrap(), alpha - 1.0),
            );
        } else if alpha == 2.0 {
            entity.update_from(&self.original.as_ref().unwrap());
        }
    }
}

/// Unwrite
///
/// First lerp fill_opacity to 0.0, then update with partial from 0.0..1.0 to 0.0..0.0
pub struct Unwrite<T: Partial + Empty + Stroke + Fill + Interpolatable + Clone> {
    pub(crate) original: Option<T>,
    pub(crate) outline: Option<T>,
    uncreate_anim: Uncreate<T>,
}

impl<T: Partial + Empty + Stroke + Fill + Interpolatable + Clone> Default for Unwrite<T> {
    fn default() -> Self {
        Self {
            original: None,
            outline: None,
            uncreate_anim: Uncreate::default(),
        }
    }
}

impl<T: Partial + Empty + Stroke + Fill + Interpolatable + Clone + 'static> AnimationFunc<T>
    for Unwrite<T>
{
    fn init(&mut self, entity: &mut T) {
        self.original = Some(entity.clone());

        // We assert that every entity has both stroke and fill
        let mut outline = entity.clone();
        outline.set_fill_opacity(0.0);
        outline.set_stroke_width(1.0);
        outline.set_stroke_opacity(1.0);
        self.outline = Some(outline);

        self.uncreate_anim.init(self.outline.as_mut().unwrap());
    }

    fn interpolate(&mut self, entity: &mut T, alpha: f32) {
        let alpha = alpha * 2.0;
        if 0.0 < alpha && alpha < 1.0 {
            entity.update_from(
                &self
                    .original
                    .as_ref()
                    .unwrap()
                    .lerp(self.outline.as_ref().unwrap(), alpha),
            );
        } else if alpha == 1.0 {
            entity.update_from(&self.outline.as_ref().unwrap());
        } else if 1.0 < alpha && alpha <= 2.0 {
            self.uncreate_anim.interpolate(entity, alpha - 1.0);
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