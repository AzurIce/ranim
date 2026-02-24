use crate::{
    RenderContext, RenderTextures,
    graph::{RenderPacketsQuery, view::ViewRenderNodeTrait},
    pipelines::VItemDepthPipeline,
    primitives::viewport::ViewportGpuPacket,
};

pub struct MergedVItemDepthNode;

impl ViewRenderNodeTrait for MergedVItemDepthNode {
    type Query = ();

    fn run(
        &self,
        #[cfg(not(feature = "profiling"))] encoder: &mut wgpu::CommandEncoder,
        #[cfg(feature = "profiling")] encoder: &mut wgpu_profiler::Scope<'_, wgpu::CommandEncoder>,
        _packets: <Self::Query as RenderPacketsQuery>::Output<'_>,
        ctx: RenderContext,
        viewport: &ViewportGpuPacket,
    ) {
        let Some(merged) = ctx.merged_buffer else {
            return;
        };
        if merged.item_count() == 0 {
            return;
        }

        #[cfg(feature = "profiling")]
        let mut encoder = encoder.scope("Merged Depth Render Pass");

        {
            let RenderTextures {
                depth_stencil_view, ..
            } = ctx.render_textures;
            let rpass_desc = wgpu::RenderPassDescriptor {
                label: Some("Merged VItem Depth Render Pass"),
                color_attachments: &[],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: depth_stencil_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            };
            #[cfg(feature = "profiling")]
            let mut rpass =
                encoder.scoped_render_pass("Merged VItem Depth Render Pass", rpass_desc);
            #[cfg(not(feature = "profiling"))]
            let mut rpass = encoder.begin_render_pass(&rpass_desc);
            rpass.set_pipeline(
                &ctx.pipelines
                    .get_or_init::<VItemDepthPipeline>(ctx.wgpu_ctx),
            );
            rpass.set_bind_group(0, &ctx.resolution_info.bind_group, &[]);
            rpass.set_bind_group(1, &viewport.uniforms_bind_group.bind_group, &[]);
            rpass.set_bind_group(2, merged.render_bind_group.as_ref().unwrap(), &[]);
            rpass.draw(0..4, 0..merged.item_count());
        }
    }
}
