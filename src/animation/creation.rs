use std::ops::Range;

use crate::{
    prelude::Interpolatable,
    rabject::{rabject2d::vmobject::VMobject, Updatable},
};

use super::{Animation, AnimationFunc};

pub enum CreationType {
    Create,
    UnCreate,
}

pub struct Creation<R: Partial + Interpolatable + Clone> {
    pub(crate) original: Option<R>,
    pub(crate) creation_type: CreationType,
}

impl<R: Partial + Empty + Interpolatable + Clone + 'static> Creation<R> {
    pub fn create() -> Animation<R> {
        Animation::new(Self {
            original: None,
            creation_type: CreationType::Create,
        })
    }

    pub fn un_create() -> Animation<R> {
        Animation::new(Self {
            original: None,
            creation_type: CreationType::UnCreate,
        })
    }
}

pub trait Partial {
    fn get_partial(&self, range: Range<f32>) -> Self;
}

pub trait Empty {
    fn empty() -> Self;
}

impl<T: Partial + Empty + Interpolatable + Clone> AnimationFunc<T> for Creation<T> {
    fn pre_anim(&mut self, entity: &mut T) {
        self.original = Some(entity.clone());
        match self.creation_type {
            CreationType::Create => entity.update_from(&T::empty()),
            CreationType::UnCreate => entity.update_from(&self.original.as_ref().unwrap()),
        }
    }

    fn interpolate(&mut self, entity: &mut T, alpha: f32) {
        entity.update_from(&self.original.as_ref().unwrap().get_partial(
            match self.creation_type {
                CreationType::Create => 0.0..alpha,
                CreationType::UnCreate => alpha..1.0,
            },
        )); // TODO: Fixalpha));
    }

    fn post_anim(&mut self, entity: &mut T) {
        match self.creation_type {
            CreationType::Create => entity.update_from(&self.original.as_ref().unwrap()),
            CreationType::UnCreate => entity.update_from(&T::empty()),
        }
    }
}
