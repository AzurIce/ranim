use std::{any::Any, fmt::Debug, marker::PhantomData, ops::Deref};

use crate::{context::RanimContext, utils::Id};

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
pub trait Entity {
    type Renderer;

    fn tick(&mut self, dt: f32);
    fn extract(&mut self);
    fn prepare(&mut self, ctx: &RanimContext);
    fn render(&mut self, ctx: &mut RanimContext, renderer: &mut Self::Renderer);
}

pub struct EntityId<E: EntityAny>(Id, PhantomData<E>);

impl<E: EntityAny> Debug for EntityId<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "EntityId({:?})", self.0)
    }
}

impl<E: EntityAny> Copy for EntityId<E> {}

impl<E: EntityAny> Clone for EntityId<E> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<E: EntityAny> EntityId<E> {
    pub fn from_id(id: Id) -> Self {
        Self(id, PhantomData)
    }
}

impl<E: EntityAny> Deref for EntityId<E> {
    type Target = Id;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

