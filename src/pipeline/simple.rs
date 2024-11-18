use std::ops::{Deref, DerefMut};

use bytemuck::{Pod, Zeroable};
use glam::{vec3, vec4, Vec3, Vec4};
use wgpu::include_wgsl;

use crate::{
    camera::{CameraUniforms, CameraUniformsBindGroup},
    WgpuContext,
};

use super::{PipelineVertex, RenderPipeline};

#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
pub struct Vertex {
    pub position: Vec3,
    _padding: f32,
    pub color: Vec4,
}

impl PipelineVertex for Vertex {
    type Pipeline = Pipeline;

    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        use std::mem::size_of;
        wgpu::VertexBufferLayout {
            array_stride: size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: size_of::<[f32; 4]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        }
    }
}

impl Into<Vec<Vertex>> for Vertices {
    fn into(self) -> Vec<Vertex> {
        self.0
    }
}

pub struct Vertices(Vec<Vertex>);

impl Into<Vertices> for Vec<Vertex> {
    fn into(self) -> Vertices {
        Vertices(self)
    }
}

impl Deref for Vertices {
    type Target = Vec<Vertex>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Vertices {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Vertex {
    pub fn new(position: Vec3, color: Vec4) -> Self {
        Self {
            position,
            _padding: 0.0,
            color,
        }
    }
}

impl Vertex {
    pub fn test_data() -> Vec<Self> {
        vec![
            Self {
                position: vec3(0.0, 0.0, 0.0),
                _padding: 0.0,
                color: vec4(1.0, 0.0, 0.0, 1.0),
            },
            Self {
                position: vec3(0.0, 1.0, 0.0),
                _padding: 0.0,
                color: vec4(0.0, 1.0, 0.0, 1.0),
            },
            Self {
                position: vec3(1.0, 0.0, 0.0),
                _padding: 0.0,
                color: vec4(0.0, 0.0, 1.0, 1.0),
            },
        ]
    }
}

pub struct Pipeline {
    pub pipeline: wgpu::RenderPipeline,
}

impl Deref for Pipeline {
    type Target = wgpu::RenderPipeline;
    fn deref(&self) -> &Self::Target {
        &self.pipeline
    }
}

impl Pipeline {
    pub fn output_format() -> wgpu::TextureFormat {
        wgpu::TextureFormat::Rgba8UnormSrgb
    }

    pub fn pipeline_layout(ctx: &WgpuContext) -> wgpu::PipelineLayout {
        ctx.device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Simple Pipeline Layout"),
                bind_group_layouts: &[&CameraUniformsBindGroup::bind_group_layout(ctx)],
                push_constant_ranges: &[],
            })
    }
}

impl RenderPipeline for Pipeline {
    type Vertex = Vertex;
    type Uniforms = CameraUniforms;

    fn new(ctx: &WgpuContext) -> Self {
        let WgpuContext { device, .. } = ctx;

        let module = &device.create_shader_module(include_wgsl!("../../shader/simple.wgsl"));
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Simple Pipeline"),
            layout: Some(&Self::pipeline_layout(ctx)),
            vertex: wgpu::VertexState {
                module,
                entry_point: Some("vs_main"),
                buffers: &[Vertex::desc()],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module,
                entry_point: Some("fs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: Self::output_format(),
                    blend: None,
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
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        Self { pipeline }
    }
}
