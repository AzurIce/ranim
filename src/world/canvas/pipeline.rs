use std::ops::Deref;

use crate::{
    context::WgpuContext,
    rabject::{RenderResource, Vertex},
    render::{CameraUniformsBindGroup, OUTPUT_TEXTURE_FORMAT},
};

use super::{camera::TextureBindGroup, CanvasVertex};

pub struct BlendPipeline {
    pub pipeline: wgpu::RenderPipeline,
}

impl Deref for BlendPipeline {
    type Target = wgpu::RenderPipeline;
    fn deref(&self) -> &Self::Target {
        &self.pipeline
    }
}

impl BlendPipeline {
    fn output_format() -> wgpu::TextureFormat {
        OUTPUT_TEXTURE_FORMAT
    }

    fn pipeline_layout(ctx: &WgpuContext) -> wgpu::PipelineLayout {
        ctx.device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Blend Pipeline Layout"),
                bind_group_layouts: &[&TextureBindGroup::bind_group_layout(ctx)],
                push_constant_ranges: &[],
            })
    }
}

impl RenderResource for BlendPipeline {
    fn new(wgpu_ctx: &WgpuContext) -> Self {
        let WgpuContext { device, .. } = wgpu_ctx;

        let module = &device.create_shader_module(wgpu::include_wgsl!("./shader/blend.wgsl"));
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Blend Pipeline"),
            layout: Some(&Self::pipeline_layout(wgpu_ctx)),
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
                    format: Self::output_format(),
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 4,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        });

        Self { pipeline }
    }
}

pub struct CanvasPipeline {
    pub pipeline: wgpu::RenderPipeline,
}

impl Deref for CanvasPipeline {
    type Target = wgpu::RenderPipeline;
    fn deref(&self) -> &Self::Target {
        &self.pipeline
    }
}

impl CanvasPipeline {
    fn output_format() -> wgpu::TextureFormat {
        OUTPUT_TEXTURE_FORMAT
    }

    fn pipeline_layout(ctx: &WgpuContext) -> wgpu::PipelineLayout {
        ctx.device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Canvas Pipeline Layout"),
                bind_group_layouts: &[
                    &CameraUniformsBindGroup::bind_group_layout(ctx),
                    &TextureBindGroup::bind_group_layout(ctx),
                ],
                push_constant_ranges: &[],
            })
    }
}

impl RenderResource for CanvasPipeline {
    fn new(wgpu_ctx: &WgpuContext) -> Self {
        let WgpuContext { device, .. } = wgpu_ctx;

        let module = &device.create_shader_module(wgpu::include_wgsl!("./shader/canvas.wgsl"));
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Canvas Pipeline"),
            layout: Some(&Self::pipeline_layout(wgpu_ctx)),
            vertex: wgpu::VertexState {
                module,
                entry_point: Some("vs_main"),
                buffers: &[CanvasVertex::desc()],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module,
                entry_point: Some("fs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: Self::output_format(),
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                // topology: wgpu::PrimitiveTopology::PointList,
                topology: wgpu::PrimitiveTopology::TriangleStrip,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth24PlusStencil8,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: 4,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        });

        Self { pipeline }
    }
}
