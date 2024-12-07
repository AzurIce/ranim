use crate::WgpuContext;
use crate::{rabject::Primitive, WgpuBuffer};
use glam::{Vec3, Vec4};

use super::pipeline::{ComputePipeline, FillPipeline, StencilPipeline, StrokePipeline};
use super::{VMobjectFillVertex, VMobjectPoint};

#[allow(unused_imports)]
use log::{info, trace};

#[derive(Debug)]
pub struct ExtractedVMobject {
    pub points: Vec<VMobjectPoint>,
    pub joint_angles: Vec<f32>,
    pub unit_normal: Vec3,
    pub fill_triangles: Vec<VMobjectFillVertex>,
}

impl Default for ExtractedVMobject {
    fn default() -> Self {
        Self {
            points: vec![VMobjectPoint::default(); 3],
            joint_angles: vec![0.0; 2],
            unit_normal: Vec3::ZERO,
            fill_triangles: vec![VMobjectFillVertex::default(); 3],
        }
    }
}

#[repr(C, align(16))]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct ComputeUniform {
    unit_normal: Vec3,
    _padding: f32,
}

#[repr(C, align(16))]
#[derive(Default, Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct VMobjectStrokeVertex {
    pub pos: Vec4,
    pub stroke_color: Vec4,
}

pub struct VMobjectPrimitive {
    /// COMPUTE INPUT: the points of the VMObject
    pub(crate) points_buffer: WgpuBuffer<VMobjectPoint>,
    /// COMPUTE INPUT: the joint angles of the VMObject
    pub(crate) joint_angles_buffer: WgpuBuffer<f32>,
    /// COMPUTE INPUT: the unit normal of the VMObject
    pub(crate) compute_uniform_buffer: WgpuBuffer<ComputeUniform>,

    /// RENDER-FILL INPUT: the vertices of the VMObject filled with color
    pub(crate) fill_vertices_buffer: WgpuBuffer<VMobjectFillVertex>,
    /// RENDER-STROKE INPUT: the vertices of the VMObject stroked
    pub(crate) stroke_vertices_buffer: WgpuBuffer<VMobjectStrokeVertex>,

    /// COMPUTE BIND GROUP: 0-points, 1-joint angles, 2-stroke vertices, 3-compute uniforms
    pub(crate) compute_bind_group: wgpu::BindGroup,
    /// RENDER-STROKE BIND GROUP: 0-stroke vertices
    pub(crate) render_stroke_bind_group: wgpu::BindGroup,
}

const MAX_STEP: u32 = 16;
impl Primitive for VMobjectPrimitive {
    type Data = ExtractedVMobject;

    fn init(wgpu_ctx: &WgpuContext, data: &Self::Data) -> Self {
        // info!("[VMobjectPrimitive::init]: {:?}", data);
        let ExtractedVMobject {
            points,
            joint_angles,
            unit_normal,
            fill_triangles,
        } = data;

        let points_buffer = WgpuBuffer::new_init(
            wgpu_ctx,
            points,
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        );
        let joint_angles_buffer = WgpuBuffer::new_init(
            wgpu_ctx,
            joint_angles,
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        );
        let stroke_vertices_buffer = WgpuBuffer::new(
            wgpu_ctx,
            (std::mem::size_of::<VMobjectStrokeVertex>() * 1024) as u64,
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        );
        let fill_vertices_buffer = WgpuBuffer::new_init(
            wgpu_ctx,
            fill_triangles,
            wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        );
        let compute_uniform_buffer = WgpuBuffer::new_init(
            wgpu_ctx,
            &[ComputeUniform {
                unit_normal: *unit_normal,
                _padding: 0.0,
            }],
            wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        );

        let compute_bind_group = Self::create_compute_bind_group(
            wgpu_ctx,
            &points_buffer,
            &joint_angles_buffer,
            &stroke_vertices_buffer,
            &compute_uniform_buffer,
        );

        let render_stroke_bind_group =
            Self::create_render_bind_group(wgpu_ctx, &stroke_vertices_buffer);

        Self {
            points_buffer,
            joint_angles_buffer,
            fill_vertices_buffer,
            stroke_vertices_buffer,
            compute_uniform_buffer,
            compute_bind_group,
            render_stroke_bind_group,
        }
    }

    fn update(&mut self, wgpu_ctx: &WgpuContext, data: &Self::Data) {
        // info!("[VMobjectPrimitive::update]: {:?}", data);
        let ExtractedVMobject {
            points,
            joint_angles,
            unit_normal,
            fill_triangles,
        } = data;

        self.points_buffer.prepare_from_slice(wgpu_ctx, points);
        self.joint_angles_buffer
            .prepare_from_slice(wgpu_ctx, joint_angles);
        self.compute_uniform_buffer.prepare_from_slice(
            wgpu_ctx,
            &[ComputeUniform {
                unit_normal: *unit_normal,
                _padding: 0.0,
            }],
        );
        self.fill_vertices_buffer
            .prepare_from_slice(wgpu_ctx, &fill_triangles);
        wgpu_ctx.queue.submit(None);

        self.compute_bind_group = Self::create_compute_bind_group(
            wgpu_ctx,
            &self.points_buffer,
            &self.joint_angles_buffer,
            &self.stroke_vertices_buffer,
            &self.compute_uniform_buffer,
        );
        self.render_stroke_bind_group =
            Self::create_render_bind_group(wgpu_ctx, &self.stroke_vertices_buffer);
    }

    fn render(
        &self,
        wgpu_ctx: &WgpuContext,
        pipelines: &mut crate::utils::RenderResourceStorage,
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
            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, &self.compute_bind_group, &[]);
            // number of segments
            let len = self.points_buffer.len() / 2;
            trace!("dispatch workgroups: {}", len);
            pass.dispatch_workgroups(len as u32, 1, 1);
        }

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("VMobject Stencil Pass"),
                color_attachments: &[],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &depth_view,
                    depth_ops: None,
                    stencil_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(0),
                        store: wgpu::StoreOp::Store,
                    }),
                }),
                occlusion_query_set: None,
                timestamp_writes: None,
            });
            pass.set_bind_group(0, uniforms_bind_group, &[]);

            let pipeline = pipelines.get_or_init::<StencilPipeline>(wgpu_ctx);
            pass.set_pipeline(pipeline);
            pass.set_vertex_buffer(0, self.fill_vertices_buffer.slice(..));
            pass.draw(0..self.fill_vertices_buffer.len() as u32, 0..1);
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
                        store: wgpu::StoreOp::Store,
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
            pass.set_pipeline(pipeline_vmobject_fill);
            pass.set_vertex_buffer(0, self.fill_vertices_buffer.slice(..));
            pass.draw(0..self.fill_vertices_buffer.len() as u32, 0..1);

            let pipeline_vmobject_stroke = pipelines.get_or_init::<StrokePipeline>(wgpu_ctx);
            pass.set_pipeline(pipeline_vmobject_stroke);
            pass.set_bind_group(1, &self.render_stroke_bind_group, &[]);
            let len = self.points_buffer.len() as u32 / 2 * MAX_STEP * 2;
            trace!("draw {}", len);
            pass.draw(0..len, 0..1);
        }
        wgpu_ctx.queue.submit(Some(encoder.finish()));
    }
}

impl VMobjectPrimitive {
    pub fn compute_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("VMObject Compute Bind Group Layout"),
            entries: &[
                // Points
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Joint Angles
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Vertices
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Uniforms
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        })
    }
    pub fn render_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("VMObject Bind Group Layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        })
    }

    pub fn create_compute_bind_group(
        wgpu_ctx: &WgpuContext,
        points_buffer: &wgpu::Buffer,
        joint_angles_buffer: &wgpu::Buffer,
        stroke_vertices_buffer: &wgpu::Buffer,
        compute_uniform_buffer: &wgpu::Buffer,
    ) -> wgpu::BindGroup {
        wgpu_ctx
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("VMObject Compute Bind Group"),
                layout: &Self::compute_bind_group_layout(&wgpu_ctx.device),
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: points_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: joint_angles_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: stroke_vertices_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: compute_uniform_buffer.as_entire_binding(),
                    },
                ],
            })
    }

    pub fn create_render_bind_group(
        wgpu_ctx: &WgpuContext,
        stroke_vertices_buffer: &wgpu::Buffer,
    ) -> wgpu::BindGroup {
        wgpu_ctx
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("VMObject Render Bind Group"),
                layout: &Self::render_bind_group_layout(&wgpu_ctx.device),
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: stroke_vertices_buffer.as_entire_binding(),
                }],
            })
    }
}
