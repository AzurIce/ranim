use std::ops::Deref;

use crate::{CameraUniforms, RenderResource, WgpuContext};

pub struct Map3dTo2dPipeline {
    pipeline: wgpu::ComputePipeline,
}

pub struct ComputeBindGroup(wgpu::BindGroup);

impl AsRef<wgpu::BindGroup> for ComputeBindGroup {
    fn as_ref(&self) -> &wgpu::BindGroup {
        &self.0
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
                // width
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
                // (x, y, is_closed, 0)
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
                // min_x, min_y, max_x, max_y
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // point_cnt
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
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
        points3d_buffer: &wgpu::Buffer,
        stroke_width_buffer: &wgpu::Buffer,
        points2d_buffer: &wgpu::Buffer,
        clip_box_buffer: &wgpu::Buffer,
        point_cnt_buffer: &wgpu::Buffer,
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
                        stroke_width_buffer.as_entire_buffer_binding(),
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Buffer(
                        points2d_buffer.as_entire_buffer_binding(),
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Buffer(
                        clip_box_buffer.as_entire_buffer_binding(),
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::Buffer(
                        point_cnt_buffer.as_entire_buffer_binding(),
                    ),
                },
            ],
        })
    }
    pub(crate) fn new(
        ctx: &WgpuContext,
        points3d_buffer: &wgpu::Buffer,
        stroke_width_buffer: &wgpu::Buffer,
        points2d_buffer: &wgpu::Buffer,
        clip_box_buffer: &wgpu::Buffer,
        point_cnt_buffer: &wgpu::Buffer,
    ) -> Self {
        Self(Self::new_bind_group(
            ctx,
            points3d_buffer,
            stroke_width_buffer,
            points2d_buffer,
            clip_box_buffer,
            point_cnt_buffer,
        ))
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
