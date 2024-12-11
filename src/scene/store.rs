use std::{
    any::{Any, TypeId},
    collections::HashMap,
    ops::{Deref, DerefMut},
};

use crate::{
    context::WgpuContext,
    rabject::{Primitive, Rabject, RabjectId},
    utils::{Id, RenderResourceStorage},
};

#[allow(unused_imports)]
use log::debug;

pub trait RenderableAny: Renderable + Any {}

impl<T: Renderable + Any> RenderableAny for T {}

pub trait Renderable {
    fn extract(&mut self);
    fn check_extract(&self) -> bool;
    fn prepare(&mut self, wgpu_ctx: &WgpuContext);
    fn render(
        &self,
        _wgpu_ctx: &WgpuContext,
        _pipelines: &mut RenderResourceStorage,
        _multisample_view: &wgpu::TextureView,
        _target_view: &wgpu::TextureView,
        _depth_view: &wgpu::TextureView,
        _uniforms_bind_group: &wgpu::BindGroup,
    );
}

impl<R: Rabject + 'static> Renderable for RabjectStore<R> {
    fn extract(&mut self) {
        self.render_data = Some(self.rabject.extract());
    }
    fn check_extract(&self) -> bool {
        self.render_data.is_some()
    }
    fn prepare(&mut self, wgpu_ctx: &WgpuContext) {
        if let Some(render_resource) = self.render_resource.as_mut() {
            render_resource.update(wgpu_ctx, self.render_data.as_ref().unwrap());
        } else {
            self.render_resource = Some(R::RenderResource::init(
                wgpu_ctx,
                self.render_data.as_ref().unwrap(),
            ));
        }
    }
    fn render(
        &self,
        _wgpu_ctx: &WgpuContext,
        _pipelines: &mut RenderResourceStorage,
        _multisample_view: &wgpu::TextureView,
        _target_view: &wgpu::TextureView,
        _depth_view: &wgpu::TextureView,
        _uniforms_bind_group: &wgpu::BindGroup,
    ) {
        if let Some(render_resource) = self.render_resource.as_ref() {
            render_resource.render(
                _wgpu_ctx,
                _pipelines,
                _multisample_view,
                _target_view,
                _depth_view,
                _uniforms_bind_group,
            );
        }
    }
}

/// An entity in the scene
///
/// rabject --extract--> render_data --init--> render_resource
pub struct RabjectStore<R: Rabject> {
    /// The rabject
    pub rabject: R,
    /// The extracted data from the rabject
    pub render_data: Option<R::RenderData>,
    /// The prepared render resource of the rabject
    pub render_resource: Option<R::RenderResource>,
}

#[derive(Default)]
pub struct RabjectStores {
    /// The rabjects
    ///
    /// Rabject's type id -> Vec<(Rabject's id, RabjectStore<Rabject>)>
    inner: HashMap<TypeId, Vec<(Id, Box<dyn RenderableAny>)>>,
}

impl Deref for RabjectStores {
    type Target = HashMap<TypeId, Vec<(Id, Box<dyn RenderableAny>)>>;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for RabjectStores {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

// Entity management - Low level apis
impl RabjectStores {
    /// Low level api to insert an entity to the store directly
    ///
    /// For high level api, see [`RabjectStores::insert`]
    pub fn insert_entity<R: Rabject + 'static>(&mut self, entity: RabjectStore<R>) -> Id {
        let id = Id::new();
        debug!(
            "[Scene::insert_entity]: inserting entity {:?} of type {:?}",
            id,
            std::any::TypeId::of::<R>()
        );
        let entry = self.inner.entry(std::any::TypeId::of::<R>()).or_default();
        entry.push((id, Box::new(entity)));
        id
    }

    /// Low level api to remove an entity from the store directly
    ///
    /// For high level api, see [`RabjectStores::remove`]
    pub fn remove_entity(&mut self, id: Id) {
        for entry in self.inner.values_mut() {
            entry.retain(|(eid, _)| id != *eid);
        }
    }

    /// Low level api to get reference of an entity from the store directly
    ///
    /// For high level api, see [`RabjectStores::get`]
    pub fn get_entity<R: Rabject + 'static>(&self, id: &Id) -> Option<&RabjectStore<R>> {
        self.inner.get(&std::any::TypeId::of::<R>()).and_then(|e| {
            e.iter()
                .find(|(eid, _)| id == eid)
                .map(|(_, e)| (e as &dyn Any).downcast_ref::<RabjectStore<R>>().unwrap())
        })
    }

    /// Low level api to get mutable reference of an entity from the store directly
    ///
    /// For high level api, see [`RabjectStores::get_mut`]
    pub fn get_entity_mut<R: Rabject + 'static>(
        &mut self,
        id: &Id,
    ) -> Option<&mut RabjectStore<R>> {
        self.inner
            .get_mut(&std::any::TypeId::of::<R>())
            .and_then(|e| {
                e.iter_mut().find(|(eid, _)| id == eid).map(|(_, e)| {
                    (e as &mut dyn Any)
                        .downcast_mut::<RabjectStore<R>>()
                        .unwrap()
                })
            })
    }
}

// Entity management - High level apis
impl RabjectStores {
    /// Insert a rabject to the store
    pub fn insert<R: Rabject + 'static>(&mut self, rabject: R) -> RabjectId<R> {
        let entity = RabjectStore {
            rabject,
            render_data: None,
            render_resource: None,
        };
        RabjectId::from_id(self.insert_entity(entity))
    }

    /// Remove a rabject from the store
    pub fn remove<R: Rabject>(&mut self, id: RabjectId<R>) {
        self.remove_entity(*id);
    }

    /// Get a reference of a rabject from the store
    pub fn get<R: Rabject + 'static>(&self, id: &RabjectId<R>) -> Option<&R> {
        self.get_entity::<R>(id).map(|e| &e.rabject)
    }

    /// Get a mutable reference of a rabject from the store
    pub fn get_mut<R: Rabject + 'static>(&mut self, id: &RabjectId<R>) -> Option<&mut R> {
        self.get_entity_mut::<R>(id).map(|e| &mut e.rabject)
    }
}
