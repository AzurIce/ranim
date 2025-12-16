use std::ops::Deref;

use crate::{RenderResource, WgpuContext};

pub struct ClipBox2dPipeline {
    pipeline: wgpu::ComputePipeline,
}

pub struct Clipbox2dComputeBindGroup(wgpu::BindGroup);

impl AsRef<wgpu::BindGroup> for Clipbox2dComputeBindGroup {
    fn as_ref(&self) -> &wgpu::BindGroup {
        &self.0
    }
}

impl Clipbox2dComputeBindGroup {
    pub fn bind_group_layout(ctx: &WgpuContext) -> wgpu::BindGroupLayout {
        ctx.device
            .create_bind_group_layout(&Self::bind_group_layout_desc())
    }
    pub fn bind_group_layout_desc<'a>() -> wgpu::BindGroupLayoutDescriptor<'a> {
        wgpu::BindGroupLayoutDescriptor {
            label: Some("ClipBox 2d Points Bind Group Layout"),
            entries: &[
                // points2d: (x, y, is_closed)
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
                // stroke_widths
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
                // clip_info: (min_x, max_x, min_y, max_y, max_w)
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
                // point_cnt
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
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
        points2d_buffer: &wgpu::Buffer,
        stroke_width_buffer: &wgpu::Buffer,
        clip_box_buffer: &wgpu::Buffer,
        point_cnt_buffer: &wgpu::Buffer,
    ) -> wgpu::BindGroup {
        ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("ClipBox 2d Compute Bind Group"),
            layout: &Clipbox2dComputeBindGroup::bind_group_layout(ctx),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer(
                        points2d_buffer.as_entire_buffer_binding(),
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
                        clip_box_buffer.as_entire_buffer_binding(),
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Buffer(
                        point_cnt_buffer.as_entire_buffer_binding(),
                    ),
                },
            ],
        })
    }
    pub(crate) fn new(
        ctx: &WgpuContext,
        points2d_buffer: &wgpu::Buffer,
        stroke_width_buffer: &wgpu::Buffer,
        clip_box_buffer: &wgpu::Buffer,
        point_cnt_buffer: &wgpu::Buffer,
    ) -> Self {
        Self(Self::new_bind_group(
            ctx,
            points2d_buffer,
            stroke_width_buffer,
            clip_box_buffer,
            point_cnt_buffer,
        ))
    }
}

impl Deref for ClipBox2dPipeline {
    type Target = wgpu::ComputePipeline;
    fn deref(&self) -> &Self::Target {
        &self.pipeline
    }
}

impl RenderResource for ClipBox2dPipeline {
    fn new(wgpu_ctx: &WgpuContext) -> Self {
        let WgpuContext { device, .. } = wgpu_ctx;

        let module = &device.create_shader_module(wgpu::include_wgsl!("./shaders/clipbox_2d.wgsl"));

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("ClipBox 2d Pipeline Layout"),
            bind_group_layouts: &[&Clipbox2dComputeBindGroup::bind_group_layout(wgpu_ctx)],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("ClipBox 2d Pipeline"),
            layout: Some(&pipeline_layout),
            module,
            entry_point: Some("cs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });

        Self { pipeline }
    }
}
