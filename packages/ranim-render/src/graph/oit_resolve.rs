use crate::{
    RenderContext, RenderTextures,
    graph::{GlobalRenderNodeTrait, RenderPacketsQuery},
    pipelines::OITResolvePipeline,
};

pub struct OITResolveNode;

impl GlobalRenderNodeTrait for OITResolveNode {
    type Query = ();
    fn run(
        &self,
        #[cfg(not(feature = "profiling"))] encoder: &mut wgpu::CommandEncoder,
        #[cfg(feature = "profiling")] encoder: &mut wgpu_profiler::Scope<'_, wgpu::CommandEncoder>,
        _packets: <Self::Query as RenderPacketsQuery>::Output<'_>,
        ctx: RenderContext,
    ) {
        let RenderTextures { render_view, .. } = ctx.render_textures;

        // OIT Resolve Pass: Blend the transparent layers onto the opaque background
        let rpass_desc = wgpu::RenderPassDescriptor {
            label: Some("OIT Resolve Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: render_view,
                resolve_target: None,
                depth_slice: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None, // No depth attachment
            timestamp_writes: None,
            occlusion_query_set: None,
        };

        {
            #[cfg(feature = "profiling")]
            let mut rpass = encoder.scoped_render_pass("OIT Resolve Pass", rpass_desc);
            #[cfg(not(feature = "profiling"))]
            let mut rpass = encoder.begin_render_pass(&rpass_desc);

            rpass.set_pipeline(
                &ctx.pipelines
                    .get_or_init::<OITResolvePipeline>(ctx.wgpu_ctx),
            );
            rpass.set_bind_group(0, &ctx.resolution_info.bind_group, &[]);
            rpass.set_bind_group(1, &ctx.render_textures.depth_bind_group, &[]);
            rpass.draw(0..3, 0..1);
        }
        // Clear OIT pixel count buffer
        encoder.clear_buffer(&ctx.resolution_info.pixel_count_buffer.buffer, 0, None);
    }
}
