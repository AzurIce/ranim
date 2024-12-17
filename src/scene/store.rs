use std::{
    any::{Any, TypeId},
    collections::HashMap,
    ops::{Deref, DerefMut},
};

use crate::{
    context::WgpuContext,
    prelude::RabjectContainer,
    rabject::{Primitive, Rabject, RabjectId},
    updater::Updater,
    utils::{Id, RenderResourceStorage},
};

#[allow(unused_imports)]
use log::debug;

use super::UpdaterStore;

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

pub trait Entity {
    fn tick(&mut self, dt: f32);
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

impl<R: Rabject + 'static> Entity for RabjectStore<R> {
    fn tick(&mut self, dt: f32) {
        let rabject = &mut self.rabject;
        self.updaters.retain_mut(|(_, updater)| {
            let keep = updater.on_update(rabject, dt);
            if !keep {
                updater.on_destroy(rabject);
            }
            keep
        });
    }
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
    pub(crate) rabject: R,
    /// The updaters for this rabject
    ///
    /// A vector of updater's id and updater itself
    /// Vec<(Id, Updater<Rabject>)>
    pub(crate) updaters: Vec<(Id, Box<dyn Updater<R>>)>,
    /// The extracted data from the rabject
    pub(crate) render_data: Option<R::RenderData>,
    /// The prepared render resource of the rabject
    pub(crate) render_resource: Option<R::RenderResource>,
}

impl<R: Rabject> Deref for RabjectStore<R> {
    type Target = R;
    fn deref(&self) -> &Self::Target {
        &self.rabject
    }
}

impl<R: Rabject> DerefMut for RabjectStore<R> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.rabject
    }
}

impl<R: Rabject> RabjectStore<R> {
    pub fn insert_updater(&mut self, mut updater: impl Updater<R> + 'static) -> Id {
        let id = Id::new();
        updater.on_create(self);
        self.updaters.push((id, Box::new(updater)));
        id
    }
    pub fn remove_updater(&mut self, id: Id) {
        self.updaters.retain(|(eid, _)| *eid != id);
    }
}

#[derive(Default)]
pub struct RabjectStores {
    /// The rabjects
    ///
    /// Rabject's type id -> Vec<(Rabject's id, RabjectStore<Rabject>)>
    inner: HashMap<TypeId, Vec<(Id, Box<dyn EntityAny>)>>,
}

impl Deref for RabjectStores {
    type Target = HashMap<TypeId, Vec<(Id, Box<dyn EntityAny>)>>;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for RabjectStores {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

// Entity management
impl RabjectContainer for RabjectStores {
    /// Insert a rabject to the store
    fn update_or_insert<R: Rabject + 'static>(&mut self, rabject: R) -> RabjectId<R> {
        let id = Id::new();
        debug!(
            "[RabjectStores::insert_entity]: inserting entity {:?} of type {:?}",
            id,
            std::any::TypeId::of::<R>()
        );
        let entry = self.inner.entry(std::any::TypeId::of::<R>()).or_default();
        let entity = RabjectStore {
            rabject,
            updaters: vec![],
            render_data: None,
            render_resource: None,
        };
        entry.push((id, Box::new(entity)));
        debug!(
            "[RabjectStores::update_or_insert]: inserted entity {:?}",
            id
        );
        RabjectId::from_id(id)
    }

    /// Remove a rabject from the store
    fn remove<R: Rabject>(&mut self, id: RabjectId<R>) {
        debug!("[RabjectStores::remove]: removing entity {:?}", id);
        for entry in self.inner.values_mut() {
            entry.retain(|(eid, _)| *id != *eid);
        }
    }

    /// Get a reference of a rabject from the store
    fn get<R: Rabject + 'static>(&self, id: &RabjectId<R>) -> Option<&RabjectStore<R>> {
        debug!(
            "[RabjectStores::get]: getting entity {:?} of type {:?}",
            id,
            std::any::TypeId::of::<R>()
        );
        self.inner.get(&std::any::TypeId::of::<R>()).and_then(|e| {
            e.iter()
                .find(|(eid, _)| **id == *eid)
                .map(|(_, e)| (e as &dyn Any).downcast_ref::<RabjectStore<R>>().unwrap())
        })
    }

    /// Get a mutable reference of a rabject from the store
    fn get_mut<R: Rabject + 'static>(&mut self, id: &RabjectId<R>) -> Option<&mut RabjectStore<R>> {
        debug!(
            "[RabjectStores::get_mut]: getting entity {:?} of type {:?}",
            id,
            std::any::TypeId::of::<R>()
        );
        self.inner
            .get_mut(&std::any::TypeId::of::<R>())
            .and_then(|e| {
                e.iter_mut()
                    .find(|(eid, _)| **id == *eid)
                    .map(|(_, e)| e.as_any_mut().downcast_mut::<RabjectStore<R>>().unwrap())
            })
    }
}
