use crate::{
    RenderContext, RenderTextures,
    graph::{RenderPacketsQuery, view::ViewRenderNodeTrait},
    pipelines::MeshItemColorPipeline,
    primitives::viewport::ViewportGpuPacket,
};

pub struct MergedMeshItemColorNode;

impl ViewRenderNodeTrait for MergedMeshItemColorNode {
    type Query = ();

    fn run(
        &self,
        #[cfg(not(feature = "profiling"))] encoder: &mut wgpu::CommandEncoder,
        #[cfg(feature = "profiling")] encoder: &mut wgpu_profiler::Scope<'_, wgpu::CommandEncoder>,
        _packets: <Self::Query as RenderPacketsQuery>::Output<'_>,
        ctx: RenderContext,
        viewport: &ViewportGpuPacket,
    ) {
        let Some(merged) = ctx.merged_mesh_buffer else {
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
            label: Some("Merged MeshItem Color Render Pass"),
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
        let mut rpass = encoder.scoped_render_pass("Merged MeshItem Color Render Pass", rpass_desc);
        #[cfg(not(feature = "profiling"))]
        let mut rpass = encoder.begin_render_pass(&rpass_desc);
        rpass.set_pipeline(
            &ctx.pipelines
                .get_or_init::<MeshItemColorPipeline>(ctx.wgpu_ctx),
        );
        rpass.set_bind_group(0, &ctx.resolution_info.bind_group, &[]);
        rpass.set_bind_group(1, &viewport.uniforms_bind_group.bind_group, &[]);
        rpass.set_bind_group(2, merged.render_bind_group.as_ref().unwrap(), &[]);
        rpass.set_vertex_buffer(0, merged.vertices_buffer.buffer.slice(..));
        rpass.set_vertex_buffer(1, merged.mesh_ids_buffer.buffer.slice(..));
        rpass.set_vertex_buffer(2, merged.vertex_colors_buffer.buffer.slice(..));
        rpass.set_vertex_buffer(3, merged.vertex_normals_buffer.buffer.slice(..));
        rpass.set_index_buffer(
            merged.indices_buffer.buffer.slice(..),
            wgpu::IndexFormat::Uint32,
        );
        rpass.draw_indexed(0..merged.total_indices(), 0, 0..1);
    }
}
