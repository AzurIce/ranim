use std::ops::Deref;

use log::trace;
use wgpu::include_wgsl;

use crate::{
    camera::CameraUniformsBindGroup,
    rabject::{
        vmobject::{render::VMObjectRenderInstance, VMobject, VMobjectFillVertex},
        Rabject, RabjectWithId,
    },
    RanimContext, RenderResourceStorage, WgpuContext,
};

use super::{RenderResource, Renderer, Vertex};

pub struct VMobjectRenderer {
    alpha_texture: wgpu::Texture,
}

impl RenderResource for VMobjectRenderer {
    fn new(wgpu_ctx: &WgpuContext) -> Self {
        let WgpuContext { device, .. } = wgpu_ctx;

        let alpha_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("VMobject Alpha Texture"),
            size: wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });

        Self { alpha_texture }
    }
}

const MAX_STEP: u32 = 16;
impl Renderer<VMobject> for VMobjectRenderer {
    fn render(
        &self,
        wgpu_ctx: &WgpuContext,
        pipelines: &mut RenderResourceStorage,
        render_instances: &[&VMObjectRenderInstance],
        multisample_view: &wgpu::TextureView,
        target_view: &wgpu::TextureView,
        depth_view: &wgpu::TextureView,
        uniforms_bind_group: &wgpu::BindGroup,
    ) {
        let mut encoder = wgpu_ctx
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("VMObject Render Encoder"),
            });

        // Compute pass for stroke
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("VMObject Compute Pass"),
                timestamp_writes: None,
            });
            let pipeline = pipelines.get_or_init::<ComputePipeline>(wgpu_ctx);
            pass.set_pipeline(&pipeline);
            for render_instance in render_instances {
                pass.set_bind_group(0, &render_instance.compute_bind_group, &[]);
                // number of segments
                trace!(
                    "dispatch workgroups: {}",
                    render_instance.points_buffer.len() / 2
                );
                pass.dispatch_workgroups(render_instance.points_buffer.len() as u32 / 2, 1, 1);
            }
        }

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("VMobject Stencil Pass"),
                color_attachments: &[],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &depth_view,
                    depth_ops: None,
                    stencil_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    }),
                }),
                occlusion_query_set: None,
                timestamp_writes: None,
            });
            pass.set_bind_group(0, uniforms_bind_group, &[]);

            let pipeline = pipelines.get_or_init::<StencilPipeline>(wgpu_ctx);
            pass.set_pipeline(&pipeline);
            for render_instance in render_instances {
                pass.set_vertex_buffer(0, render_instance.fill_vertices_buffer.slice(..));
                pass.draw(0..render_instance.fill_vertices_buffer.len() as u32, 0..1);
            }
        }

        // Render pass for fill and stroke
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("VMobject Fill Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &multisample_view,
                    resolve_target: Some(&target_view),
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Discard,
                    }),
                    stencil_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Discard,
                    }),
                }),
                occlusion_query_set: None,
                timestamp_writes: None,
            });
            // let mut pass =
            //     Self::begin_render_pass(&mut encoder, &multisample_view, &target_view, &depth_view);
            pass.set_bind_group(0, uniforms_bind_group, &[]);

            let pipeline_vmobject_fill = pipelines.get_or_init::<FillPipeline>(wgpu_ctx);
            pass.set_pipeline(&pipeline_vmobject_fill);
            for render_instance in render_instances {
                pass.set_vertex_buffer(0, render_instance.fill_vertices_buffer.slice(..));
                pass.draw(0..render_instance.fill_vertices_buffer.len() as u32, 0..1);
            }

            let pipeline_vmobject_stroke = pipelines.get_or_init::<StrokePipeline>(wgpu_ctx);
            pass.set_pipeline(&pipeline_vmobject_stroke);
            for render_instance in render_instances {
                pass.set_bind_group(1, &render_instance.render_stroke_bind_group, &[]);
                let len = render_instance.points_buffer.len() as u32 / 2 * MAX_STEP * 2;
                trace!("draw {}", len);
                pass.draw(0..len, 0..1);
            }
        }
        wgpu_ctx.queue.submit(Some(encoder.finish()));
    }
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
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32,
                },
                wgpu::VertexAttribute {
                    offset: size_of::<[f32; 4]>() as wgpu::BufferAddress,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        }
    }
}

pub struct DepthPipeline {
    pub pipeline: wgpu::RenderPipeline,
}

impl Deref for DepthPipeline {
    type Target = wgpu::RenderPipeline;
    fn deref(&self) -> &Self::Target {
        &self.pipeline
    }
}

impl DepthPipeline {
    fn pipeline_layout(ctx: &WgpuContext) -> wgpu::PipelineLayout {
        ctx.device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("VMobject Fill Pipeline Layout"),
                bind_group_layouts: &[&CameraUniformsBindGroup::bind_group_layout(ctx)],
                push_constant_ranges: &[],
            })
    }
}

impl RenderResource for DepthPipeline {
    fn new(wgpu_ctx: &WgpuContext) -> Self {
        let WgpuContext { device, .. } = wgpu_ctx;

        let module =
            &device.create_shader_module(include_wgsl!("../../shader/vmobject_depth.wgsl"));
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("VMobject Depth Pipeline"),
            layout: Some(&Self::pipeline_layout(&wgpu_ctx)),
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
                targets: &[],
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

pub struct StencilPipeline {
    pub pipeline: wgpu::RenderPipeline,
}

impl Deref for StencilPipeline {
    type Target = wgpu::RenderPipeline;
    fn deref(&self) -> &Self::Target {
        &self.pipeline
    }
}

impl StencilPipeline {
    fn pipeline_layout(ctx: &WgpuContext) -> wgpu::PipelineLayout {
        ctx.device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("VMobject Stencil Pipeline Layout"),
                bind_group_layouts: &[&CameraUniformsBindGroup::bind_group_layout(ctx)],
                push_constant_ranges: &[],
            })
    }
}

impl RenderResource for StencilPipeline {
    fn new(wgpu_ctx: &WgpuContext) -> Self {
        let WgpuContext { device, .. } = wgpu_ctx;

        let module = &device.create_shader_module(include_wgsl!("../../shader/vmobject_fill.wgsl"));
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("VMobject Stencil Pipeline"),
            layout: Some(&Self::pipeline_layout(&wgpu_ctx)),
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
                targets: &[],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth24PlusStencil8,
                depth_write_enabled: false,
                depth_compare: wgpu::CompareFunction::Always,
                stencil: wgpu::StencilState {
                    front: wgpu::StencilFaceState {
                        compare: wgpu::CompareFunction::Always,
                        fail_op: wgpu::StencilOperation::IncrementWrap,
                        depth_fail_op: wgpu::StencilOperation::IncrementWrap,
                        pass_op: wgpu::StencilOperation::IncrementWrap,
                    },
                    back: wgpu::StencilFaceState {
                        compare: wgpu::CompareFunction::Always,
                        fail_op: wgpu::StencilOperation::DecrementWrap,
                        depth_fail_op: wgpu::StencilOperation::DecrementWrap,
                        pass_op: wgpu::StencilOperation::DecrementWrap,
                    },
                    read_mask: 0xff,
                    write_mask: 0xff,
                },
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
        wgpu::TextureFormat::Rgba16Float
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

impl RenderResource for FillPipeline {
    fn new(wgpu_ctx: &WgpuContext) -> Self {
        let WgpuContext { device, .. } = wgpu_ctx;

        let module = &device.create_shader_module(include_wgsl!("../../shader/vmobject_fill.wgsl"));
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("VMobject Fill Pipeline"),
            layout: Some(&Self::pipeline_layout(&wgpu_ctx)),
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
                    blend: Some(wgpu::BlendState {
                        color: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::SrcAlpha,
                            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                            operation: wgpu::BlendOperation::Add,
                        },
                        alpha: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::OneMinusDstAlpha,
                            dst_factor: wgpu::BlendFactor::One,
                            operation: wgpu::BlendOperation::Add,
                        },
                    }),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth24PlusStencil8,
                depth_write_enabled: false,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState {
                    front: wgpu::StencilFaceState {
                        compare: wgpu::CompareFunction::Less,
                        fail_op: wgpu::StencilOperation::Keep,
                        depth_fail_op: wgpu::StencilOperation::Keep,
                        pass_op: wgpu::StencilOperation::Keep,
                    },
                    back: wgpu::StencilFaceState {
                        compare: wgpu::CompareFunction::Less,
                        fail_op: wgpu::StencilOperation::Keep,
                        depth_fail_op: wgpu::StencilOperation::Keep,
                        pass_op: wgpu::StencilOperation::Keep,
                    },
                    read_mask: 0xff,
                    write_mask: 0xff,
                },
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
        wgpu::TextureFormat::Rgba16Float
    }

    fn pipeline_layout(ctx: &WgpuContext) -> wgpu::PipelineLayout {
        ctx.device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("VMobject Fill Pipeline Layout"),
                bind_group_layouts: &[
                    &CameraUniformsBindGroup::bind_group_layout(ctx),
                    &VMObjectRenderInstance::render_bind_group_layout(&ctx.device),
                ],
                push_constant_ranges: &[],
            })
    }
}

impl RenderResource for StrokePipeline {
    fn new(wgpu_ctx: &WgpuContext) -> Self {
        let WgpuContext { device, .. } = wgpu_ctx;

        let module =
            &device.create_shader_module(include_wgsl!("../../shader/vmobject_stroke.wgsl"));
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("VMobject Stroke Pipeline"),
            layout: Some(&Self::pipeline_layout(&wgpu_ctx)),
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

pub struct ComputePipeline {
    pub pipeline: wgpu::ComputePipeline,
}

impl Deref for ComputePipeline {
    type Target = wgpu::ComputePipeline;
    fn deref(&self) -> &Self::Target {
        &self.pipeline
    }
}

impl RenderResource for ComputePipeline {
    fn new(wgpu_ctx: &WgpuContext) -> Self {
        let WgpuContext { device, .. } = wgpu_ctx;

        let module =
            &device.create_shader_module(include_wgsl!("../../shader/vmobject_compute.wgsl"));

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("VMobject Compute Pipeline Layout"),
            bind_group_layouts: &[&VMObjectRenderInstance::compute_bind_group_layout(&device)],
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
