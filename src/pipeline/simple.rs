use std::ops::Deref;

use wgpu::include_wgsl;

use crate::{
    camera::CameraUniformsBindGroup, renderer::vmobject::VMobjectVertex, RanimContext, WgpuContext,
};

use super::RenderPipeline;

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
    fn output_format() -> wgpu::TextureFormat {
        wgpu::TextureFormat::Rgba8UnormSrgb
    }

    fn pipeline_layout(ctx: &WgpuContext) -> wgpu::PipelineLayout {
        ctx.device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Simple Pipeline Layout"),
                bind_group_layouts: &[&CameraUniformsBindGroup::bind_group_layout(ctx)],
                push_constant_ranges: &[],
            })
    }
}

impl RenderPipeline for Pipeline {
    type Vertex = VMobjectVertex;

    fn new(ctx: &RanimContext) -> Self {
        let WgpuContext { device, .. } = &ctx.wgpu_ctx;

        let module = &device.create_shader_module(include_wgsl!("../../shader/simple.wgsl"));
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Simple Pipeline"),
            layout: Some(&Self::pipeline_layout(&ctx.wgpu_ctx)),
            vertex: wgpu::VertexState {
                module,
                entry_point: Some("vs_main"),
                buffers: &[Self::Vertex::desc()],
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
