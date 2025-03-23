pub mod svg_item;
pub mod vitem;

use std::{
    any::{Any, TypeId},
    collections::HashMap,
};

use crate::{context::WgpuContext, items::Entity};

use super::RenderTextures;

pub trait RenderInstance {
    fn encode_render_command(
        &self,
        ctx: &WgpuContext,
        pipelines: &mut super::PipelinesStorage,
        encoder: &mut wgpu::CommandEncoder,
        uniforms_bind_group: &wgpu::BindGroup,
        render_textures: &RenderTextures,
        #[cfg(feature = "profiling")]
        profiler: &mut wgpu_profiler::GpuProfiler,
    );
}

pub trait ExtractFrom<T: Entity>: RenderInstance + Any {
    fn update_from(&mut self, ctx: &WgpuContext, data: &T);
}

#[derive(Default)]
pub struct RenderInstances {
    // Rabject's id, RenderInstance's TypeId -> RenderInstance
    dynamic_items: HashMap<(usize, TypeId), Box<dyn Any>>,
}

impl RenderInstances {
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

impl RenderInstance for Vec<&dyn RenderInstance> {
    fn encode_render_command(
        &self,
        ctx: &WgpuContext,
        pipelines: &mut super::PipelinesStorage,
        encoder: &mut wgpu::CommandEncoder,
        uniforms_bind_group: &wgpu::BindGroup,
        render_textures: &RenderTextures,
        #[cfg(feature = "profiling")]
        profiler: &mut wgpu_profiler::GpuProfiler,
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
}
