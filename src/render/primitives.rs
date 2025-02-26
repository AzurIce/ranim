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
        &mut self,
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
    // Entity T -> Extract<T>
    inner: HashMap<(Id, TypeId), Box<dyn Any>>,
}

impl RenderInstances {
    pub fn get_or_init<T: Entity + 'static>(&mut self, id: Id) -> &mut T::Primitive {
        self.inner
            .entry((id, TypeId::of::<T>()))
            .or_insert_with(|| Box::new(T::Primitive::default()))
            .downcast_mut::<T::Primitive>()
            .unwrap()
    }
}
