use std::ops::Deref;

use crate::{
    ResolutionInfo, WgpuContext,
    primitives::{mesh_items::MeshItemsBuffer, viewport::ViewportBindGroup},
    resource::{GpuResource, OUTPUT_TEXTURE_FORMAT},
};

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
                    &ResolutionInfo::create_bind_group_layout(ctx),
                    &ViewportBindGroup::bind_group_layout(ctx),
                    &MeshItemsBuffer::render_bind_group_layout(ctx),
                ],
                push_constant_ranges: &[],
            });
        let vertex_buffer_layouts = MeshItemsBuffer::vertex_buffer_layouts();
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
                    &ResolutionInfo::create_bind_group_layout(ctx),
                    &ViewportBindGroup::bind_group_layout(ctx),
                    &MeshItemsBuffer::render_bind_group_layout(ctx),
                ],
                push_constant_ranges: &[],
            });
        let vertex_buffer_layouts = MeshItemsBuffer::vertex_buffer_layouts();
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
