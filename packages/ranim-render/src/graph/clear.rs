use crate::{RenderContext, graph::GlobalRenderNodeTrait, resource::RenderTextures};

pub struct ClearNode;

impl GlobalRenderNodeTrait for ClearNode {
    type Query = ();
    fn run(
        &self,
        #[cfg(not(feature = "profiling"))] encoder: &mut wgpu::CommandEncoder,
        #[cfg(feature = "profiling")] encoder: &mut wgpu_profiler::Scope<'_, wgpu::CommandEncoder>,
        _render_packets: <Self::Query as super::RenderPacketsQuery>::Output<'_>,
        render_ctx: RenderContext,
    ) {
        #[cfg(feature = "profiling")]
        profiling::scope!("clear_screen");
        let RenderTextures {
            render_view,
            // multisample_view,
            depth_stencil_view,
            ..
        } = render_ctx.render_textures;

        let pass_desc = wgpu::RenderPassDescriptor {
            label: Some("Clear Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                // view: multisample_view,
                // resolve_target: Some(render_view),
                depth_slice: None,
                view: render_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(render_ctx.clear_color),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: depth_stencil_view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            // depth_stencil_attachment: None,
            occlusion_query_set: None,
            timestamp_writes: None,
        };
        encoder.begin_render_pass(&pass_desc);
    }
}
