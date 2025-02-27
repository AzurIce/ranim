use std::ops::Deref;

use crate::{
    context::WgpuContext,
    render::{CameraUniformsBindGroup, RenderResource, OUTPUT_TEXTURE_FORMAT},
};

pub struct RenderBindGroup(wgpu::BindGroup);

impl AsRef<wgpu::BindGroup> for RenderBindGroup {
    fn as_ref(&self) -> &wgpu::BindGroup {
        &self.0
    }
}

impl RenderBindGroup {
    pub fn bind_group_layout(ctx: &WgpuContext) -> wgpu::BindGroupLayout {
        ctx.device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("VItem Render Bind Group Layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 4,
                        visibility: wgpu::ShaderStages::VERTEX,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            })
    }

    fn new_bind_group(
        ctx: &WgpuContext,
        points: &wgpu::Buffer,
        fill_rgbas: &wgpu::Buffer,
        stroke_rgbas: &wgpu::Buffer,
        stroke_widths: &wgpu::Buffer,
        cc: &wgpu::Buffer,
    ) -> wgpu::BindGroup {
        ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("VItem Render Bind Group"),
            layout: &RenderBindGroup::bind_group_layout(ctx),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer(points.as_entire_buffer_binding()),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Buffer(fill_rgbas.as_entire_buffer_binding()),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Buffer(
                        stroke_rgbas.as_entire_buffer_binding(),
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Buffer(
                        stroke_widths.as_entire_buffer_binding(),
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::Buffer(cc.as_entire_buffer_binding()),
                },
            ],
        })
    }
    pub(crate) fn new(
        ctx: &WgpuContext,
        points: &wgpu::Buffer,
        fill_rgbas: &wgpu::Buffer,
        stroke_rgbas: &wgpu::Buffer,
        stroke_widths: &wgpu::Buffer,
        cc: &wgpu::Buffer,
    ) -> Self {
        Self(Self::new_bind_group(
            ctx,
            points,
            fill_rgbas,
            stroke_rgbas,
            stroke_widths,
            cc,
        ))
    }
}

pub struct VItemPipeline {
    pipeline: wgpu::RenderPipeline,
}

impl Deref for VItemPipeline {
    type Target = wgpu::RenderPipeline;
    fn deref(&self) -> &Self::Target {
        &self.pipeline
    }
}

impl RenderResource for VItemPipeline {
    fn new(wgpu_ctx: &WgpuContext) -> Self {
        let WgpuContext { device, .. } = wgpu_ctx;

        let module = &device.create_shader_module(wgpu::include_wgsl!("./shaders/vitem.wgsl"));

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("VItem Pipeline Layout"),
            bind_group_layouts: &[
                &CameraUniformsBindGroup::bind_group_layout(wgpu_ctx),
                &RenderBindGroup::bind_group_layout(wgpu_ctx),
            ],
            push_constant_ranges: &[],
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("VItem Pipeline"),
            layout: Some(&pipeline_layout),
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
                // topology: wgpu::PrimitiveTopology::PointList,
                ..Default::default()
            },
            depth_stencil: None,
            // depth_stencil: Some(wgpu::DepthStencilState {
            //     format: wgpu::TextureFormat::Depth32Float,
            //     depth_write_enabled: true,
            //     depth_compare: wgpu::CompareFunction::LessEqual,
            //     stencil: wgpu::StencilState::default(),
            //     bias: wgpu::DepthBiasState::default(),
            // }),
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
