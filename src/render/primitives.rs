pub mod svg_item;
pub mod vitem;

use std::{
    any::{Any, TypeId},
    collections::HashMap,
};

use crate::{context::WgpuContext, items::Entity, utils::Id};

use super::RenderTextures;

pub trait RenderInstance {
    fn encode_render_command(
        &self,
        ctx: &WgpuContext,
        pipelines: &mut super::PipelinesStorage,
        encoder: &mut wgpu::CommandEncoder,
        uniforms_bind_group: &wgpu::BindGroup,
        render_textures: &RenderTextures,
    );
}

pub trait ExtractFrom<T: Entity>: RenderInstance + Any {
    fn update_from(&mut self, ctx: &WgpuContext, data: &T);
}

#[derive(Default)]
pub struct RenderInstances {
    // Entity Id, EntityTypeId -> Extract<T>
    dynamic_items: HashMap<(Id, TypeId), Box<dyn Any>>,
}

impl RenderInstances {
    pub fn get_dynamic<T: Entity + 'static>(&self, id: Id) -> Option<&T::Primitive> {
        self.dynamic_items
            .get(&(id, TypeId::of::<T>()))
            .map(|x| x.downcast_ref::<T::Primitive>().unwrap())
    }
    pub fn get_dynamic_or_init<T: Entity + 'static>(&mut self, id: Id) -> &mut T::Primitive {
        self.dynamic_items
            .entry((id, TypeId::of::<T>()))
            .or_insert_with(|| Box::new(T::Primitive::default()))
            .downcast_mut::<T::Primitive>()
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
    ) {
        for render_instance in self {
            render_instance.encode_render_command(
                ctx,
                pipelines,
                encoder,
                uniforms_bind_group,
                render_textures,
            );
        }
    }
}
