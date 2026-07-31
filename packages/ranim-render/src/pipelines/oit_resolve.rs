use std::ops::Deref;

use bevy_ecs::prelude::*;

use crate::{
    ResolutionInfo, WgpuContext,
    resource::{GpuResource, OUTPUT_TEXTURE_FORMAT, PipelinesPool},
    schedule::{FrameTarget, RenderContext},
};

pub(crate) fn resolve(
    mut render: RenderContext,
    ctx: Res<WgpuContext>,
    pipelines: Res<PipelinesPool>,
    resolution: Res<ResolutionInfo>,
    target: Res<FrameTarget>,
) {
    let mut pass = render
        .encoder()
        .begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("OIT Resolve Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &target.render_view,
                resolve_target: None,
                depth_slice: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
    pass.set_pipeline(&pipelines.get_or_init::<OITResolvePipeline>(&ctx));
    pass.set_bind_group(0, &resolution.bind_group, &[]);
    pass.set_bind_group(1, &target.depth_bind_group, &[]);
    pass.draw(0..3, 0..1);
    drop(pass);
    render
        .encoder()
        .clear_buffer(&resolution.pixel_count_buffer.buffer, 0, None);
}

pub struct OITResolvePipeline {
    pipeline: wgpu::RenderPipeline,
}

impl Deref for OITResolvePipeline {
    type Target = wgpu::RenderPipeline;
    fn deref(&self) -> &Self::Target {
        &self.pipeline
    }
}

impl OITResolvePipeline {
    pub fn depth_bind_group_layout(ctx: &WgpuContext) -> wgpu::BindGroupLayout {
        ctx.device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("OIT Resolve Depth BGL"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Depth,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                }],
            })
    }
}

impl GpuResource for OITResolvePipeline {
    fn new(wgpu_ctx: &WgpuContext) -> Self {
        let WgpuContext { device, .. } = wgpu_ctx;

        let module =
            &device.create_shader_module(wgpu::include_wgsl!("./shaders/oit_resolve.wgsl"));

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("OIT Resolve Pipeline Layout"),
            bind_group_layouts: &[
                Some(&ResolutionInfo::create_bind_group_layout(wgpu_ctx)),
                Some(&Self::depth_bind_group_layout(wgpu_ctx)),
            ],
            immediate_size: 0,
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("OIT Resolve Pipeline"),
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
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: None, // No depth attachment
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview_mask: None,
            cache: None,
        });

        Self { pipeline }
    }
}
