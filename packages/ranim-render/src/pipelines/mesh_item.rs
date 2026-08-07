use std::ops::Deref;

use bevy_ecs::prelude::*;

use crate::{
    ResolutionInfo, WgpuContext,
    primitives::{
        mesh_items::MeshItemsBuffer,
        viewport::{ViewportBindGroup, ViewportGpuPacket},
    },
    resource::{GpuResource, OUTPUT_TEXTURE_FORMAT, PipelinesPool},
    schedule::{FrameTarget, RenderContext},
};

pub(crate) fn depth(
    mut render: RenderContext,
    ctx: Res<WgpuContext>,
    pipelines: Res<PipelinesPool>,
    resolution: Res<ResolutionInfo>,
    target: Res<FrameTarget>,
    viewport: Res<ViewportGpuPacket>,
    merged: Res<MeshItemsBuffer>,
) {
    if merged.item_count() == 0 {
        return;
    }
    let mut pass = render
        .encoder()
        .begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Merged MeshItem Depth Render Pass"),
            color_attachments: &[],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &target.depth_stencil_view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
    pass.set_pipeline(&pipelines.get_or_init::<MeshItemDepthPipeline>(&ctx));
    pass.set_bind_group(0, &resolution.bind_group, &[]);
    pass.set_bind_group(1, &viewport.uniforms_bind_group.bind_group, &[]);
    pass.set_bind_group(2, merged.render_bind_group.as_ref().unwrap(), &[]);
    pass.set_vertex_buffer(0, merged.vertices_buffer.buffer.slice(..));
    pass.set_vertex_buffer(1, merged.mesh_ids_buffer.buffer.slice(..));
    pass.set_vertex_buffer(2, merged.vertex_colors_buffer.buffer.slice(..));
    pass.set_vertex_buffer(3, merged.vertex_normals_buffer.buffer.slice(..));
    pass.set_index_buffer(
        merged.indices_buffer.buffer.slice(..),
        wgpu::IndexFormat::Uint32,
    );
    pass.draw_indexed(0..merged.total_indices(), 0, 0..1);
}

pub(crate) fn color(
    mut render: RenderContext,
    ctx: Res<WgpuContext>,
    pipelines: Res<PipelinesPool>,
    resolution: Res<ResolutionInfo>,
    target: Res<FrameTarget>,
    viewport: Res<ViewportGpuPacket>,
    merged: Res<MeshItemsBuffer>,
) {
    if merged.item_count() == 0 {
        return;
    }
    let mut pass = render
        .encoder()
        .begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Merged MeshItem Color Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &target.render_view,
                resolve_target: None,
                depth_slice: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &target.depth_stencil_view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
    pass.set_pipeline(&pipelines.get_or_init::<MeshItemColorPipeline>(&ctx));
    pass.set_bind_group(0, &resolution.bind_group, &[]);
    pass.set_bind_group(1, &viewport.uniforms_bind_group.bind_group, &[]);
    pass.set_bind_group(2, merged.render_bind_group.as_ref().unwrap(), &[]);
    pass.set_vertex_buffer(0, merged.vertices_buffer.buffer.slice(..));
    pass.set_vertex_buffer(1, merged.mesh_ids_buffer.buffer.slice(..));
    pass.set_vertex_buffer(2, merged.vertex_colors_buffer.buffer.slice(..));
    pass.set_vertex_buffer(3, merged.vertex_normals_buffer.buffer.slice(..));
    pass.set_index_buffer(
        merged.indices_buffer.buffer.slice(..),
        wgpu::IndexFormat::Uint32,
    );
    pass.draw_indexed(0..merged.total_indices(), 0, 0..1);
}

pub struct MeshItemColorPipeline {
    pipeline: wgpu::RenderPipeline,
}

impl Deref for MeshItemColorPipeline {
    type Target = wgpu::RenderPipeline;
    fn deref(&self) -> &Self::Target {
        &self.pipeline
    }
}

impl GpuResource for MeshItemColorPipeline {
    fn new(ctx: &WgpuContext) -> Self {
        let module = &ctx
            .device
            .create_shader_module(wgpu::include_wgsl!("./shaders/mesh_item.wgsl"));
        let layout = ctx
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("MeshItem Color Pipeline Layout"),
                bind_group_layouts: &[
                    Some(&ResolutionInfo::create_bind_group_layout(ctx)),
                    Some(&ViewportBindGroup::bind_group_layout(ctx)),
                    Some(&MeshItemsBuffer::render_bind_group_layout(ctx)),
                ],
                immediate_size: 0,
            });
        let vertex_buffer_layouts = MeshItemsBuffer::vertex_buffer_layouts().map(Some);
        let pipeline = ctx
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("MeshItem Color Pipeline"),
                layout: Some(&layout),
                vertex: wgpu::VertexState {
                    module,
                    entry_point: Some("vs_main"),
                    buffers: &vertex_buffer_layouts,
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module,
                    entry_point: Some("fs_color"),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: OUTPUT_TEXTURE_FORMAT,
                        blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    ..Default::default()
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: wgpu::TextureFormat::Depth32Float,
                    depth_write_enabled: Some(false),
                    depth_compare: Some(wgpu::CompareFunction::LessEqual),
                    stencil: wgpu::StencilState::default(),
                    bias: wgpu::DepthBiasState::default(),
                }),
                multisample: wgpu::MultisampleState {
                    count: 1,
                    mask: !0,
                    alpha_to_coverage_enabled: false,
                },
                multiview_mask: None,
                cache: None,
            });
        Self { pipeline }
    }
}

pub struct MeshItemDepthPipeline {
    pipeline: wgpu::RenderPipeline,
}

impl Deref for MeshItemDepthPipeline {
    type Target = wgpu::RenderPipeline;
    fn deref(&self) -> &Self::Target {
        &self.pipeline
    }
}

impl GpuResource for MeshItemDepthPipeline {
    fn new(ctx: &WgpuContext) -> Self {
        let module = &ctx
            .device
            .create_shader_module(wgpu::include_wgsl!("./shaders/mesh_item.wgsl"));
        let layout = ctx
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("MeshItem Depth Pipeline Layout"),
                bind_group_layouts: &[
                    Some(&ResolutionInfo::create_bind_group_layout(ctx)),
                    Some(&ViewportBindGroup::bind_group_layout(ctx)),
                    Some(&MeshItemsBuffer::render_bind_group_layout(ctx)),
                ],
                immediate_size: 0,
            });
        let vertex_buffer_layouts = MeshItemsBuffer::vertex_buffer_layouts().map(Some);
        let pipeline = ctx
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("MeshItem Depth Pipeline"),
                layout: Some(&layout),
                vertex: wgpu::VertexState {
                    module,
                    entry_point: Some("vs_main"),
                    buffers: &vertex_buffer_layouts,
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module,
                    entry_point: Some("fs_depth"),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    targets: &[],
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    ..Default::default()
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: wgpu::TextureFormat::Depth32Float,
                    depth_write_enabled: Some(true),
                    depth_compare: Some(wgpu::CompareFunction::Less),
                    stencil: wgpu::StencilState::default(),
                    bias: wgpu::DepthBiasState::default(),
                }),
                multisample: wgpu::MultisampleState {
                    count: 1,
                    mask: !0,
                    alpha_to_coverage_enabled: false,
                },
                multiview_mask: None,
                cache: None,
            });
        Self { pipeline }
    }
}
