pub mod svg_item;
pub mod vitem;

use std::{
    any::{Any, TypeId},
    collections::HashMap,
};

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

pub fn prepare_render_primitive<T: Extract<Primitive = P>, P: Renderable + Primitive + 'static>(
    ctx: &WgpuContext,
    render_instances: &mut RenderInstances,
    id: usize,
    data: T,
) {
    let primitive_data = data.extract();
    let primitive = P::init(ctx, &primitive_data);
    render_instances
        .items
        .insert((id, TypeId::of::<P>()), Box::new(primitive));
}

// pub trait ExtractFrom<T: Entity>: Renderable + Any {
//     fn update_from(&mut self, ctx: &WgpuContext, data: &T);
// }

#[derive(Default)]
pub struct RenderInstances {
    // Rabject's id, RenderInstance's TypeId -> RenderInstance
    dynamic_items: HashMap<(usize, TypeId), Box<dyn Any>>,
    //
    items: HashMap<(usize, TypeId), Box<dyn Any>>,
}

impl RenderInstances {
    pub fn prepare<T: Extract<Primitive = P>, P: Renderable + Primitive + 'static>(
        &mut self,
        ctx: &WgpuContext,
        id: usize,
        item: &T,
    ) {
        let primitive_data = item.extract();
        let primitive = P::init(ctx, &primitive_data);
        self.items
            .insert((id, TypeId::of::<P>()), Box::new(primitive));
    }
    pub fn get_renderable<T: Extract<Primitive = P>, P: Renderable + Primitive + 'static>(
        &self,
        id: usize,
    ) -> Option<&dyn Renderable> {
        self.items
            .get(&(id, TypeId::of::<P>()))
            .map(|x| x.downcast_ref::<P>().unwrap() as &dyn Renderable)
    }
    pub fn get_dynamic<T: 'static>(&self, id: usize) -> Option<&T> {
        self.dynamic_items
            .get(&(id, TypeId::of::<T>()))
            .map(|x| x.downcast_ref::<T>().unwrap())
    }
    pub fn get_dynamic_or_init<T: Default + 'static>(&mut self, id: usize) -> &mut T {
        self.dynamic_items
            .entry((id, TypeId::of::<T>()))
            .or_insert_with(|| Box::new(T::default()))
            .downcast_mut::<T>()
            .unwrap()
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
