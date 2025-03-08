use std::{
    ops::{Deref, DerefMut},
    rc::Rc,
};

use crate::{
    context::WgpuContext,
    prelude::Empty,
    render::primitives::{ExtractFrom, RenderInstance, RenderInstances},
    Timeline,
};

pub mod camera_frame;
pub mod svg_item;
pub mod vitem;

/// An `Rabject` is a wrapper of an entity that can be rendered.
///
/// The `Rabject`s with same `Id` will use the same `EntityTimeline` to animate.
pub struct Rabject<'a, T> {
    pub timeline: &'a Timeline,
    pub id: usize,
    pub data: T,
}

impl<T> Deref for Rabject<'_, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl<T> DerefMut for Rabject<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.data
    }
}

impl<T> Drop for Rabject<'_, T> {
    fn drop(&mut self) {
        self.timeline.hide(self);
        // TODO: remove it
    }
}

pub trait Entity: Clone + Empty + Send + Sync {
    type Primitive: ExtractFrom<Self> + Default;
}

pub trait ItemEntity {
    fn get_render_instance_for_entity<'a>(
        &self,
        render_instances: &'a RenderInstances,
        entity_id: usize,
    ) -> Option<&'a dyn RenderInstance>;
    fn prepare_render_instance_for_entity<'a>(
        &self,
        ctx: &WgpuContext,
        render_instances: &'a mut RenderInstances,
        entity_id: usize,
    );
}

impl<T: Entity + 'static> ItemEntity for T {
    fn get_render_instance_for_entity<'a>(
        &self,
        render_instances: &'a RenderInstances,
        entity_id: usize,
    ) -> Option<&'a dyn RenderInstance> {
        render_instances
            .get_dynamic::<T>(entity_id)
            .map(|x| x as &dyn RenderInstance)
    }
    fn prepare_render_instance_for_entity<'a>(
        &self,
        ctx: &WgpuContext,
        render_instances: &'a mut RenderInstances,
        entity_id: usize,
    ) {
        let render_instance = render_instances.get_dynamic_or_init::<T>(entity_id);
        render_instance.update_from(ctx, self);
    }
}

impl<T: Entity + 'static> ItemEntity for Rc<T> {
    fn get_render_instance_for_entity<'a>(
        &self,
        render_instances: &'a RenderInstances,
        entity_id: usize,
    ) -> Option<&'a dyn RenderInstance> {
        self.as_ref()
            .get_render_instance_for_entity(render_instances, entity_id)
    }
    fn prepare_render_instance_for_entity<'a>(
        &self,
        ctx: &WgpuContext,
        render_instances: &'a mut RenderInstances,
        entity_id: usize,
    ) {
        self.as_ref()
            .prepare_render_instance_for_entity(ctx, render_instances, entity_id);
    }
}

/// Blueprints are the data structures that are used to create [`Rabject`]s
pub trait Blueprint<T: Entity> {
    fn build(self) -> T;
}

pub trait Updatable {
    fn update_from(&mut self, other: &Self);
}

impl<T: Clone> Updatable for T {
    fn update_from(&mut self, other: &Self) {
        *self = other.clone();
    }
}
