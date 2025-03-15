use crate::render::RenderTextures;

use super::{RenderInstance, vitem::VItemPrimitive};

#[derive(Default)]
pub struct SvgItemPrimitive {
    pub(crate) vitem_primitives: Vec<VItemPrimitive>,
}

impl RenderInstance for SvgItemPrimitive {
    fn encode_render_command(
        &self,
        ctx: &crate::context::WgpuContext,
        pipelines: &mut crate::render::PipelinesStorage,
        encoder: &mut wgpu::CommandEncoder,
        uniforms_bind_group: &wgpu::BindGroup,
        render_textures: &RenderTextures,
    ) {
        // trace!("SvgItemPrimitive encode_render_command vitem_primitives: {}", self.vitem_primitives.len());
        self.vitem_primitives.iter().for_each(|vimte_primitive| {
            vimte_primitive.encode_render_command(
                ctx,
                pipelines,
                encoder,
                uniforms_bind_group,
                render_textures,
            );
        });
    }
}
