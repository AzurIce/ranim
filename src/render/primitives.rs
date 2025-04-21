pub mod svg_item;
pub mod vitem;

use std::{any::Any, collections::HashMap};

use crate::context::WgpuContext;

use super::RenderTextures;

/// A Primitive is a structure that encapsules the wgpu resources
pub trait Primitive {
    type Data;
    fn init(ctx: &WgpuContext, data: &Self::Data) -> Self;
    fn update(&mut self, ctx: &WgpuContext, data: &Self::Data);
}

pub trait Renderable {
    fn encode_render_command(
        &self,
        ctx: &WgpuContext,
        pipelines: &mut super::PipelinesStorage,
        encoder: &mut wgpu::CommandEncoder,
        uniforms_bind_group: &wgpu::BindGroup,
        render_textures: &RenderTextures,
        #[cfg(feature = "profiling")] profiler: &mut wgpu_profiler::GpuProfiler,
    );
    fn debug(&self, _ctx: &WgpuContext) {}
}

impl<T0: Renderable> Renderable for (T0,) {
    fn encode_render_command(
        &self,
        ctx: &WgpuContext,
        pipelines: &mut super::PipelinesStorage,
        encoder: &mut wgpu::CommandEncoder,
        uniforms_bind_group: &wgpu::BindGroup,
        render_textures: &RenderTextures,
        #[cfg(feature = "profiling")] profiler: &mut wgpu_profiler::GpuProfiler,
    ) {
        self.0.encode_render_command(
            ctx,
            pipelines,
            encoder,
            uniforms_bind_group,
            render_textures,
            #[cfg(feature = "profiling")]
            profiler,
        );
    }
    fn debug(&self, ctx: &WgpuContext) {
        self.0.debug(ctx);
    }
}

impl<T0: Renderable, T1: Renderable> Renderable for (T0, T1) {
    fn encode_render_command(
        &self,
        ctx: &WgpuContext,
        pipelines: &mut super::PipelinesStorage,
        encoder: &mut wgpu::CommandEncoder,
        uniforms_bind_group: &wgpu::BindGroup,
        render_textures: &RenderTextures,
        #[cfg(feature = "profiling")] profiler: &mut wgpu_profiler::GpuProfiler,
    ) {
        self.0.encode_render_command(
            ctx,
            pipelines,
            encoder,
            uniforms_bind_group,
            render_textures,
            #[cfg(feature = "profiling")]
            profiler,
        );
        self.1.encode_render_command(
            ctx,
            pipelines,
            encoder,
            uniforms_bind_group,
            render_textures,
            #[cfg(feature = "profiling")]
            profiler,
        );
    }
    fn debug(&self, ctx: &WgpuContext) {
        self.0.debug(ctx);
        self.1.debug(ctx);
    }
}

/// Extract is the process of getting [`Primitive::Data`] from an item.
///
/// If [`Primitive::Data`] is [`Renderable`], then [`RenderableItem`] will be automatically implemented.
pub trait Extract {
    type Primitive: Primitive;
    fn extract(&self) -> <Self::Primitive as Primitive>::Data;
}

/// RenderableItem is what can [`Extract`] to a [`Renderable`] [`Primitive`].
/// This is automatically implemented for all types that implement [`Extract<Primitive = P>`]
/// where `P` implements [`Renderable`] and [`Primitive`].
///
/// If you want to implement your own [`RenderableItem`], all you need to do is implement [`Extract`].
pub trait RenderableItem {
    fn prepare_for_id(&self, ctx: &WgpuContext, render_instances: &mut RenderInstances, id: usize);
    fn renderable_of_id<'a>(
        &'a self,
        render_instances: &'a RenderInstances,
        id: usize,
    ) -> Option<&'a dyn Renderable>;
}

impl<T, P> RenderableItem for T
where
    T: Extract<Primitive = P>,
    P: Renderable + Primitive + 'static,
{
    fn prepare_for_id(&self, ctx: &WgpuContext, render_instances: &mut RenderInstances, id: usize) {
        render_instances.prepare(ctx, id, self);
    }
    fn renderable_of_id<'a>(
        &'a self,
        render_instances: &'a RenderInstances,
        id: usize,
    ) -> Option<&'a dyn Renderable> {
        render_instances.get_renderable::<T, P>(id)
    }
}

// pub trait ExtractFrom<T: Entity>: Renderable + Any {
//     fn update_from(&mut self, ctx: &WgpuContext, data: &T);
// }

#[derive(Default)]
pub struct RenderInstances {
    // Rabject's id -> RenderInstance
    items: HashMap<usize, Box<dyn Any>>,
}

impl RenderInstances {
    pub(crate) fn insert_render_instance<T: Renderable + 'static>(
        &mut self,
        id: usize,
        instance: T,
    ) {
        self.items.insert(id, Box::new(instance));
    }
    pub(crate) fn get_render_instance<T: Renderable + 'static>(&self, id: usize) -> Option<&T> {
        self.items
            .get(&id)
            .and_then(|x| x.as_ref().downcast_ref::<T>())
    }
    pub(crate) fn get_render_instance_mut<T: Renderable + 'static>(
        &mut self,
        id: usize,
    ) -> Option<&mut T> {
        self.items
            .get_mut(&id)
            .and_then(|x| x.as_mut().downcast_mut::<T>())
    }
    pub fn prepare<T: Extract<Primitive = P>, P: Renderable + Primitive + 'static>(
        &mut self,
        ctx: &WgpuContext,
        id: usize,
        item: &T,
    ) {
        let primitive_data = item.extract();
        if let Some(render_instance) = self.get_render_instance_mut::<P>(id) {
            render_instance.update(ctx, &primitive_data);
        } else {
            self.insert_render_instance(id, P::init(ctx, &primitive_data));
        }
    }
    pub fn get_renderable<T: Extract<Primitive = P>, P: Renderable + Primitive + 'static>(
        &self,
        id: usize,
    ) -> Option<&dyn Renderable> {
        self.items
            .get(&id)
            .map(|x| x.downcast_ref::<P>().unwrap() as &dyn Renderable)
    }
}

impl Renderable for Vec<&dyn Renderable> {
    fn encode_render_command(
        &self,
        ctx: &WgpuContext,
        pipelines: &mut super::PipelinesStorage,
        encoder: &mut wgpu::CommandEncoder,
        uniforms_bind_group: &wgpu::BindGroup,
        render_textures: &RenderTextures,
        #[cfg(feature = "profiling")] profiler: &mut wgpu_profiler::GpuProfiler,
    ) {
        for render_instance in self {
            render_instance.encode_render_command(
                ctx,
                pipelines,
                encoder,
                uniforms_bind_group,
                render_textures,
                #[cfg(feature = "profiling")]
                profiler,
            );
        }
    }
    fn debug(&self, ctx: &WgpuContext) {
        for render_instance in self {
            render_instance.debug(ctx);
        }
    }
}
