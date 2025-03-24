use std::ops::Deref;

use crate::{
    context::WgpuContext,
    render::{CameraUniforms, RenderResource},
};

pub struct NVItemMapPointsPipeline {
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
            label: Some("NVItem Map Points Bind Group Layout"),
            entries: &[
                // NVPoint in
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
                // NVPoint out
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
                // points_len
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
                // min_x, min_y, max_x, max_y
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
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
        input_nvpoints_buffer: &wgpu::Buffer,
        stroke_width_buffer: &wgpu::Buffer,
        output_nvpoints_buffer: &wgpu::Buffer,
        points_len_buffer: &wgpu::Buffer,
        clip_box_buffer: &wgpu::Buffer,
    ) -> wgpu::BindGroup {
        ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("NVItem Map Points Compute Bind Group"),
            layout: &ComputeBindGroup::bind_group_layout(ctx),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer(
                        input_nvpoints_buffer.as_entire_buffer_binding(),
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
                        output_nvpoints_buffer.as_entire_buffer_binding(),
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Buffer(
                        points_len_buffer.as_entire_buffer_binding(),
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::Buffer(
                        clip_box_buffer.as_entire_buffer_binding(),
                    ),
                },
            ],
        })
    }
    pub(crate) fn new(
        ctx: &WgpuContext,
        input_nvpoints_buffer: &wgpu::Buffer,
        stroke_width_buffer: &wgpu::Buffer,
        output_nvpoints_buffer: &wgpu::Buffer,
        points_len_buffer: &wgpu::Buffer,
        clip_box_buffer: &wgpu::Buffer,
    ) -> Self {
        Self(Self::new_bind_group(
            ctx,
            input_nvpoints_buffer,
            stroke_width_buffer,
            output_nvpoints_buffer,
            points_len_buffer,
            clip_box_buffer,
        ))
    }
}

impl Deref for NVItemMapPointsPipeline {
    type Target = wgpu::ComputePipeline;
    fn deref(&self) -> &Self::Target {
        &self.pipeline
    }
}

impl RenderResource for NVItemMapPointsPipeline {
    fn new(wgpu_ctx: &WgpuContext) -> Self {
        let WgpuContext { device, .. } = wgpu_ctx;

        let module =
            &device.create_shader_module(wgpu::include_wgsl!("./shaders/nvitem_map_points.wgsl"));

        let cam_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("NVItem Map Points Bind Group Layout"),
                entries: &[CameraUniforms::as_bind_group_layout_entry(0)],
            });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("NVItem Map Points Pipeline Layout"),
            bind_group_layouts: &[
                &cam_bind_group_layout,
                &ComputeBindGroup::bind_group_layout(wgpu_ctx),
            ],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("NVItem Map Points Pipeline"),
            layout: Some(&pipeline_layout),
            module,
            entry_point: Some("cs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });

        Self { pipeline }
    }
}
