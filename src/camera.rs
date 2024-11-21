use std::{
    any::{Any, TypeId},
    collections::HashMap,
};

use glam::{Mat4, Vec3};
use log::{debug, trace};

use crate::{
    mobject::ExtractedMobject, renderer::Renderer, utils::Id, RanimContext, WgpuBuffer, WgpuContext,
};

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
/// Uniforms for the camera
pub struct CameraUniforms {
    view_mat: Mat4,
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
    multisample_texture: wgpu::Texture,
    target_texture: wgpu::Texture,
    texture_data: Option<Vec<u8>>,
    texture_data_updated: bool,
    depth_texture: wgpu::Texture,
    output_staging_buffer: wgpu::Buffer,

    uniforms_buffer: WgpuBuffer<CameraUniforms>,
    uniforms_bind_group: CameraUniformsBindGroup,
}

impl Camera {
    pub(crate) fn new(ctx: &RanimContext, width: usize, height: usize, fps: u32) -> Self {
        let frame = CameraFrame::new_with_size(width, height);

        let ctx = &ctx.wgpu_ctx;
        let target_texture = ctx.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Target Texture"),
            size: wgpu::Extent3d {
                width: width as u32,
                height: height as u32,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[wgpu::TextureFormat::Rgba8UnormSrgb],
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
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[wgpu::TextureFormat::Rgba8UnormSrgb],
        });
        let depth_texture = ctx.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Depth Texture"),
            size: wgpu::Extent3d {
                width: width as u32,
                height: height as u32,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 4,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
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
            view_mat: frame.view_matrix(),
            frame_rescale_factors: frame.rescale_factors(),
            _padding: 0.0,
        };
        let uniforms_buffer = WgpuBuffer::new_init(
            ctx,
            &[uniforms],
            wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        );
        let uniforms_bind_group = CameraUniformsBindGroup::new(ctx, &uniforms_buffer);

        Self {
            frame,
            fps,
            uniforms,
            target_texture,
            multisample_texture,
            texture_data: None,
            texture_data_updated: false,
            depth_texture,
            output_staging_buffer,
            uniforms_buffer,
            uniforms_bind_group,
        }
    }

    pub fn render<R: Renderer + 'static>(
        &mut self,
        ctx: &mut RanimContext,
        objects: &mut HashMap<TypeId, Vec<(Id, Box<dyn Any>)>>,
    ) {
        let objects = objects.entry(std::any::TypeId::of::<R>()).or_default();
        let mut objects = objects
            .into_iter()
            .map(|(_, mobject)| {
                mobject
                    .downcast_mut::<ExtractedMobject<R::Vertex>>()
                    .unwrap()
            })
            .collect::<Vec<_>>();
        trace!("[Camera] Rendering...");

        // Update the uniforms buffer
        trace!("[Camera]: Refreshing uniforms...");
        self.refresh_uniforms();
        debug!("[Camera]: Uniforms: {:?}", self.uniforms);
        trace!("[Camera] uploading camera uniforms to buffer...");
        ctx.wgpu_ctx.queue.write_buffer(
            &self.uniforms_buffer,
            0,
            bytemuck::cast_slice(&[self.uniforms]),
        );

        // Preparing the object
        for object in &mut objects {
            object.prepare(&ctx.wgpu_ctx);
        }

        let target_view = self
            .target_texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let multisample_view = self
            .multisample_texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let depth_view = self
            .depth_texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder =
            ctx.wgpu_ctx
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Render Encoder"),
                });

        {
            let mut render_pass =
                R::begin_pass(&mut encoder, &multisample_view, &target_view, &depth_view);
            // bind group 0 is reserved for camera uniforms
            render_pass.set_bind_group(0, &self.uniforms_bind_group.bind_group, &[]);
            R::render(ctx, &mut render_pass, &mut objects);
        }

        ctx.wgpu_ctx.queue.submit(Some(encoder.finish()));
        self.texture_data_updated = false;
    }

    fn update_rendered_texture_data(&mut self, ctx: &WgpuContext) {
        let mut texture_data =
            self.texture_data
                .take()
                .unwrap_or(vec![0; self.frame.size.0 * self.frame.size.1 * 4]);

        let mut encoder = ctx
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        encoder.copy_texture_to_buffer(
            wgpu::ImageCopyTexture {
                aspect: wgpu::TextureAspect::All,
                texture: &self.target_texture,
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
            self.target_texture.size(),
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

        self.texture_data = Some(texture_data);
        self.texture_data_updated = true;
    }

    pub(crate) fn get_rendered_texture(&mut self, ctx: &WgpuContext) -> &[u8] {
        if !self.texture_data_updated {
            self.update_rendered_texture_data(ctx);
        }
        &self.texture_data.as_ref().unwrap()
    }

    pub fn refresh_uniforms(&mut self) {
        self.uniforms.view_mat = self.frame.view_matrix();
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
            fovy: std::f32::consts::PI / 4.0,
            pos: Vec3::ZERO,
            rotation: Mat4::IDENTITY,
        }
    }
}

impl CameraFrame {
    pub fn view_matrix(&self) -> Mat4 {
        self.rotation.inverse() * Mat4::from_translation(-self.pos)
    }

    pub fn rescale_factors(&self) -> Vec3 {
        trace!("[CameraFrame] Calculating rescale factors...");
        let res = Vec3::new(
            2.0 / self.size.0 as f32,
            2.0 / self.size.1 as f32,
            1.0 / self.get_focal_distance(),
        );
        trace!("[CameraFrame] Rescale factors: {:?}", res);
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
