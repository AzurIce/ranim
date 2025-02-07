use super::{vitem::VItemPrimitive, RenderInstance};

#[derive(Default)]
pub struct SvgItemPrimitive {
    clip_box: [glam::Vec2; 4],
    pub(crate) vitem_primitives: Vec<VItemPrimitive>,
}

impl SvgItemPrimitive {
    pub fn refresh_clip_box(&mut self, ctx: &crate::context::WgpuContext) {
        self.vitem_primitives
            .iter_mut()
            .for_each(|vitem_primitive| {
                vitem_primitive.update_clip_box(ctx, &self.clip_box);
            });
    }
}

impl RenderInstance for SvgItemPrimitive {
    fn update_clip_box(&mut self, ctx: &crate::context::WgpuContext, clip_box: &[glam::Vec2; 4]) {
        // trace!("SvgItemPrimitive update_clip_box vitem_primitives: {}", self.vitem_primitives.len());
        self.clip_box = *clip_box;
        self.vitem_primitives
            .iter_mut()
            .for_each(|vitem_primitive| {
                vitem_primitive.update_clip_box(ctx, clip_box);
            });
    }
    fn encode_render_command(
        &mut self,
        ctx: &crate::context::WgpuContext,
        pipelines: &mut crate::render::RenderResourceStorage,
        encoder: &mut wgpu::CommandEncoder,
        uniforms_bind_group: &wgpu::BindGroup,
        multisample_view: &wgpu::TextureView,
        target_view: &wgpu::TextureView,
    ) {
        // trace!("SvgItemPrimitive encode_render_command vitem_primitives: {}", self.vitem_primitives.len());
        self.vitem_primitives
            .iter_mut()
            .for_each(|vimte_primitive| {
                vimte_primitive.encode_render_command(
                    ctx,
                    pipelines,
                    encoder,
                    uniforms_bind_group,
                    multisample_view,
                    target_view,
                );
            });
    }
}
