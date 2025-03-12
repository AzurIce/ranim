use std::rc::Rc;

use crate::{
    animation::{AnimSchedule, Animation},
    context::WgpuContext,
    render::primitives::{RenderInstance, RenderInstances},
    RanimTimeline,
};

pub mod camera_frame;
pub mod svg_item;
pub mod vitem;

/// An `Rabject` is a wrapper of an entity that can be rendered.
///
/// The `Rabject`s with same `Id` will use the same `EntityTimeline` to animate.
pub struct Rabject<'a, T> {
    pub timeline: &'a RanimTimeline,
    pub id: usize,
    pub data: T,
}

impl<T> Drop for Rabject<'_, T> {
    fn drop(&mut self) {
        self.timeline.hide(self);
        // TODO: remove it
    }
}

impl<'t, T: 'static> Rabject<'t, T> {
    pub fn schedule<'r>(
        &'r mut self,
        anim_builder: impl FnOnce(&mut Self) -> Animation<T>,
    ) -> AnimSchedule<'r, 't, T> {
        let animation = (anim_builder)(self);
        AnimSchedule::new(self, animation)
    }
}

pub trait Entity {
    fn get_render_instance_for_entity<'a>(
        &self,
        render_instances: &'a RenderInstances,
        entity_id: usize,
    ) -> Option<&'a dyn RenderInstance>;
    fn prepare_render_instance_for_entity(
        &self,
        ctx: &WgpuContext,
        render_instances: &mut RenderInstances,
        entity_id: usize,
    );
}

impl<T: Entity + 'static> Entity for Rc<T> {
    fn get_render_instance_for_entity<'a>(
        &self,
        render_instances: &'a RenderInstances,
        entity_id: usize,
    ) -> Option<&'a dyn RenderInstance> {
        self.as_ref()
            .get_render_instance_for_entity(render_instances, entity_id)
    }
    fn prepare_render_instance_for_entity(
        &self,
        ctx: &WgpuContext,
        render_instances: &mut RenderInstances,
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
