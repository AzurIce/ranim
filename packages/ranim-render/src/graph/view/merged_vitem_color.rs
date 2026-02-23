use crate::{
    graph::{view::ViewRenderNodeTrait, RenderPacketsQuery},
    pipelines::MergedVItemColorPipeline,
    primitives::viewport::ViewportGpuPacket,
    RenderContext, RenderTextures,
};

pub struct MergedVItemColorNode;

impl ViewRenderNodeTrait for MergedVItemColorNode {
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

        let RenderTextures {
            render_view,
            depth_stencil_view,
            ..
        } = ctx.render_textures;
        let rpass_desc = wgpu::RenderPassDescriptor {
            label: Some("Merged VItem Color Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: render_view,
                resolve_target: None,
                depth_slice: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
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
        let mut rpass = encoder.scoped_render_pass("Merged VItem Color Render Pass", rpass_desc);
        #[cfg(not(feature = "profiling"))]
        let mut rpass = encoder.begin_render_pass(&rpass_desc);
        rpass.set_pipeline(
            &ctx.pipelines
                .get_or_init::<MergedVItemColorPipeline>(ctx.wgpu_ctx),
        );
        rpass.set_bind_group(0, &ctx.resolution_info.bind_group, &[]);
        rpass.set_bind_group(1, &viewport.uniforms_bind_group.bind_group, &[]);
        rpass.set_bind_group(2, merged.render_bind_group.as_ref().unwrap(), &[]);
        rpass.draw(0..4, 0..merged.item_count());
    }
}
