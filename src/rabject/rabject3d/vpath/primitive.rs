use super::pipeline::{ComputePipeline, FillPipeline, StrokePipeline};
use crate::context::WgpuContext;
use crate::{rabject::Primitive, utils::wgpu::WgpuBuffer};
use bevy_color::LinearRgba;
use glam::{Vec3, Vec4};

#[repr(C, align(16))]
#[derive(Default, Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct VPathPoint {
    pub position: Vec4,
    /// If it is equal to the position, it means the end of curve
    pub prev_handle: Vec4,
    /// If it is equal to the position, it means the end of curve
    pub next_handle: Vec4,
    pub fill_color: LinearRgba,
    pub stroke_color: LinearRgba,
    pub stroke_width: f32,
    pub joint_angle: f32,
    pub(super) _padding: [f32; 2],
}

#[allow(unused_imports)]
use log::{info, trace};

use super::pipeline::{StencilPipeline, VPathFillVertex};

#[derive(Debug)]
pub struct ExtractedVPath {
    pub points: Vec<VPathPoint>,
    pub unit_normal: Vec3,
    pub fill_triangles: Vec<VPathFillVertex>,
    pub render_order: usvg::PaintOrder,
}

impl Default for ExtractedVPath {
    fn default() -> Self {
        Self {
            points: vec![VPathPoint::default(); 3],
            unit_normal: Vec3::ZERO,
            fill_triangles: vec![VPathFillVertex::default(); 3],
            render_order: Default::default(),
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
pub struct VPathStrokeVertex {
    pub pos: Vec4,
    pub stroke_color: Vec4,
}

pub struct VPathPrimitive {
    /// COMPUTE INPUT: the points of the VMObject
    pub(crate) points_buffer: WgpuBuffer<VPathPoint>,
    /// COMPUTE INPUT: the unit normal of the VMObject
    pub(crate) compute_uniform_buffer: WgpuBuffer<ComputeUniform>,

    /// RENDER-FILL INPUT: the vertices of the VMObject filled with color
    pub(crate) fill_vertices_buffer: WgpuBuffer<VPathFillVertex>,
    /// RENDER-STROKE INPUT: the vertices of the VMObject stroked
    pub(crate) stroke_vertices_buffer: WgpuBuffer<VPathStrokeVertex>,

    /// COMPUTE BIND GROUP: 0-points, 1-joint angles, 2-stroke vertices, 3-compute uniforms
    pub(crate) compute_bind_group: wgpu::BindGroup,
    /// RENDER-STROKE BIND GROUP: 0-stroke vertices
    pub(crate) render_stroke_bind_group: wgpu::BindGroup,

    pub paint_order: usvg::PaintOrder,
}

const MAX_STEP: u32 = 16;
impl Primitive for VPathPrimitive {
    type Data = ExtractedVPath;

    fn init(wgpu_ctx: &WgpuContext, data: &Self::Data) -> Self {
        // info!("[VMobjectPrimitive::init]: {:?}", data);
        let ExtractedVPath {
            points,
            unit_normal,
            fill_triangles,
            render_order,
        } = data;

        let points_buffer = WgpuBuffer::new_init(
            wgpu_ctx,
            points,
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        );
        let stroke_vertices_buffer = WgpuBuffer::new(
            wgpu_ctx,
            (std::mem::size_of::<VPathStrokeVertex>() * 1024) as u64,
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
            &stroke_vertices_buffer,
            &compute_uniform_buffer,
        );

        let render_stroke_bind_group =
            Self::create_render_bind_group(wgpu_ctx, &stroke_vertices_buffer);

        Self {
            points_buffer,
            fill_vertices_buffer,
            stroke_vertices_buffer,
            compute_uniform_buffer,
            compute_bind_group,
            render_stroke_bind_group,
            paint_order: *render_order,
        }
    }

    fn update(&mut self, wgpu_ctx: &WgpuContext, data: &Self::Data) {
        // info!("[VMobjectPrimitive::update]: {:?}", data);
        let ExtractedVPath {
            points,
            unit_normal,
            fill_triangles,
            render_order,
        } = data;

        self.points_buffer.prepare_from_slice(wgpu_ctx, points);
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
            &self.stroke_vertices_buffer,
            &self.compute_uniform_buffer,
        );
        self.render_stroke_bind_group =
            Self::create_render_bind_group(wgpu_ctx, &self.stroke_vertices_buffer);
        self.paint_order = *render_order;
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
                label: Some("VPath Render Encoder"),
            });

        // Compute pass for stroke
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("VPath Compute Pass"),
                timestamp_writes: None,
            });
            let pipeline = pipelines.get_or_init::<ComputePipeline>(wgpu_ctx);
            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, &self.compute_bind_group, &[]);
            // number of segments
            let len = self.points_buffer.len() - 1;
            trace!("dispatch workgroups: {}", len);
            pass.dispatch_workgroups(len as u32, 1, 1);
        }

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("VPath Stencil Pass"),
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
                label: Some("VPath Render Pass"),
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
            pass.set_bind_group(0, uniforms_bind_group, &[]);

            match self.paint_order {
                usvg::PaintOrder::FillAndStroke => {
                    let pipeline_fill = pipelines.get_or_init::<FillPipeline>(wgpu_ctx);
                    pass.set_pipeline(pipeline_fill);
                    pass.set_vertex_buffer(0, self.fill_vertices_buffer.slice(..));
                    pass.draw(0..self.fill_vertices_buffer.len() as u32, 0..1);

                    let pipeline_vmobject_stroke =
                        pipelines.get_or_init::<StrokePipeline>(wgpu_ctx);
                    pass.set_pipeline(pipeline_vmobject_stroke);
                    pass.set_bind_group(1, &self.render_stroke_bind_group, &[]);
                    let len = (self.points_buffer.len() - 1) as u32 * MAX_STEP * 2;
                    trace!("draw {}", len);
                    pass.draw(0..len, 0..1);
                }
                usvg::PaintOrder::StrokeAndFill => {
                    let pipeline_vmobject_stroke =
                        pipelines.get_or_init::<StrokePipeline>(wgpu_ctx);
                    pass.set_pipeline(pipeline_vmobject_stroke);
                    pass.set_bind_group(1, &self.render_stroke_bind_group, &[]);
                    let len = (self.points_buffer.len() - 1) as u32 * MAX_STEP * 2;
                    trace!("draw {}", len);
                    pass.draw(0..len, 0..1);

                    let pipeline_fill = pipelines.get_or_init::<FillPipeline>(wgpu_ctx);
                    pass.set_pipeline(pipeline_fill);
                    pass.set_vertex_buffer(0, self.fill_vertices_buffer.slice(..));
                    pass.draw(0..self.fill_vertices_buffer.len() as u32, 0..1);
                }
            }
        }
        wgpu_ctx.queue.submit(Some(encoder.finish()));
    }
}

impl VPathPrimitive {
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
                // Vertices
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
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
                    binding: 2,
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
                        resource: stroke_vertices_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
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
