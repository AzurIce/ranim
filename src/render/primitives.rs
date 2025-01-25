pub mod vitem;

use crate::{context::WgpuContext, utils::RenderResourceStorage};

pub trait Primitive {
    type Data;
    fn init(wgpu_ctx: &WgpuContext, data: &Self::Data) -> Self;
    fn update(&mut self, wgpu_ctx: &WgpuContext, data: &Self::Data);
    fn start_compute_pass<'a>(
        wgpu_ctx: &WgpuContext,
        pipelines: &mut RenderResourceStorage,
        encoder: &'a mut wgpu::CommandEncoder,
        uniforms_bind_group: &wgpu::BindGroup,
    ) -> wgpu::ComputePass<'a>;
    fn compute_command(&self, cpass: &mut wgpu::ComputePass);
    fn start_render_pass<'a>(
        wgpu_ctx: &WgpuContext,
        pipelines: &mut crate::utils::RenderResourceStorage,
        encoder: &'a mut wgpu::CommandEncoder,
        multisample_view: &wgpu::TextureView,
        target_view: &wgpu::TextureView,
        depth_stencil_view: &wgpu::TextureView,
        uniforms_bind_group: &wgpu::BindGroup,
    ) -> wgpu::RenderPass<'a>;

    fn render_command(&self, rpass: &mut wgpu::RenderPass);
}
