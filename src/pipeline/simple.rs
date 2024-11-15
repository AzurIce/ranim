use bytemuck::{Pod, Zeroable};
use glam::{vec3, vec4, Vec3, Vec4};
use wgpu::include_wgsl;

use crate::{WgpuBuffer, WgpuContext};

use super::Pipeline;

#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
pub struct SimpleVertex {
    pub position: Vec3,
    _padding: f32,
    pub color: Vec4,
}

impl SimpleVertex {
    pub fn new(position: Vec3, color: Vec4) -> Self {
        Self { position, _padding: 0.0, color }
    }
}

impl SimpleVertex {
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

impl SimpleVertex {
    pub fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
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

pub struct SimplePipeline {
    pub pipeline: wgpu::RenderPipeline,
}

impl SimplePipeline {
    pub fn output_format() -> wgpu::TextureFormat {
        wgpu::TextureFormat::Rgba8UnormSrgb
    }

    pub fn layout_desc<'a>() -> wgpu::PipelineLayoutDescriptor<'a> {
        wgpu::PipelineLayoutDescriptor {
            label: Some("Simple Pipeline Layout"),
            bind_group_layouts: &[],
            push_constant_ranges: &[],
        }
    }
}
impl Pipeline for SimplePipeline {
    type Vertex = SimpleVertex;

    fn new(ctx: &WgpuContext) -> Self {
        let WgpuContext { device, .. } = ctx;

        let module = &device.create_shader_module(include_wgsl!("../../shader/simple.wgsl"));
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Simple Pipeline"),
            layout: Some(&device.create_pipeline_layout(&Self::layout_desc())),
            vertex: wgpu::VertexState {
                module,
                entry_point: Some("vs_main"),
                buffers: &[SimpleVertex::desc()],
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
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        Self { pipeline }
    }

    fn render(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        vertex_buffer: &WgpuBuffer<Self::Vertex>,
    ) {
        let render_pass_desc = wgpu::RenderPassDescriptor {
            label: Some("Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &target,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.1,
                        g: 0.2,
                        b: 0.3,
                        a: 1.0,
                    }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            occlusion_query_set: None,
            timestamp_writes: None,
        };
        let mut render_pass = encoder.begin_render_pass(&render_pass_desc);
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
        render_pass.draw(0..vertex_buffer.len() as u32, 0..1);
    }
}
