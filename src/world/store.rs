use std::{
    collections::HashMap,
    ops::{Deref, DerefMut},
};

use crate::{updater::Updater, utils::Id};

#[allow(unused_imports)]
use log::debug;

use super::{Entity, EntityAny, EntityId};

pub struct EntityStore<E: EntityAny> {
    inner: E,
    pub(crate) updaters: Vec<(Id, Box<dyn Updater<E>>)>,
}

impl<E: EntityAny> Deref for EntityStore<E> {
    type Target = E;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<E: EntityAny> DerefMut for EntityStore<E> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl<E: EntityAny> EntityStore<E> {
    pub fn new(entity: E) -> Self {
        Self {
            inner: entity,
            updaters: Vec::new(),
        }
    }
    pub fn insert_updater(&mut self, mut updater: impl Updater<E> + 'static) -> Id {
        let id = Id::new();
        updater.on_create(self);
        self.updaters.push((id, Box::new(updater)));
        id
    }
    pub fn remove_updater(&mut self, id: Id) {
        self.updaters.retain(|(eid, _)| *eid != id);
    }
}

impl<T: EntityAny> Entity for EntityStore<T> {
    type Renderer = T::Renderer;

    fn tick(&mut self, dt: f32) {
        self.inner.tick(dt);
        let entity = &mut self.inner;
        self.updaters.retain_mut(|(_, updater)| {
            let keep = updater.on_update(entity, dt);
            if !keep {
                updater.on_destroy(entity);
            }
            keep
        });
    }
    fn extract(&mut self) {
        self.inner.extract();
    }
    fn prepare(&mut self, ctx: &crate::context::RanimContext) {
        self.inner.prepare(ctx);
    }
    fn render(&mut self, ctx: &mut crate::context::RanimContext, renderer: &mut Self::Renderer) {
        self.inner.render(ctx, renderer);
    }
}

/// A store of entities
///
/// Entity's type id -> Vec<(Entity's id, EntityStore(Entity))>
pub struct EntitiesStore<R> {
    inner: HashMap<Id, Box<dyn EntityAny<Renderer = R>>>,
}

impl<Renderer> Default for EntitiesStore<Renderer> {
    fn default() -> Self {
        Self {
            inner: HashMap::new(),
        }
    }
}

impl<R> Deref for EntitiesStore<R> {
    type Target = HashMap<Id, Box<dyn EntityAny<Renderer = R>>>;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<R> DerefMut for EntitiesStore<R> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

pub trait Store<R: 'static> {
    fn insert<E: EntityAny<Renderer = R>>(&mut self, entity: E) -> EntityId<E>;
    fn remove<E: EntityAny<Renderer = R>>(&mut self, id: EntityId<E>);
    fn get<E: EntityAny<Renderer = R>>(&self, id: &EntityId<E>) -> &EntityStore<E>;
    fn get_mut<E: EntityAny<Renderer = R>>(&mut self, id: &EntityId<E>) -> &mut EntityStore<E>;
}

// Entity management
impl<R: 'static> Store<R> for EntitiesStore<R> {
    fn insert<E: EntityAny<Renderer = R>>(&mut self, entity: E) -> EntityId<E> {
        let id = Id::new();
        // debug!(
        //     "[RabjectStores::insert]: inserting entity {:?} of type {:?}",
        //     id,
        //     std::any::TypeId::of::<E>()
        // );
        self.inner.insert(id, Box::new(EntityStore::new(entity)));
        // debug!("[RabjectStores::insert]: inserted entity {:?}", id);
        EntityId::from_id(id)
    }

    fn remove<E: EntityAny<Renderer = R>>(&mut self, id: EntityId<E>) {
        // debug!("[RabjectStores::remove]: removing entity {:?}", id);
        self.inner.remove(&id);
    }

    fn get<E: EntityAny<Renderer = R>>(&self, id: &EntityId<E>) -> &EntityStore<E> {
        // debug!(
        //     "[RabjectStores::get]: getting entity {:?} of type {:?}",
        //     id,
        //     std::any::TypeId::of::<E>()
        // );
        // Since removing an entity consumes the [`EntityId`],
        // so if we have a reference of [`EntityId`], the entity
        // must be there and we can safely unwrap it.
        self.inner
            .get(&id)
            .and_then(|e| e.as_any().downcast_ref::<EntityStore<E>>())
            .unwrap()
    }

    fn get_mut<E: EntityAny<Renderer = R>>(&mut self, id: &EntityId<E>) -> &mut EntityStore<E> {
        // debug!(
        //     "[RabjectStores::get_mut]: getting entity {:?} of type {:?}",
        //     id,
        //     std::any::TypeId::of::<E>()
        // );
        // Since removing an entity consumes the [`EntityId`],
        // so if we have a reference of [`EntityId`], the entity
        // must be there and we can safely unwrap it.
        self.inner
            .get_mut(&id)
            .and_then(|e| e.as_any_mut().downcast_mut::<EntityStore<E>>())
            .unwrap()
    }
}
