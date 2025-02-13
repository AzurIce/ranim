use std::ops::{Deref, DerefMut};

use glam::Vec2;

use crate::{
    context::WgpuContext,
    prelude::Empty,
    render::{
        primitives::{Extract, RenderInstance, RenderInstances},
        CameraFrame, RenderTextures, Renderable,
    },
    timeline::Timeline,
    utils::{Id, PipelinesStorage},
};

pub mod svg_item;
pub mod vitem;

/// An `Rabject` is a wrapper of an entity that can be rendered.
///
/// The `Rabject`s with same `Id` will use the same `EntityTimeline` to animate.
///
/// The cloned `Rabject` has the same Id
pub struct Rabject<'a, T: Entity> {
    pub timeline: &'a Timeline,
    pub id: Id,
    pub data: T,
}

pub struct FrozenRabject<T: Entity> {
    pub id: Id,
    pub data: T,
}

impl<T: Entity> Deref for Rabject<'_, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl<T: Entity> DerefMut for Rabject<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.data
    }
}

impl<T: Entity + 'static> Renderable for Rabject<'_, T> {
    fn render(
        &self,
        ctx: &WgpuContext,
        render_instances: &mut RenderInstances,
        pipelines: &mut PipelinesStorage,
        encoder: &mut wgpu::CommandEncoder,
        uniforms_bind_group: &wgpu::BindGroup,
        render_textures: &RenderTextures,
        camera: &CameraFrame,
    ) {
        let render_instance = render_instances.get_or_init::<T>(self.id);
        render_instance.update_clip_box(ctx, &self.data.clip_box(camera));
        render_instance.update(ctx, &self.data);
        render_instance.encode_render_command(
            ctx,
            pipelines,
            encoder,
            uniforms_bind_group,
            render_textures,
        );
    }
}

impl<T: Entity + 'static> Renderable for FrozenRabject<T> {
    fn render(
        &self,
        ctx: &WgpuContext,
        render_instances: &mut RenderInstances,
        pipelines: &mut PipelinesStorage,
        encoder: &mut wgpu::CommandEncoder,
        uniforms_bind_group: &wgpu::BindGroup,
        render_textures: &RenderTextures,
        camera: &CameraFrame,
    ) {
        let render_instance = render_instances.get_or_init::<T>(self.id);
        render_instance.update_clip_box(ctx, &self.data.clip_box(camera));
        render_instance.update(ctx, &self.data);
        render_instance.encode_render_command(
            ctx,
            pipelines,
            encoder,
            uniforms_bind_group,
            render_textures,
        );
    }
}

impl<T: Entity + Clone> Clone for FrozenRabject<T> {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            data: self.data.clone(),
        }
    }
}

impl<'a, T: Entity + 'static> Rabject<'a, T> {
    pub fn new(timeline: &'a Timeline, entity: T) -> Self {
        Self {
            timeline,
            id: Id::new(),
            data: entity,
        }
    }
}

impl<T: Entity> Drop for Rabject<'_, T> {
    fn drop(&mut self) {
        self.timeline.hide(self);
        // TODO: remove it
    }
}

pub trait Entity: Clone + Empty + Send + Sync {
    type Primitive: Extract<Self> + Default;

    #[allow(unused)]
    fn clip_box(&self, camera: &CameraFrame) -> [Vec2; 4] {
        [
            Vec2::new(-1.0, -1.0),
            Vec2::new(-1.0, 1.0),
            Vec2::new(1.0, -1.0),
            Vec2::new(1.0, 1.0),
        ]
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
