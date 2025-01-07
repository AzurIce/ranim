use std::{num::NonZeroUsize, ops::Deref};

use bevy_color::Color;
use glam::{vec3, Mat4, Vec2, Vec3};

use crate::{
    context::{RanimContext, WgpuContext},
    scene::store::EntitiesStore,
    utils::wgpu::WgpuBuffer,
};

// use super::pipeline::BlendPipeline;

#[allow(unused)]
use log::{debug, trace};

use super::pipeline::BlendPipeline;

#[repr(C, align(16))]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniforms {
    view_mat: Mat4,
}

impl CameraUniforms {
    pub fn as_bind_group_layout_entry() -> wgpu::BindGroupLayoutEntry {
        wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::all(),
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }
    }
}

pub struct CameraUniformsBindGroup {
    pub bind_group: wgpu::BindGroup,
}

impl CameraUniformsBindGroup {
    pub(crate) fn bind_group_layout(ctx: &WgpuContext) -> wgpu::BindGroupLayout {
        ctx.device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Simple Pipeline Uniforms"),
                entries: &[CameraUniforms::as_bind_group_layout_entry()],
            })
    }

    pub(crate) fn new(ctx: &WgpuContext, uniforms_buffer: &WgpuBuffer<CameraUniforms>) -> Self {
        let bind_group = ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("CanvasCamera Uniforms"),
            layout: &Self::bind_group_layout(ctx),
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(uniforms_buffer.as_entire_buffer_binding()),
            }],
        });
        Self { bind_group }
    }
}

impl Deref for CameraUniformsBindGroup {
    type Target = wgpu::BindGroup;
    fn deref(&self) -> &Self::Target {
        &self.bind_group
    }
}

pub struct TextureBindGroup {
    pub sampler: wgpu::Sampler,
    pub bind_group: wgpu::BindGroup,
}

impl TextureBindGroup {
    pub fn bind_group_layout(ctx: &WgpuContext) -> wgpu::BindGroupLayout {
        ctx.device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Texture Bind Group"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            })
    }
    pub fn new(ctx: &WgpuContext, texture: &wgpu::TextureView) -> Self {
        let sampler = ctx.device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Texture Sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let bind_group = ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Texture Bind Group"),
            layout: &Self::bind_group_layout(ctx),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(texture),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });
        Self {
            sampler,
            bind_group,
        }
    }
}

impl Deref for TextureBindGroup {
    type Target = wgpu::BindGroup;
    fn deref(&self) -> &Self::Target {
        &self.bind_group
    }
}

pub struct CanvasCamera {
    pub vello_renderer: vello::Renderer,

    /// The width of the viewport
    pub viewport_width: u32,
    /// The height of the viewport
    pub viewport_height: u32,
    /// The scaling factor
    pub scale: f32,
    /// The offset of the viewport
    pub offset: Vec2,

    pub vello_scene: vello::Scene,
    pub vello_texture: wgpu::Texture,
    pub vello_view: wgpu::TextureView,

    pub render_texture: wgpu::Texture,
    pub render_view: wgpu::TextureView,
    pub multisample_view: wgpu::TextureView,
    pub depth_stencil_view: wgpu::TextureView,
    uniforms_buffer: WgpuBuffer<CameraUniforms>,
    pub uniforms_bind_group: CameraUniformsBindGroup,
    pub blend_bind_group: TextureBindGroup,
    pub result_bind_group: TextureBindGroup,
}

impl CanvasCamera {
    pub fn new(ctx: &WgpuContext, viewport_width: u32, viewport_height: u32) -> Self {
        let vello_renderer = vello::Renderer::new(
            &ctx.device,
            vello::RendererOptions {
                surface_format: None,
                antialiasing_support: vello::AaSupport::all(),
                use_cpu: false,
                num_init_threads: NonZeroUsize::new(1),
            },
        )
        .expect("failed to create vello renderer");

        let render_texture = ctx.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Target Texture"),
            size: wgpu::Extent3d {
                width: viewport_width as u32,
                height: viewport_height as u32,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::COPY_SRC
                | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let multisample_texture = ctx.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Multisample Texture"),
            size: wgpu::Extent3d {
                width: viewport_width as u32,
                height: viewport_height as u32,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 4,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let depth_stencil_texture = ctx.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Depth Stencil Texture"),
            size: wgpu::Extent3d {
                width: viewport_width as u32,
                height: viewport_height as u32,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 4,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth24PlusStencil8,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });

        let vello_texture = ctx.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Vello Texture"),
            size: wgpu::Extent3d {
                width: viewport_width as u32,
                height: viewport_height as u32,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::STORAGE_BINDING
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[
                wgpu::TextureFormat::Rgba8Unorm,
                wgpu::TextureFormat::Rgba8UnormSrgb,
            ],
        });

        let uniforms = CameraUniforms {
            view_mat: Mat4::IDENTITY,
        };
        let uniforms_buffer = WgpuBuffer::new_init(
            ctx,
            &[uniforms],
            wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        );
        let uniforms_bind_group = CameraUniformsBindGroup::new(ctx, &uniforms_buffer);

        let format = wgpu::TextureFormat::Rgba8UnormSrgb;
        let render_view = render_texture.create_view(&wgpu::TextureViewDescriptor {
            format: Some(format),
            ..Default::default()
        });
        let multisample_view = multisample_texture.create_view(&wgpu::TextureViewDescriptor {
            format: Some(format),
            ..Default::default()
        });
        let depth_stencil_view =
            depth_stencil_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let vello_view = vello_texture.create_view(&wgpu::TextureViewDescriptor::default());
        // Although the vello texture is Rgba8Unorm format, the vello outputs are Rgba8UnormSrgb
        let blend_input_view = vello_texture.create_view(&wgpu::TextureViewDescriptor {
            format: Some(wgpu::TextureFormat::Rgba8UnormSrgb),
            ..Default::default()
        });
        let blend_bind_group = TextureBindGroup::new(ctx, &blend_input_view);
        let result_bind_group = TextureBindGroup::new(ctx, &render_view);

        let vello_scene = vello::Scene::new();

        Self {
            vello_renderer,
            viewport_width,
            viewport_height,
            scale: 1.0,
            offset: Vec2::ZERO,

            vello_scene,
            vello_texture,
            vello_view,

            render_texture,
            render_view,
            multisample_view,
            depth_stencil_view,
            uniforms_buffer,
            uniforms_bind_group,
            blend_bind_group,
            result_bind_group,
        }
    }
}

impl CanvasCamera {
    pub fn clear_screen(&mut self, wgpu_ctx: &WgpuContext) {
        let mut encoder = wgpu_ctx
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Encoder"),
            });

        // Clear
        {
            let bg = Color::srgba_u8(0x33, 0x33, 0x33, 0xff).to_linear();
            // let bg = Color::srgba_u8(41, 171, 202, 255).to_linear();
            let bg = wgpu::Color {
                r: bg.red as f64,
                g: bg.green as f64,
                b: bg.blue as f64,
                a: bg.alpha as f64,
            };
            let _ = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("VMobject Clear Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.multisample_view,
                    resolve_target: Some(&self.render_view),
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(bg),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_stencil_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(0),
                        store: wgpu::StoreOp::Store,
                    }),
                }),
                occlusion_query_set: None,
                timestamp_writes: None,
            });
        }
        wgpu_ctx.queue.submit(Some(encoder.finish()));
        // self.output_texture_updated = false;
    }

    pub fn update_uniforms(&mut self, wgpu_ctx: &WgpuContext) {
        let mat4 = Mat4::from_scale(Vec3::splat(self.scale))
            * Mat4::from_translation(self.offset.extend(0.0))
            * Mat4::from_scale(vec3(
                1.0 / (0.5 * self.viewport_width as f32),
                1.0 / (0.5 * self.viewport_height as f32),
                1.0,
            ));
        // debug!("[CanvasCamera]: Uniforms: {:?}", mat4);
        wgpu_ctx
            .queue
            .write_buffer(&self.uniforms_buffer, 0, bytemuck::cast_slice(&[mat4]));
    }

    pub fn render(&mut self, ctx: &mut RanimContext, entities: &mut EntitiesStore<Self>) {
        self.update_uniforms(&ctx.wgpu_ctx);
        self.clear_screen(&ctx.wgpu_ctx);
        self.vello_scene.reset();
        // For the entities renders with wgpu, this renders the entities to camera's render_texture
        // For the entities renders with vello, this does nothing
        for (_id, entity) in entities.iter_mut() {
            // trace!("[CanvasCamera] rendering entity {:?}...", id);
            entity.render(ctx, self);
        }

        // This renders the vello scene to the canvas texture
        // trace!("[CanvasCamera] rendering vello scene...");
        self.vello_renderer
            .render_to_texture(
                &ctx.wgpu_ctx.device,
                &ctx.wgpu_ctx.queue,
                &self.vello_scene,
                &self.vello_view,
                &vello::RenderParams {
                    base_color: vello::peniko::Color::TRANSPARENT,
                    width: self.viewport_width,
                    height: self.viewport_height,
                    antialiasing_method: vello::AaConfig::Msaa16,
                },
            )
            .unwrap();

        // TODO: implement a blit pipeline
        // This blends the vello scene onto the canvas texture
        // trace!("[CanvasCamera] blending...");
        let pipeline = ctx.pipelines.get_or_init::<BlendPipeline>(&ctx.wgpu_ctx);
        let mut encoder =
            ctx.wgpu_ctx
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Blend"),
                });

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Blend Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.multisample_view,
                    resolve_target: Some(&self.render_view),
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                ..Default::default()
            });
            pass.set_bind_group(0, self.blend_bind_group.deref(), &[]);
            pass.set_pipeline(pipeline);
            pass.draw(0..6, 0..1);
        }

        ctx.wgpu_ctx.queue.submit(Some(encoder.finish()));
    }
}
