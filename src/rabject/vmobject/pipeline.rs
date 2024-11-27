use std::ops::Deref;

use glam::Vec4;
use wgpu::include_wgsl;

use crate::{camera::CameraUniformsBindGroup, RanimContext, WgpuContext};

use crate::pipeline::{Pipeline, Vertex};

use super::VMObjectRenderResource;

#[derive(Default, Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
pub struct VMobjectFillVertex {
    pub pos: Vec4,
    pub fill_color: Vec4,
    pub unit_normal: Vec4,
}

impl Vertex for VMobjectFillVertex {
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        use std::mem::size_of;
        wgpu::VertexBufferLayout {
            array_stride: size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: size_of::<[f32; 4]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: size_of::<[f32; 8]>() as wgpu::BufferAddress,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        }
    }
}

pub struct FillPipeline {
    pub pipeline: wgpu::RenderPipeline,
}

impl Deref for FillPipeline {
    type Target = wgpu::RenderPipeline;
    fn deref(&self) -> &Self::Target {
        &self.pipeline
    }
}

impl FillPipeline {
    fn output_format() -> wgpu::TextureFormat {
        wgpu::TextureFormat::Rgba8UnormSrgb
    }

    fn pipeline_layout(ctx: &WgpuContext) -> wgpu::PipelineLayout {
        ctx.device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("VMobject Fill Pipeline Layout"),
                bind_group_layouts: &[&CameraUniformsBindGroup::bind_group_layout(ctx)],
                push_constant_ranges: &[],
            })
    }
}

impl Pipeline for FillPipeline {
    fn new(ctx: &RanimContext) -> Self {
        let WgpuContext { device, .. } = &ctx.wgpu_ctx;

        let module =
            &device.create_shader_module(include_wgsl!("../../../shader/vmobject_fill.wgsl"));
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("VMobject Fill Pipeline"),
            layout: Some(&Self::pipeline_layout(&ctx.wgpu_ctx)),
            vertex: wgpu::VertexState {
                module,
                entry_point: Some("vs_main"),
                buffers: &[VMobjectFillVertex::desc()],
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
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
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

pub struct StrokePipeline {
    pub pipeline: wgpu::RenderPipeline,
}

impl Deref for StrokePipeline {
    type Target = wgpu::RenderPipeline;
    fn deref(&self) -> &Self::Target {
        &self.pipeline
    }
}

impl StrokePipeline {
    fn output_format() -> wgpu::TextureFormat {
        wgpu::TextureFormat::Rgba8UnormSrgb
    }

    fn pipeline_layout(ctx: &WgpuContext) -> wgpu::PipelineLayout {
        ctx.device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("VMobject Fill Pipeline Layout"),
                bind_group_layouts: &[
                    &CameraUniformsBindGroup::bind_group_layout(ctx),
                    &VMObjectRenderResource::render_bind_group_layout(&ctx.device),
                ],
                push_constant_ranges: &[],
            })
    }
}

impl Pipeline for StrokePipeline {
    fn new(ctx: &RanimContext) -> Self {
        let WgpuContext { device, .. } = &ctx.wgpu_ctx;

        let module =
            &device.create_shader_module(include_wgsl!("../../../shader/vmobject_stroke.wgsl"));
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("VMobject Stroke Pipeline"),
            layout: Some(&Self::pipeline_layout(&ctx.wgpu_ctx)),
            vertex: wgpu::VertexState {
                module,
                entry_point: Some("vs_main"),
                // buffers: &[VMobjectStrokeVertex::desc()],
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
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleStrip,
                // topology: wgpu::PrimitiveTopology::PointList,
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

pub struct ComputePipeline {
    pub pipeline: wgpu::ComputePipeline,
}

impl Deref for ComputePipeline {
    type Target = wgpu::ComputePipeline;
    fn deref(&self) -> &Self::Target {
        &self.pipeline
    }
}

impl Pipeline for ComputePipeline {
    fn new(ctx: &RanimContext) -> Self {
        let WgpuContext { device, .. } = &ctx.wgpu_ctx;

        let module =
            &device.create_shader_module(include_wgsl!("../../../shader/vmobject_compute.wgsl"));

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("VMobject Compute Pipeline Layout"),
            bind_group_layouts: &[&VMObjectRenderResource::compute_bind_group_layout(
                &ctx.wgpu_ctx.device,
            )],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("VMobject Compute Pipeline"),
            layout: Some(&pipeline_layout),
            module,
            entry_point: Some("cs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });

        Self { pipeline }
    }
}
