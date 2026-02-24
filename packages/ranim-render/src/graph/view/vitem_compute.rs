use crate::{
    RenderContext,
    graph::{RenderPacketsQuery, view::ViewRenderNodeTrait},
    pipelines::VItemComputePipeline,
    primitives::viewport::ViewportGpuPacket,
};

pub struct MergedVItemComputeNode;

impl ViewRenderNodeTrait for MergedVItemComputeNode {
    type Query = ();

    fn run(
        &self,
        #[cfg(not(feature = "profiling"))] encoder: &mut wgpu::CommandEncoder,
        #[cfg(feature = "profiling")] encoder: &mut wgpu_profiler::Scope<'_, wgpu::CommandEncoder>,
        _packets: <Self::Query as RenderPacketsQuery>::Output<'_>,
        ctx: RenderContext,
        _viewport: &ViewportGpuPacket,
    ) {
        let Some(merged) = ctx.merged_buffer else {
            return;
        };
        if merged.item_count() == 0 {
            return;
        }

        #[cfg(feature = "profiling")]
        let mut encoder = encoder.scope("Merged Compute Pass");

        {
            #[cfg(feature = "profiling")]
            let mut cpass = encoder.scoped_compute_pass("Merged VItem Map Points Compute Pass");
            #[cfg(not(feature = "profiling"))]
            let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Merged VItem Map Points Compute Pass"),
                timestamp_writes: None,
            });
            cpass.set_pipeline(
                &ctx.pipelines
                    .get_or_init::<VItemComputePipeline>(ctx.wgpu_ctx),
            );
            cpass.set_bind_group(0, merged.compute_bind_group.as_ref().unwrap(), &[]);
            cpass.dispatch_workgroups(merged.total_points().div_ceil(256), 1, 1);
        }
    }
}
