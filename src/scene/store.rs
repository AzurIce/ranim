use std::{
    collections::HashMap,
    fmt::Debug,
    marker::PhantomData,
    ops::{Deref, DerefMut},
};

use crate::utils::Id;

#[allow(unused_imports)]
use log::debug;

use super::entity::EntityAny;

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

/// A store of entities
///
/// Entity's type id -> Vec<(Entity's id, Entity)>
pub struct EntityStore<R> {
    inner: HashMap<Id, Box<dyn EntityAny<Renderer = R>>>,
}

impl<Renderer> Default for EntityStore<Renderer> {
    fn default() -> Self {
        Self {
            inner: HashMap::new(),
        }
    }
}

impl<R> Deref for EntityStore<R> {
    type Target = HashMap<Id, Box<dyn EntityAny<Renderer = R>>>;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<R> DerefMut for EntityStore<R> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

// Entity management
impl<R: 'static> EntityStore<R> {
    pub fn insert<E: EntityAny<Renderer = R>>(&mut self, entity: E) -> EntityId<E> {
        let id = Id::new();
        debug!(
            "[RabjectStores::insert]: inserting entity {:?} of type {:?}",
            id,
            std::any::TypeId::of::<E>()
        );
        self.inner.insert(id, Box::new(entity));
        debug!("[RabjectStores::insert]: inserted entity {:?}", id);
        EntityId::from_id(id)
    }

    pub fn remove<E: EntityAny<Renderer = R>>(&mut self, id: EntityId<E>) {
        debug!("[RabjectStores::remove]: removing entity {:?}", id);
        self.inner.remove(&id);
    }

    pub fn get<E: EntityAny<Renderer = R>>(&self, id: &EntityId<E>) -> &E {
        debug!(
            "[RabjectStores::get]: getting entity {:?} of type {:?}",
            id,
            std::any::TypeId::of::<E>()
        );
        // Since removing an entity consumes the [`EntityId`],
        // so if we have a reference of [`EntityId`], the entity
        // must be there and we can safely unwrap it.
        self.inner
            .get(&id)
            .and_then(|e| e.as_any().downcast_ref::<E>())
            .unwrap()
    }

    pub fn get_mut<E: EntityAny<Renderer = R>>(&mut self, id: &EntityId<E>) -> &mut E {
        debug!(
            "[RabjectStores::get_mut]: getting entity {:?} of type {:?}",
            id,
            std::any::TypeId::of::<E>()
        );
        // Since removing an entity consumes the [`EntityId`],
        // so if we have a reference of [`EntityId`], the entity
        // must be there and we can safely unwrap it.
        self.inner
            .get_mut(&id)
            .and_then(|e| e.as_any_mut().downcast_mut::<E>())
            .unwrap()
    }
}
