use crate::{
    RenderContext, RenderTextures,
    graph::{RenderPacketsQuery, view::ViewRenderNodeTrait},
    pipelines::{VItem2dColorPipeline, VItemPipeline},
    primitives::{
        viewport::ViewportGpuPacket, vitem::VItemRenderInstance, vitem2d::VItem2dRenderInstance,
    },
};

pub struct VItemRenderNode;

impl ViewRenderNodeTrait for VItemRenderNode {
    type Query = VItemRenderInstance;
    fn run(
        &self,
        #[cfg(not(feature = "profiling"))] encoder: &mut wgpu::CommandEncoder,
        #[cfg(feature = "profiling")] scope: &mut wgpu_profiler::Scope<'_, wgpu::CommandEncoder>,
        vitem_packets: <Self::Query as RenderPacketsQuery>::Output<'_>,
        ctx: RenderContext,
        viewport: &ViewportGpuPacket,
    ) {
        let RenderTextures {
            // multisample_view,
            render_view,
            depth_stencil_view,
            ..
        } = ctx.render_textures;
        let rpass_desc = wgpu::RenderPassDescriptor {
            label: Some("VItem Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                // view: multisample_view,
                // resolve_target: Some(render_view),
                depth_slice: None,
                view: render_view,
                resolve_target: None,
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
        let mut rpass = scope.scoped_render_pass("VItem Render Pass", rpass_desc);
        #[cfg(not(feature = "profiling"))]
        let mut rpass = encoder.begin_render_pass(&rpass_desc);
        rpass.set_pipeline(&ctx.pipelines.get_or_init::<VItemPipeline>(ctx.wgpu_ctx));
        rpass.set_bind_group(0, &viewport.uniforms_bind_group.bind_group, &[]);
        vitem_packets
            .iter()
            .map(|h| ctx.render_pool.get_packet(h))
            .for_each(|vitem| vitem.encode_render_pass_command(&mut rpass));
    }
}

pub struct VItem2dColorNode;

impl ViewRenderNodeTrait for VItem2dColorNode {
    type Query = VItem2dRenderInstance;
    fn run(
        &self,
        #[cfg(not(feature = "profiling"))] encoder: &mut wgpu::CommandEncoder,
        #[cfg(feature = "profiling")] encoder: &mut wgpu_profiler::Scope<'_, wgpu::CommandEncoder>,
        vitem2d_packets: <Self::Query as RenderPacketsQuery>::Output<'_>,
        ctx: RenderContext,
        viewport: &ViewportGpuPacket,
    ) {
        // VItem2d Render Pass
        let RenderTextures {
            render_view,
            depth_stencil_view,
            ..
        } = ctx.render_textures;
        let rpass_desc = wgpu::RenderPassDescriptor {
            label: Some("VItem2d Render Pass"),
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
        let mut rpass = encoder.scoped_render_pass("VItem2d Render Pass", rpass_desc);
        #[cfg(not(feature = "profiling"))]
        let mut rpass = encoder.begin_render_pass(&rpass_desc);
        rpass.set_pipeline(
            &ctx.pipelines
                .get_or_init::<VItem2dColorPipeline>(ctx.wgpu_ctx),
        );
        rpass.set_bind_group(0, &ctx.resolution_info.bind_group, &[]);
        rpass.set_bind_group(1, &viewport.uniforms_bind_group.bind_group, &[]);
        vitem2d_packets
            .iter()
            .map(|h| ctx.render_pool.get_packet(h))
            .for_each(|vitem| vitem.encode_render_pass_command(&mut rpass));
    }
}
