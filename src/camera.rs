use std::{any::TypeId, collections::HashMap};

use glam::{Mat4, Vec3};
use log::{debug, trace, warn};

use crate::{
    pipeline::{
        simple::{SimplePipeline, SimpleVertex},
        RenderPipeline,
    },
    WgpuBuffer, WgpuContext,
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
    pub fn bind_group_layout(ctx: &WgpuContext) -> wgpu::BindGroupLayout {
        ctx.device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Simple Pipeline Uniforms"),
                entries: &[CameraUniforms::as_bind_group_layout_entry()],
            })
    }

    pub fn new(ctx: &WgpuContext, uniforms_buffer: &WgpuBuffer<CameraUniforms>) -> Self {
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
    uniforms: CameraUniforms,
    target_texture: wgpu::Texture,
    depth_texture: wgpu::Texture,
    output_staging_buffer: wgpu::Buffer,

    uniforms_buffer: WgpuBuffer<CameraUniforms>,
    uniforms_bind_group: CameraUniformsBindGroup,
}

impl Camera {
    pub fn new(ctx: &WgpuContext, width: usize, height: usize) -> Self {
        let frame = CameraFrame::new_with_size(width, height);

        let target_texture = ctx.device.create_texture(&wgpu::TextureDescriptor {
            label: None,
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
        let depth_texture = ctx.device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: wgpu::Extent3d {
                width: width as u32,
                height: height as u32,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
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
            uniforms,
            frame,
            target_texture,
            depth_texture,
            output_staging_buffer,
            uniforms_buffer,
            uniforms_bind_group,
        }
    }

    pub fn render(
        &mut self,
        ctx: &WgpuContext,
        vertex_buffer: &WgpuBuffer<SimpleVertex>,
        texture_data: &mut Vec<u8>,
    ) {
        debug!("[Camera] Rendering...");

        // Update the uniforms buffer
        debug!("[Camera]: Refreshing uniforms...");
        self.refresh_uniforms();
        debug!("[Camera]: Uniforms: {:?}", self.uniforms);
        trace!("[Camera] uploading camera uniforms to buffer...");
        ctx.queue.write_buffer(
            &self.uniforms_buffer,
            0,
            bytemuck::cast_slice(&[self.uniforms]),
        );

        if texture_data.len() != self.frame.size.0 * self.frame.size.1 * 4 {
            warn!("[Camera] texture data len mismatch, resizing...");
            texture_data.resize(self.frame.size.0 * self.frame.size.1 * 4, 0);
        }
        enum Pipeline {
            Simple(SimplePipeline),
        }
        let mut pipelines = HashMap::<TypeId, Pipeline>::new();
        let id = std::any::TypeId::of::<SimplePipeline>();
        let pipeline = SimplePipeline::new(&ctx);
        pipelines.insert(id, Pipeline::Simple(pipeline));

        let texture_view = self
            .target_texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let depth_view = self
            .depth_texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = ctx
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        {
            let pipeline = pipelines.get(&id).unwrap();
            match pipeline {
                Pipeline::Simple(p) => p.render(
                    &mut encoder,
                    &texture_view,
                    Some(&depth_view),
                    &vertex_buffer,
                    &[&self.uniforms_bind_group.bind_group],
                ),
            }
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
        }

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
    }

    pub fn refresh_uniforms(&mut self) {
        self.uniforms.view_mat = self.frame.view_matrix();
        self.uniforms.frame_rescale_factors = self.frame.rescale_factors();
    }
}

/// Default pos is at the origin, looking to the negative z-axis
pub struct CameraFrame {
    fovy: f32,
    size: (usize, usize),
    pos: Vec3,
    rotation: Mat4,
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
