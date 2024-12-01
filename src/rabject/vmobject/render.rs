#[allow(unused)]
use log::trace;

use crate::{
    rabject::{RabjectWithId, RenderInstance},
    RanimContext, WgpuBuffer,
};

use super::{ComputeUniform, VMobject, VMobjectFillVertex, VMobjectPoint, VMobjectStrokeVertex};

impl VMobject {
    pub fn render_fill(&self) {}
}

pub struct VMObjectRenderInstance {
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

impl VMObjectRenderInstance {
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
        ctx: &RanimContext,
        points_buffer: &wgpu::Buffer,
        joint_angles_buffer: &wgpu::Buffer,
        stroke_vertices_buffer: &wgpu::Buffer,
        compute_uniform_buffer: &wgpu::Buffer,
    ) -> wgpu::BindGroup {
        ctx.wgpu_ctx
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("VMObject Compute Bind Group"),
                layout: &Self::compute_bind_group_layout(&ctx.wgpu_ctx.device),
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
        ctx: &RanimContext,
        stroke_vertices_buffer: &wgpu::Buffer,
    ) -> wgpu::BindGroup {
        ctx.wgpu_ctx
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("VMObject Render Bind Group"),
                layout: &Self::render_bind_group_layout(&ctx.wgpu_ctx.device),
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: stroke_vertices_buffer.as_entire_binding(),
                }],
            })
    }
}

impl RenderInstance<VMobject> for VMObjectRenderInstance {
    fn init(ctx: &mut RanimContext, rabject: &VMobject) -> Self {
        // trace!("INIT: {:?}", rabject.points().iter().map(|p| p.position()).collect::<Vec<_>>());
        let points = rabject.points();
        let joint_angles = rabject.get_joint_angles();
        let unit_normal = rabject.get_unit_normal();
        // trace!(
        //     "INIT points: {:?}",
        //     points.iter().map(|p| p.position()).collect::<Vec<_>>()
        // );
        // trace!("INIT joint_angles: {:?}", joint_angles);
        // trace!("INIT unit_normal: {:?}", unit_normal);

        let points_buffer = WgpuBuffer::new_init(
            &ctx.wgpu_ctx,
            &points,
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        );
        let joint_angles_buffer = WgpuBuffer::new_init(
            &ctx.wgpu_ctx,
            &joint_angles,
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        );
        let stroke_vertices_buffer = WgpuBuffer::new(
            &ctx.wgpu_ctx,
            (std::mem::size_of::<VMobjectStrokeVertex>() * 1024) as u64,
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        );
        let fill_vertex_buffer = WgpuBuffer::new(
            &ctx.wgpu_ctx,
            (std::mem::size_of::<VMobjectFillVertex>() * 1024) as u64,
            wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        );
        let compute_uniform_buffer = WgpuBuffer::new_init(
            &ctx.wgpu_ctx,
            &[ComputeUniform {
                unit_normal,
                _padding: 0.0,
            }],
            wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        );

        let compute_bind_group = Self::create_compute_bind_group(
            &ctx,
            &points_buffer,
            &joint_angles_buffer,
            &stroke_vertices_buffer,
            &compute_uniform_buffer,
        );

        let render_stroke_bind_group =
            VMObjectRenderInstance::create_render_bind_group(&ctx, &stroke_vertices_buffer);

        Self {
            points_buffer,
            joint_angles_buffer,
            fill_vertices_buffer: fill_vertex_buffer,
            stroke_vertices_buffer,
            compute_uniform_buffer,
            compute_bind_group,
            render_stroke_bind_group,
        }
    }

    fn update(&mut self, ctx: &mut crate::RanimContext, rabject: &RabjectWithId<VMobject>) {
        let points = rabject.points();
        let joint_angles = rabject.get_joint_angles();
        let unit_normal = rabject.get_unit_normal();
        let fill_vertices = rabject.parse_fill();
        // trace!("UPDATE joint_angles: {:?}", joint_angles);
        // trace!("UPDATE unit_normal: {:?}", unit_normal);
        // trace!(
        //     "UPDATE points: {:?}",
        //     rabject
        //         .points()
        //         .iter()
        //         .map(|p| p.position())
        //         .collect::<Vec<_>>()
        // );
        trace!(
            "UPDATE fill_vertices: {:?}",
            fill_vertices
                .iter()
                .map(|v| v.pos)
                .collect::<Vec<_>>()
        );
        self.points_buffer
            .prepare_from_slice(&ctx.wgpu_ctx, &points);
        self.joint_angles_buffer
            .prepare_from_slice(&ctx.wgpu_ctx, &joint_angles);
        self.compute_uniform_buffer.prepare_from_slice(
            &ctx.wgpu_ctx,
            &[ComputeUniform {
                unit_normal,
                _padding: 0.0,
            }],
        );
        self.fill_vertices_buffer
            .prepare_from_slice(&ctx.wgpu_ctx, &fill_vertices);
        ctx.wgpu_ctx.queue.submit([]);

        self.compute_bind_group = VMObjectRenderInstance::create_compute_bind_group(
            ctx,
            &self.points_buffer,
            &self.joint_angles_buffer,
            &self.stroke_vertices_buffer,
            &self.compute_uniform_buffer,
        );
        self.render_stroke_bind_group =
            VMObjectRenderInstance::create_render_bind_group(ctx, &self.stroke_vertices_buffer);
    }
}
