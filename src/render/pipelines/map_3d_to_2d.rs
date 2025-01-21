use std::ops::Deref;

use glam::{Vec3, Vec4};

use crate::{
    context::WgpuContext,
    rabject::RenderResource,
    render::{CameraUniforms, WgpuBuffer},
};

pub struct Map3dTo2dPipeline {
    pipeline: wgpu::ComputePipeline,
}

pub struct ComputeBindGroup {
    bind_group: wgpu::BindGroup,
}

impl Deref for ComputeBindGroup {
    type Target = wgpu::BindGroup;
    fn deref(&self) -> &Self::Target {
        &self.bind_group
    }
}

impl ComputeBindGroup {
    pub fn bind_group_layout(ctx: &WgpuContext) -> wgpu::BindGroupLayout {
        ctx.device
            .create_bind_group_layout(&Self::bind_group_layout_desc())
    }
    pub fn bind_group_layout_desc<'a>() -> wgpu::BindGroupLayoutDescriptor<'a> {
        wgpu::BindGroupLayoutDescriptor {
            label: Some("Map 3D to 2D Points Bind Group Layout"),
            entries: &[
                // (x, y, z, is_closed)
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
                // (x, y, is_closed, 0)
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
            ],
        }
    }

    fn new_bind_group(
        ctx: &WgpuContext,
        points3d_buffer: &WgpuBuffer<Vec4>,
        points2d_buffer: &WgpuBuffer<Vec4>,
    ) -> wgpu::BindGroup {
        ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Map 3D to 2D Compute Bind Group"),
            layout: &ComputeBindGroup::bind_group_layout(ctx),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer(
                        points3d_buffer.as_entire_buffer_binding(),
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Buffer(
                        points2d_buffer.as_entire_buffer_binding(),
                    ),
                },
            ],
        })
    }
    pub fn new(
        ctx: &WgpuContext,
        points3d_buffer: &WgpuBuffer<Vec4>,
        points2d_buffer: &WgpuBuffer<Vec4>,
    ) -> Self {
        Self {
            bind_group: Self::new_bind_group(ctx, points3d_buffer, points2d_buffer),
        }
    }

    pub fn update(
        &mut self,
        ctx: &WgpuContext,
        points3d_buffer: &WgpuBuffer<Vec4>,
        points2d_buffer: &WgpuBuffer<Vec4>,
    ) {
        self.bind_group = Self::new_bind_group(ctx, points3d_buffer, points2d_buffer);
    }
}

impl Deref for Map3dTo2dPipeline {
    type Target = wgpu::ComputePipeline;
    fn deref(&self) -> &Self::Target {
        &self.pipeline
    }
}

impl RenderResource for Map3dTo2dPipeline {
    fn new(wgpu_ctx: &WgpuContext) -> Self {
        let WgpuContext { device, .. } = wgpu_ctx;

        let module =
            &device.create_shader_module(wgpu::include_wgsl!("./shaders/map_3d_to_2d.wgsl"));

        let cam_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Map 3D to 2D Bind Group Layout"),
                entries: &[CameraUniforms::as_bind_group_layout_entry(0)],
            });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Map 3D to 2D Pipeline Layout"),
            bind_group_layouts: &[
                &cam_bind_group_layout,
                &ComputeBindGroup::bind_group_layout(wgpu_ctx),
            ],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Map 3D to 2D Pipeline"),
            layout: Some(&pipeline_layout),
            module,
            entry_point: Some("cs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });

        Self { pipeline }
    }
}
