use std::{
    any::Any,
    ops::{Deref, DerefMut},
};

use crate::{
    camera::{Camera},
    context::RanimContext,
    rabject::{Primitive, Rabject},
    updater::Updater,
    utils::Id,
};

pub trait EntityAny: Entity + Any {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

impl<T: Entity + Any> EntityAny for T {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

/// An entity in the scene
///
/// This only implent way to [`Entity::tick`], [`Entity::extract`], [`Entity::prepare`] the entity.
/// For the actual render code, check the implementor of [`crate::camera::Renderer<E: EntityAny>`]
pub trait Entity {
    type Renderer;

    fn tick(&mut self, dt: f32);
    fn extract(&mut self);
    fn prepare(&mut self, ctx: &RanimContext);
    fn render(&mut self, ctx: &mut RanimContext, renderer: &mut Self::Renderer);
}
