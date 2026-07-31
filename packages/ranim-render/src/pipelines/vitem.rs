use std::ops::Deref;

use bevy_ecs::prelude::*;

use crate::{
    ResolutionInfo, WgpuContext,
    primitives::{
        viewport::{ViewportBindGroup, ViewportGpuPacket},
        vitems::VItemsBuffer,
    },
    resource::{GpuResource, OUTPUT_TEXTURE_FORMAT, PipelinesPool},
    schedule::{FrameTarget, RenderContext},
};

pub(crate) fn compute(
    mut render: RenderContext,
    ctx: Res<WgpuContext>,
    pipelines: Res<PipelinesPool>,
    merged: Res<VItemsBuffer>,
) {
    if merged.item_count() == 0 {
        return;
    }
    let mut pass = render
        .encoder()
        .begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("Merged VItem Map Points Compute Pass"),
            timestamp_writes: None,
        });
    pass.set_pipeline(&pipelines.get_or_init::<VItemComputePipeline>(&ctx));
    pass.set_bind_group(0, merged.compute_bind_group.as_ref().unwrap(), &[]);
    pass.dispatch_workgroups(merged.total_points().div_ceil(256), 1, 1);
}

pub(crate) fn depth(
    mut render: RenderContext,
    ctx: Res<WgpuContext>,
    pipelines: Res<PipelinesPool>,
    resolution: Res<ResolutionInfo>,
    target: Res<FrameTarget>,
    viewport: Res<ViewportGpuPacket>,
    merged: Res<VItemsBuffer>,
) {
    if merged.item_count() == 0 {
        return;
    }
    let mut pass = render
        .encoder()
        .begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Merged VItem Depth Render Pass"),
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
    pass.set_pipeline(&pipelines.get_or_init::<VItemDepthPipeline>(&ctx));
    pass.set_bind_group(0, &resolution.bind_group, &[]);
    pass.set_bind_group(1, &viewport.uniforms_bind_group.bind_group, &[]);
    pass.set_bind_group(2, merged.render_bind_group.as_ref().unwrap(), &[]);
    pass.draw(0..4, 0..merged.item_count());
}

pub(crate) fn color(
    mut render: RenderContext,
    ctx: Res<WgpuContext>,
    pipelines: Res<PipelinesPool>,
    resolution: Res<ResolutionInfo>,
    target: Res<FrameTarget>,
    viewport: Res<ViewportGpuPacket>,
    merged: Res<VItemsBuffer>,
) {
    if merged.item_count() == 0 {
        return;
    }
    let mut pass = render
        .encoder()
        .begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Merged VItem Color Render Pass"),
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
    pass.set_pipeline(&pipelines.get_or_init::<VItemColorPipeline>(&ctx));
    pass.set_bind_group(0, &resolution.bind_group, &[]);
    pass.set_bind_group(1, &viewport.uniforms_bind_group.bind_group, &[]);
    pass.set_bind_group(2, merged.render_bind_group.as_ref().unwrap(), &[]);
    pass.draw(0..4, 0..merged.item_count());
}

// MARK: Compute pipeline

pub struct VItemComputePipeline {
    pipeline: wgpu::ComputePipeline,
}

impl Deref for VItemComputePipeline {
    type Target = wgpu::ComputePipeline;
    fn deref(&self) -> &Self::Target {
        &self.pipeline
    }
}

impl GpuResource for VItemComputePipeline {
    fn new(ctx: &WgpuContext) -> Self {
        let module = &ctx
            .device
            .create_shader_module(wgpu::include_wgsl!("./shaders/vitem_compute.wgsl"));
        let layout = ctx
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("VItem Compute Pipeline Layout"),
                bind_group_layouts: &[Some(&VItemsBuffer::compute_bind_group_layout(ctx))],
                immediate_size: 0,
            });
        let pipeline = ctx
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("VItem Compute Pipeline"),
                layout: Some(&layout),
                module,
                entry_point: Some("cs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                cache: None,
            });
        Self { pipeline }
    }
}

// MARK: Color pipeline

pub struct VItemColorPipeline {
    pipeline: wgpu::RenderPipeline,
}

impl Deref for VItemColorPipeline {
    type Target = wgpu::RenderPipeline;
    fn deref(&self) -> &Self::Target {
        &self.pipeline
    }
}

impl GpuResource for VItemColorPipeline {
    fn new(ctx: &WgpuContext) -> Self {
        let module = &ctx
            .device
            .create_shader_module(wgpu::include_wgsl!("./shaders/vitem.wgsl"));
        let layout = ctx
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("VItem Color Pipeline Layout"),
                bind_group_layouts: &[
                    Some(&ResolutionInfo::create_bind_group_layout(ctx)),
                    Some(&ViewportBindGroup::bind_group_layout(ctx)),
                    Some(&VItemsBuffer::render_bind_group_layout(ctx)),
                ],
                immediate_size: 0,
            });
        let pipeline = ctx
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("VItem Color Pipeline"),
                layout: Some(&layout),
                vertex: wgpu::VertexState {
                    module,
                    entry_point: Some("vs_main"),
                    buffers: &[],
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module,
                    entry_point: Some("fs_main"),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: OUTPUT_TEXTURE_FORMAT,
                        blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleStrip,
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

// MARK: Depth pipeline

pub struct VItemDepthPipeline {
    pipeline: wgpu::RenderPipeline,
}

impl Deref for VItemDepthPipeline {
    type Target = wgpu::RenderPipeline;
    fn deref(&self) -> &Self::Target {
        &self.pipeline
    }
}

impl GpuResource for VItemDepthPipeline {
    fn new(ctx: &WgpuContext) -> Self {
        let module = &ctx
            .device
            .create_shader_module(wgpu::include_wgsl!("./shaders/vitem.wgsl"));
        let layout = ctx
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("VItem Depth Pipeline Layout"),
                bind_group_layouts: &[
                    Some(&ResolutionInfo::create_bind_group_layout(ctx)),
                    Some(&ViewportBindGroup::bind_group_layout(ctx)),
                    Some(&VItemsBuffer::render_bind_group_layout(ctx)),
                ],
                immediate_size: 0,
            });
        let pipeline = ctx
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("VItem Depth Pipeline"),
                layout: Some(&layout),
                vertex: wgpu::VertexState {
                    module,
                    entry_point: Some("vs_main"),
                    buffers: &[],
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module,
                    entry_point: Some("fs_depth_only"),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    targets: &[],
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleStrip,
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
