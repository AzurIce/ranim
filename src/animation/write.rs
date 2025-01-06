use bevy_color::LinearRgba;

use crate::{prelude::Interpolatable, rabject::Updatable, utils::math::interpolate_usize};

use super::{
    creation::{Create, CreationType, Empty, Partial, Uncreate},
    transform::Transform,
    Animation, AnimationFunc,
};

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

impl<T: Partial + Empty + Stroke + Fill + Interpolatable + Clone + 'static> Write<T> {
    pub fn new() -> Animation<T> {
        Animation::new(Self::default())
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

impl<T: Partial + Empty + Stroke + Fill + Interpolatable + Clone + 'static> Unwrite<T> {
    pub fn new() -> Animation<T> {
        Animation::new(Self::default())
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

pub trait Fill {
    fn set_fill_opacity(&mut self, opacity: f32) -> &mut Self;
    fn set_fill_color(&mut self, color: impl Into<LinearRgba> + Copy) -> &mut Self;
}

pub trait Stroke {
    fn set_stroke_width(&mut self, width: f32) -> &mut Self;
    fn set_stroke_color(&mut self, color: impl Into<LinearRgba> + Copy) -> &mut Self;
    fn set_stroke_opacity(&mut self, opacity: f32) -> &mut Self;
}
