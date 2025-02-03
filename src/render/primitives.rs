pub mod vitem;
pub mod svg_item;

use glam::Vec2;

use crate::{context::WgpuContext, items::Entity};

pub trait Primitive {
    #[allow(unused)]
    fn update_clip_box(&mut self, ctx: &WgpuContext, clip_box: &[Vec2; 4]) {}
    fn encode_render_command(
        &mut self,
        ctx: &WgpuContext,
        pipelines: &mut super::RenderResourceStorage,
        encoder: &mut wgpu::CommandEncoder,
        uniforms_bind_group: &wgpu::BindGroup,
        multisample_view: &wgpu::TextureView,
        target_view: &wgpu::TextureView,
    );
}

pub trait Extract<T: Entity>: Primitive {
    fn update(&mut self, ctx: &WgpuContext, data: &T);
}
