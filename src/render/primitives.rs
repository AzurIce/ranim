pub mod vitem;

use crate::{context::WgpuContext, utils::RenderResourceStorage};

use super::CameraFrame;

pub trait Primitive {
    type Data;
    fn init(wgpu_ctx: &WgpuContext, data: &Self::Data) -> Self;
    fn update(&mut self, wgpu_ctx: &WgpuContext, data: &Self::Data);
    #[allow(unused)]
    fn update_clip_info(&mut self, ctx: &WgpuContext, camera: &CameraFrame) {}
    fn encode_render_command(
        &mut self,
        ctx: &crate::context::WgpuContext,
        pipelines: &mut RenderResourceStorage,
        encoder: &mut wgpu::CommandEncoder,
        uniforms_bind_group: &wgpu::BindGroup,
        multisample_view: &wgpu::TextureView,
        target_view: &wgpu::TextureView,
    );
}
