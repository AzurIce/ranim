use std::ops::Deref;

use crate::{
    primitives::{merged_vitem::MergedVItemBuffer, viewport::ViewportBindGroup},
    resource::{GpuResource, OUTPUT_TEXTURE_FORMAT},
    ResolutionInfo, WgpuContext,
};

// MARK: Compute pipeline

pub struct MergedVItemComputePipeline {
    pipeline: wgpu::ComputePipeline,
}

impl Deref for MergedVItemComputePipeline {
    type Target = wgpu::ComputePipeline;
    fn deref(&self) -> &Self::Target {
        &self.pipeline
    }
}

impl GpuResource for MergedVItemComputePipeline {
    fn new(ctx: &WgpuContext) -> Self {
        let module = &ctx
            .device
            .create_shader_module(wgpu::include_wgsl!("./shaders/merged_vitem_compute.wgsl"));
        let layout = ctx
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Merged VItem Compute Pipeline Layout"),
                bind_group_layouts: &[&MergedVItemBuffer::compute_bind_group_layout(ctx)],
                push_constant_ranges: &[],
            });
        let pipeline = ctx
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("Merged VItem Compute Pipeline"),
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

pub struct MergedVItemColorPipeline {
    pipeline: wgpu::RenderPipeline,
}

impl Deref for MergedVItemColorPipeline {
    type Target = wgpu::RenderPipeline;
    fn deref(&self) -> &Self::Target {
        &self.pipeline
    }
}

impl GpuResource for MergedVItemColorPipeline {
    fn new(ctx: &WgpuContext) -> Self {
        let module = &ctx
            .device
            .create_shader_module(wgpu::include_wgsl!("./shaders/merged_vitem.wgsl"));
        let layout = ctx
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Merged VItem Color Pipeline Layout"),
                bind_group_layouts: &[
                    &ResolutionInfo::create_bind_group_layout(ctx),
                    &ViewportBindGroup::bind_group_layout(ctx),
                    &MergedVItemBuffer::render_bind_group_layout(ctx),
                ],
                push_constant_ranges: &[],
            });
        let pipeline = ctx
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Merged VItem Color Pipeline"),
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
                    depth_write_enabled: false,
                    depth_compare: wgpu::CompareFunction::LessEqual,
                    stencil: wgpu::StencilState::default(),
                    bias: wgpu::DepthBiasState::default(),
                }),
                multisample: wgpu::MultisampleState {
                    count: 1,
                    mask: !0,
                    alpha_to_coverage_enabled: false,
                },
                multiview: None,
                cache: None,
            });
        Self { pipeline }
    }
}

// MARK: Depth pipeline

pub struct MergedVItemDepthPipeline {
    pipeline: wgpu::RenderPipeline,
}

impl Deref for MergedVItemDepthPipeline {
    type Target = wgpu::RenderPipeline;
    fn deref(&self) -> &Self::Target {
        &self.pipeline
    }
}

impl GpuResource for MergedVItemDepthPipeline {
    fn new(ctx: &WgpuContext) -> Self {
        let module = &ctx
            .device
            .create_shader_module(wgpu::include_wgsl!("./shaders/merged_vitem.wgsl"));
        let layout = ctx
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Merged VItem Depth Pipeline Layout"),
                bind_group_layouts: &[
                    &ResolutionInfo::create_bind_group_layout(ctx),
                    &ViewportBindGroup::bind_group_layout(ctx),
                    &MergedVItemBuffer::render_bind_group_layout(ctx),
                ],
                push_constant_ranges: &[],
            });
        let pipeline = ctx
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Merged VItem Depth Pipeline"),
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
                    depth_write_enabled: true,
                    depth_compare: wgpu::CompareFunction::Less,
                    stencil: wgpu::StencilState::default(),
                    bias: wgpu::DepthBiasState::default(),
                }),
                multisample: wgpu::MultisampleState {
                    count: 1,
                    mask: !0,
                    alpha_to_coverage_enabled: false,
                },
                multiview: None,
                cache: None,
            });
        Self { pipeline }
    }
}
