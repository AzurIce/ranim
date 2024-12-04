use std::{
    any::{Any, TypeId},
    collections::HashMap,
};

use bevy_color::Color;
use glam::{vec3, Mat4, Vec3};

use crate::{
    rabject::{ExtractedRabjectWithId, Rabject},
    renderer::Renderer,
    utils::Id,
    RanimContext, WgpuBuffer, WgpuContext,
};

#[allow(unused)]
use log::{debug, trace};

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
/// Uniforms for the camera
pub struct CameraUniforms {
    view_projection_mat: Mat4,
    frame_rescale_factors: Vec3,
    _padding: f32,
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
            label: Some("Camera Uniforms"),
            layout: &Self::bind_group_layout(ctx),
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(uniforms_buffer.as_entire_buffer_binding()),
            }],
        });
        Self { bind_group }
    }
}

pub struct Camera {
    pub frame: CameraFrame,
    pub fps: u32,
    uniforms: CameraUniforms,
    render_texture: wgpu::Texture,
    // multisample_texture: wgpu::Texture,
    // depth_stencil_texture: wgpu::Texture,

    render_view: wgpu::TextureView,
    multisample_view: wgpu::TextureView,
    depth_stencil_view: wgpu::TextureView,

    // output_view: wgpu::TextureView,
    output_staging_buffer: wgpu::Buffer,
    output_texture_data: Option<Vec<u8>>,
    output_texture_updated: bool,

    uniforms_buffer: WgpuBuffer<CameraUniforms>,
    uniforms_bind_group: CameraUniformsBindGroup,
}

pub const OUTPUT_TEXTURE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;

impl Camera {
    pub(crate) fn new(ctx: &RanimContext, width: usize, height: usize, fps: u32) -> Self {
        let frame = CameraFrame::new_with_size(width, height);

        let format = OUTPUT_TEXTURE_FORMAT;
        let ctx = &ctx.wgpu_ctx;
        let render_texture = ctx.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Target Texture"),
            size: wgpu::Extent3d {
                width: width as u32,
                height: height as u32,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::COPY_SRC
                | wgpu::TextureUsages::COPY_DST,
            view_formats: &[
                wgpu::TextureFormat::Rgba8UnormSrgb,
                wgpu::TextureFormat::Rgba8Unorm,
            ],
        });
        let multisample_texture = ctx.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Multisample Texture"),
            size: wgpu::Extent3d {
                width: width as u32,
                height: height as u32,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 4,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[
                wgpu::TextureFormat::Rgba8UnormSrgb,
                wgpu::TextureFormat::Rgba8Unorm,
            ],
        });
        let depth_stencil_texture = ctx.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Depth Stencil Texture"),
            size: wgpu::Extent3d {
                width: width as u32,
                height: height as u32,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 4,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth24PlusStencil8,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let output_staging_buffer = ctx.device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: (width * height * 4) as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        let uniforms = CameraUniforms {
            view_projection_mat: frame.view_projection_matrix(),
            frame_rescale_factors: frame.rescale_factors(),
            _padding: 0.0,
        };
        let uniforms_buffer = WgpuBuffer::new_init(
            ctx,
            &[uniforms],
            wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        );
        let uniforms_bind_group = CameraUniformsBindGroup::new(ctx, &uniforms_buffer);

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
        // let output_view = render_texture.create_view(&wgpu::TextureViewDescriptor {
        //     format: Some(wgpu::TextureFormat::Rgba8UnormSrgb),
        //     ..Default::default()
        // });

        Self {
            frame,
            fps,
            uniforms,
            // Textures
            render_texture,
            // multisample_texture,
            // depth_stencil_texture,
            // Texture views
            render_view,
            multisample_view,
            depth_stencil_view,
            // Outputs
            // output_view,
            output_staging_buffer,
            output_texture_data: None,
            output_texture_updated: false,
            // Uniforms
            uniforms_buffer,
            uniforms_bind_group,
        }
    }

    pub fn render<R: Rabject>(
        &mut self,
        ctx: &mut RanimContext,
        rabjects: &mut HashMap<TypeId, Vec<(Id, Box<dyn Any>)>>,
    ) {
        let Some(rabjects) = rabjects.get_mut(&std::any::TypeId::of::<R>()) else {
            return;
        };
        let rabjects = rabjects
            .iter_mut()
            .map(|(_, rabject)| rabject.downcast_mut::<ExtractedRabjectWithId<R>>().unwrap())
            .collect::<Vec<_>>();

        let wgpu_ctx = ctx.wgpu_ctx();

        // trace!("[Camera] Rendering...");

        // Update the uniforms buffer
        // trace!("[Camera]: Refreshing uniforms...");
        self.refresh_uniforms();
        debug!("[Camera]: Uniforms: {:?}", self.uniforms);
        // trace!("[Camera] uploading camera uniforms to buffer...");
        wgpu_ctx.queue.write_buffer(
            &self.uniforms_buffer,
            0,
            bytemuck::cast_slice(&[self.uniforms]),
        );

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

        let instances = rabjects
            .iter()
            .map(|rabject| &rabject.render_resource)
            .collect::<Vec<_>>();
        let renderer = ctx.renderers.get_or_init_mut::<R::Renderer>(&wgpu_ctx);
        let pipelines = &mut ctx.pipelines;
        renderer.render(
            &wgpu_ctx,
            pipelines,
            &instances,
            &self.multisample_view,
            &self.render_view,
            &self.depth_stencil_view,
            &self.uniforms_bind_group.bind_group,
        );

        self.output_texture_updated = false;
    }

    fn update_rendered_texture_data(&mut self, ctx: &WgpuContext) {
        let mut texture_data =
            self.output_texture_data.take().unwrap_or(vec![
                0;
                self.frame.size.0
                    * self.frame.size.1
                    * 4
            ]);

        let mut encoder = ctx
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        encoder.copy_texture_to_buffer(
            wgpu::ImageCopyTexture {
                aspect: wgpu::TextureAspect::All,
                texture: &self.render_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
            },
            wgpu::ImageCopyBuffer {
                buffer: &self.output_staging_buffer,
                layout: wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some((self.frame.size.0 * 4) as u32),
                    rows_per_image: Some(self.frame.size.1 as u32),
                },
            },
            self.render_texture.size(),
        );
        ctx.queue.submit(Some(encoder.finish()));

        pollster::block_on(async {
            let buffer_slice = self.output_staging_buffer.slice(..);

            // NOTE: We have to create the mapping THEN device.poll() before await
            // the future. Otherwise the application will freeze.
            let (tx, rx) = async_channel::bounded(1);
            buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
                tx.send_blocking(result).unwrap()
            });
            ctx.device.poll(wgpu::Maintain::Wait).panic_on_timeout();
            rx.recv().await.unwrap().unwrap();

            {
                let view = buffer_slice.get_mapped_range();
                texture_data.copy_from_slice(&view);
            }
        });
        self.output_staging_buffer.unmap();

        self.output_texture_data = Some(texture_data);
        self.output_texture_updated = true;
    }

    pub(crate) fn get_rendered_texture(&mut self, ctx: &WgpuContext) -> &[u8] {
        if !self.output_texture_updated {
            // trace!("[Camera] Updating rendered texture data...");
            self.update_rendered_texture_data(ctx);
        }
        &self.output_texture_data.as_ref().unwrap()
    }

    pub fn refresh_uniforms(&mut self) {
        self.uniforms.view_projection_mat = self.frame.view_projection_matrix();
        self.uniforms.frame_rescale_factors = self.frame.rescale_factors();
    }
}

/// Default pos is at the origin, looking to the negative z-axis
pub struct CameraFrame {
    pub fovy: f32,
    pub size: (usize, usize),
    pub pos: Vec3,
    pub rotation: Mat4,
}

impl CameraFrame {
    pub fn new_with_size(width: usize, height: usize) -> Self {
        Self {
            size: (width, height),
            fovy: std::f32::consts::PI / 2.0,
            pos: Vec3::ZERO,
            rotation: Mat4::IDENTITY,
        }
    }
}

impl CameraFrame {
    pub fn view_matrix(&self) -> Mat4 {
        Mat4::look_at_rh(vec3(0.0, 0.0, 540.0), Vec3::NEG_Z, Vec3::Y)
        // self.rotation.inverse() * Mat4::from_translation(-self.pos)
    }

    pub fn projection_matrix(&self) -> Mat4 {
        Mat4::perspective_rh(
            self.fovy,
            self.size.0 as f32 / self.size.1 as f32,
            0.1,
            1000.0,
        )
    }

    pub fn view_projection_matrix(&self) -> Mat4 {
        self.projection_matrix() * self.view_matrix()
    }

    pub fn rescale_factors(&self) -> Vec3 {
        // trace!("[CameraFrame] Calculating rescale factors...");
        let res = Vec3::new(
            2.0 / self.size.0 as f32,
            2.0 / self.size.1 as f32,
            1.0 / self.get_focal_distance(),
        );
        // trace!("[CameraFrame] Rescale factors: {:?}", res);
        res
    }

    pub fn get_focal_distance(&self) -> f32 {
        0.5 * self.size.1 as f32 / (0.5 * self.fovy).tan()
    }
}

impl CameraFrame {
    pub fn set_fovy(&mut self, fovy: f32) {
        self.fovy = fovy;
    }

    pub fn move_to(&mut self, pos: Vec3) {
        self.pos = pos;
    }
}
