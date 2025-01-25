use std::{
    any::{Any, TypeId},
    collections::HashMap,
};

use crate::{
    context::RanimContext, items::Entity, render::primitives::Primitive, updater::Updater,
    utils::Id,
};

#[allow(unused_imports)]
use log::debug;

use super::EntityId;

pub struct EntityCell<T: Entity> {
    inner: T,
    pub(crate) extract_data: Option<T::ExtractData>,
    pub(crate) primitive: Option<T::Primitive>,
    pub(crate) updaters: Vec<(Id, Box<dyn Updater<T>>)>,
}

impl<T: Entity> AsRef<T> for EntityCell<T> {
    fn as_ref(&self) -> &T {
        &self.inner
    }
}
impl<T: Entity> AsMut<T> for EntityCell<T> {
    fn as_mut(&mut self) -> &mut T {
        &mut self.inner
    }
}

impl<T: Entity> EntityCell<T> {
    pub fn new(inner: T) -> Self {
        Self {
            inner,
            extract_data: None,
            primitive: None,
            updaters: vec![],
        }
    }
    pub fn tick(&mut self, dt: f32) {
        let entity = &mut self.inner;
        self.updaters.retain_mut(|(_, updater)| {
            let keep = updater.on_update(entity, dt);
            if !keep {
                updater.on_destroy(entity);
            }
            keep
        });
    }
    pub fn extract(&mut self) {
        self.extract_data = self.inner.extract();
    }
    pub fn prepare(&mut self, ctx: &RanimContext) {
        let wgpu_ctx = ctx.wgpu_ctx();
        let Some(extract_data) = self.extract_data.as_ref() else {
            return;
        };
        if let Some(primitive) = self.primitive.as_mut() {
            primitive.update(&wgpu_ctx, extract_data);
        } else {
            self.primitive = Some(T::Primitive::init(&wgpu_ctx, extract_data));
        }
    }
    pub fn insert_updater(&mut self, mut updater: impl Updater<T> + 'static) -> Id {
        let id = Id::new();
        updater.on_create(&mut self.inner);
        self.updaters.push((id, Box::new(updater)));
        id
    }
    pub fn remove_updater(&mut self, id: Id) {
        self.updaters.retain(|(eid, _)| *eid != id);
    }
}

pub struct EntityStore<T: Entity> {
    inner: HashMap<Id, EntityCell<T>>,
}

impl<T: Entity> Default for EntityStore<T> {
    fn default() -> Self {
        Self {
            inner: HashMap::new(),
        }
    }
}

impl<T: Entity> EntityStore<T> {
    pub fn iter(&self) -> impl Iterator<Item = (&Id, &EntityCell<T>)> {
        self.inner.iter()
    }
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&Id, &mut EntityCell<T>)> {
        self.inner.iter_mut()
    }
}

impl<T: Entity> Store<T> for EntityStore<T> {
    fn insert(&mut self, entity: T) -> EntityId<T> {
        let id = Id::new();
        self.inner.insert(id, EntityCell::new(entity));
        // debug!("[RabjectStores::insert]: inserted entity {:?}", id);
        EntityId::from_id(id)
    }

    fn remove(&mut self, id: EntityId<T>) {
        // debug!("[RabjectStores::remove]: removing entity {:?}", id);
        self.inner.remove(&id);
    }

    fn get(&self, id: &EntityId<T>) -> &EntityCell<T> {
        // Since removing an entity consumes the [`EntityId`],
        // so if we have a reference of [`EntityId`], the entity
        // must be there and we can safely unwrap it.
        self.inner.get(&id).unwrap()
    }

    fn get_mut(&mut self, id: &EntityId<T>) -> &mut EntityCell<T> {
        // Since removing an entity consumes the [`EntityId`],
        // so if we have a reference of [`EntityId`], the entity
        // must be there and we can safely unwrap it.
        self.inner.get_mut(&id).unwrap()
    }
}

pub struct EntityStores {
    /// TypeId of T -> EntityStore<T>
    stores: HashMap<TypeId, Box<dyn Any>>,
}

impl Default for EntityStores {
    fn default() -> Self {
        Self {
            stores: HashMap::new(),
        }
    }
}

impl EntityStores {
    pub fn get_store<T: Entity + 'static>(&self) -> Option<&EntityStore<T>> {
        self.stores
            .get(&std::any::TypeId::of::<T>())
            .map(|s| s.downcast_ref::<EntityStore<T>>().unwrap())
    }
    pub fn get_store_mut<T: Entity + 'static>(&mut self) -> Option<&mut EntityStore<T>> {
        self.stores
            .get_mut(&std::any::TypeId::of::<T>())
            .map(|s| s.downcast_mut::<EntityStore<T>>().unwrap())
    }
    pub fn init_store<T: Entity + 'static>(&mut self) {
        self.stores
            .entry(std::any::TypeId::of::<T>())
            .or_insert(Box::<EntityStore<T>>::default());
    }
    pub fn entry_or_default<T: Entity + 'static>(&mut self) -> &mut EntityStore<T> {
        self.stores
            .entry(std::any::TypeId::of::<T>())
            .or_insert(Box::<EntityStore<T>>::default())
            .downcast_mut::<EntityStore<T>>()
            .unwrap()
    }
}

pub trait Store<T: Entity> {
    fn get(&self, id: &EntityId<T>) -> &EntityCell<T>;
    fn get_mut(&mut self, id: &EntityId<T>) -> &mut EntityCell<T>;
    fn insert(&mut self, entity: T) -> EntityId<T>;
    fn remove(&mut self, id: EntityId<T>);
}

// Entity management
// impl<R: 'static> Store for EntitiesStore<R> {}
