use crate::{
    RenderContext, ViewportGpuPacket,
    graph::{RenderNodeTrait, RenderPacketsQuery},
    pipelines::{ClipBox2dPipeline, Map3dTo2dPipeline},
    primitives::{vitem::VItemRenderInstance, vitem2d::VItem2dRenderInstance},
};
pub struct VItemComputeRenderNode;

impl RenderNodeTrait for VItemComputeRenderNode {
    type Query = (VItemRenderInstance, VItem2dRenderInstance);

    fn run(
        &self,
        #[cfg(not(feature = "profiling"))] encoder: &mut wgpu::CommandEncoder,
        #[cfg(feature = "profiling")] scope: &mut wgpu_profiler::Scope<'_, wgpu::CommandEncoder>,
        (vitem_packets, vitem2d_packets): <Self::Query as RenderPacketsQuery>::Output<'_>,
        ctx: &mut RenderContext,
        viewport: &ViewportGpuPacket,
    ) {
        #[cfg(feature = "profiling")]
        let mut scope = scope.scope("Compute Pass");
        // VItem Compute Pass
        {
            #[cfg(feature = "profiling")]
            let mut cpass = scope.scoped_compute_pass("VItem Map Points Compute Pass");
            #[cfg(not(feature = "profiling"))]
            let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("VItem Map Points Compute Pass"),
                timestamp_writes: None,
            });
            cpass.set_pipeline(ctx.pipelines.get_or_init::<Map3dTo2dPipeline>(ctx.wgpu_ctx));
            cpass.set_bind_group(0, &viewport.uniforms_bind_group.bind_group, &[]);

            vitem_packets
                .iter()
                .map(|h| ctx.render_pool.get_packet(h))
                .for_each(|vitem| vitem.encode_compute_pass_command(&mut cpass));
        }
        // VItem2d Compute Pass
        {
            #[cfg(feature = "profiling")]
            let mut cpass = scope.scoped_compute_pass("VItem2d Map Points Compute Pass");
            #[cfg(not(feature = "profiling"))]
            let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("VItem2d Map Points Compute Pass"),
                timestamp_writes: None,
            });
            cpass.set_pipeline(ctx.pipelines.get_or_init::<ClipBox2dPipeline>(ctx.wgpu_ctx));

            vitem2d_packets
                .iter()
                .map(|h| ctx.render_pool.get_packet(h))
                .for_each(|vitem| vitem.encode_compute_pass_command(&mut cpass));
        }
    }
}
