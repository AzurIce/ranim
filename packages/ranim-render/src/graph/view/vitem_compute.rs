use crate::{
    RenderContext,
    graph::{RenderPacketsQuery, view::ViewRenderNodeTrait},
    pipelines::VItemComputePipeline,
    primitives::{viewport::ViewportGpuPacket, vitem::VItemRenderInstance},
};
pub struct VItemComputeNode;

impl ViewRenderNodeTrait for VItemComputeNode {
    type Query = VItemRenderInstance;

    fn run(
        &self,
        #[cfg(not(feature = "profiling"))] encoder: &mut wgpu::CommandEncoder,
        #[cfg(feature = "profiling")] encoder: &mut wgpu_profiler::Scope<'_, wgpu::CommandEncoder>,
        vitem2d_packets: <Self::Query as RenderPacketsQuery>::Output<'_>,
        ctx: RenderContext,
        _viewport: &ViewportGpuPacket,
    ) {
        #[cfg(feature = "profiling")]
        let mut encoder = encoder.scope("Compute Pass");
        // VItem2d Compute Pass
        {
            #[cfg(feature = "profiling")]
            let mut cpass = encoder.scoped_compute_pass("VItem2d Map Points Compute Pass");
            #[cfg(not(feature = "profiling"))]
            let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("VItem2d Map Points Compute Pass"),
                timestamp_writes: None,
            });
            cpass.set_pipeline(
                &ctx.pipelines
                    .get_or_init::<VItemComputePipeline>(ctx.wgpu_ctx),
            );

            vitem2d_packets
                .iter()
                .map(|h| ctx.render_pool.get_packet(h))
                .for_each(|vitem| vitem.encode_compute_pass_command(&mut cpass));
        }
    }
}
