use std::ops::Range;

use crate::{prelude::Interpolatable, rabject::Updatable};

use super::{Animation, AnimationFunc};

pub enum CreationType {
    Create,
    Uncreate,
}

pub struct Create<T: Partial + Empty + Interpolatable + Clone> {
    pub original: Option<T>,
}

impl<T: Partial + Empty + Interpolatable + Clone> Default for Create<T> {
    fn default() -> Self {
        Self {
            original: None,
        }
    }
}

impl<T: Partial + Empty + Interpolatable + Clone + 'static> Create<T> {
    pub fn new() -> Animation<T> {
        Animation::new( Self::default())
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
        Self {
            original: None,
        }
    }
}

impl<T: Partial + Empty + Interpolatable + Clone + 'static> Uncreate<T> {
    pub fn new() -> Animation<T> {
        Animation::new( Self::default())
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


pub trait Partial {
    fn get_partial(&self, range: Range<f32>) -> Self;
}

pub trait Empty {
    fn empty() -> Self;
}