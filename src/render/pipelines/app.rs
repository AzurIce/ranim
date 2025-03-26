use std::ops::Deref;

use crate::context::WgpuContext;

pub struct AppBindGroup(wgpu::BindGroup);

impl AsRef<wgpu::BindGroup> for AppBindGroup {
    fn as_ref(&self) -> &wgpu::BindGroup {
        &self.0
    }
}

impl AppBindGroup {
    pub fn bind_group_layout(ctx: &WgpuContext) -> wgpu::BindGroupLayout {
        ctx.device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("App Bind Group Layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::all(),
                        count: None,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::all(),
                        count: None,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    },
                ],
            })
    }
    pub fn new_bind_group(
        ctx: &WgpuContext,
        texture_view: &wgpu::TextureView,
        sampler: &wgpu::Sampler,
    ) -> Self {
        let bind_group = ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("App Bind Group"),
            layout: &AppBindGroup::bind_group_layout(ctx),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(sampler),
                },
            ],
        });
        Self(bind_group)
    }
}

pub struct AppPipeline {
    pipeline: wgpu::RenderPipeline,
    pub(crate) bind_group: AppBindGroup,
}

impl Deref for AppPipeline {
    type Target = wgpu::RenderPipeline;
    fn deref(&self) -> &Self::Target {
        &self.pipeline
    }
}

impl AppPipeline {
    pub(crate) fn new(
        ctx: &WgpuContext,
        texture_view: &wgpu::TextureView,
        sampler: &wgpu::Sampler,
        target: wgpu::ColorTargetState,
    ) -> Self {
        let WgpuContext { device, .. } = ctx;

        let module = &device.create_shader_module(wgpu::include_wgsl!("./shaders/app.wgsl"));

        let pipeline_layout = ctx
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("App Pipeline Layout"),
                bind_group_layouts: &[&AppBindGroup::bind_group_layout(&ctx)],
                push_constant_ranges: &[],
            });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("App Pipeline"),
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
                targets: &[Some(target)],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleStrip,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        });

        let bind_group = AppBindGroup::new_bind_group(ctx, texture_view, sampler);

        Self {
            pipeline,
            bind_group,
        }
    }
}
