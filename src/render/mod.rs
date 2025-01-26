pub mod pipelines;
pub mod primitives;

use bevy_color::Color;
use glam::{vec2, Mat4, Vec2, Vec3};
use primitives::Primitive;

use crate::{
    context::{RanimContext, WgpuContext},
    items::{vitem::VItem, Entity},
    utils::wgpu::WgpuBuffer,
    world::World,
};

#[repr(C, align(16))]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
/// Uniforms for the camera
pub struct CameraUniforms {
    proj_mat: Mat4,
    view_mat: Mat4,
    half_frame_size: Vec2,
    _padding: [f32; 2],
}

impl CameraUniforms {
    pub fn as_bind_group_layout_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
        wgpu::BindGroupLayoutEntry {
            binding,
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

impl AsRef<wgpu::BindGroup> for CameraUniformsBindGroup {
    fn as_ref(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }
}

impl CameraUniformsBindGroup {
    pub(crate) fn bind_group_layout(ctx: &WgpuContext) -> wgpu::BindGroupLayout {
        ctx.device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Simple Pipeline Uniforms"),
                entries: &[CameraUniforms::as_bind_group_layout_entry(0)],
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

/// A Renderer to render the [`World`]
///
/// The [`Renderer`] is just responsible to render the current state
/// of the [`World`] to the texture with the setted uniforms.
///
///
pub struct Renderer {
    size: (usize, usize),
    camera: CameraFrame,

    clear_color: wgpu::Color,
    uniforms: CameraUniforms,
    render_texture: wgpu::Texture,
    // multisample_texture: wgpu::Texture,
    // depth_stencil_texture: wgpu::Texture,
    pub(crate) render_view: wgpu::TextureView,
    pub(crate) multisample_view: wgpu::TextureView,
    pub(crate) depth_stencil_view: wgpu::TextureView,

    // output_view: wgpu::TextureView,
    output_staging_buffer: wgpu::Buffer,
    output_texture_data: Option<Vec<u8>>,
    pub(crate) output_texture_updated: bool,

    uniforms_buffer: WgpuBuffer<CameraUniforms>,
    pub(crate) uniforms_bind_group: CameraUniformsBindGroup,
}

pub const OUTPUT_TEXTURE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;
const ALIGNMENT: usize = 256;

impl Renderer {
    pub(crate) fn new(ctx: &RanimContext, width: usize, height: usize) -> Self {
        let camera = CameraFrame::new_with_size(width, height);

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
        let bytes_per_row = ((width * 4) as f32 / ALIGNMENT as f32).ceil() as usize * ALIGNMENT;
        let output_staging_buffer = ctx.device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: (bytes_per_row * height) as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        let uniforms = CameraUniforms {
            proj_mat: camera.projection_matrix(),
            view_mat: camera.view_matrix(),
            half_frame_size: Vec2::new(width as f32 / 2.0, height as f32 / 2.0),
            _padding: [0.0; 2],
        };
        // trace!("init renderer uniform: {:?}", uniforms);
        let uniforms_buffer = WgpuBuffer::new_init(
            ctx,
            Some("Uniforms Buffer"),
            wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            uniforms,
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
        let bg = Color::srgba_u8(0x33, 0x33, 0x33, 0xff).to_linear();
        // let bg = Color::srgba_u8(41, 171, 202, 255).to_linear();
        let clear_color = wgpu::Color {
            r: bg.red as f64,
            g: bg.green as f64,
            b: bg.blue as f64,
            a: bg.alpha as f64,
        };

        Self {
            camera,
            size: (width, height),
            clear_color,
            // fps,
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

    pub fn clear_screen(&mut self, wgpu_ctx: &WgpuContext) {
        // trace!("clear screen {:?}", self.clear_color);
        let mut encoder = wgpu_ctx
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Encoder"),
            });

        // Clear
        {
            let _ = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("VMobject Clear Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.multisample_view,
                    resolve_target: Some(&self.render_view),
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(self.clear_color),
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
        self.output_texture_updated = false;
    }

    fn render_entities<T: Entity + 'static>(
        &mut self,
        ctx: &mut RanimContext,
        encoder: &mut wgpu::CommandEncoder,
        world: &mut World,
    ) {
        if let Some(store) = world.entity_stores.get_store_mut::<T>() {
            for (_id, entity) in store.iter_mut() {
                let Some(primitive) = &mut entity.primitive else {
                    continue;
                };

                primitive.update_clip_info(&ctx.wgpu_ctx, &self.camera);
                primitive.encode_render_command(
                    &ctx.wgpu_ctx,
                    &mut ctx.pipelines,
                    encoder,
                    &self.uniforms_bind_group.bind_group,
                    &self.multisample_view,
                    &self.render_view,
                );
            }
        }
    }

    pub fn render(&mut self, ctx: &mut RanimContext, world: &mut World) {
        self.clear_screen(&ctx.wgpu_ctx);

        let mut encoder = ctx
            .wgpu_ctx
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

        self.render_entities::<VItem>(ctx, &mut encoder, world);

        ctx.wgpu_ctx.queue.submit([encoder.finish()]);

        self.output_texture_updated = false;
    }
    fn update_rendered_texture_data(&mut self, ctx: &WgpuContext) {
        let bytes_per_row =
            ((self.size.0 * 4) as f32 / ALIGNMENT as f32).ceil() as usize * ALIGNMENT;
        let mut texture_data =
            self.output_texture_data
                .take()
                .unwrap_or(vec![0; self.size.0 * self.size.1 * 4]);

        let mut encoder = ctx
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                aspect: wgpu::TextureAspect::All,
                texture: &self.render_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &self.output_staging_buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(bytes_per_row as u32),
                    rows_per_image: Some(self.size.1 as u32),
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
                // texture_data.copy_from_slice(&view);
                for y in 0..self.size.1 {
                    let src_row_start = y * bytes_per_row;
                    let dst_row_start = y * self.size.0 * 4;

                    texture_data[dst_row_start..dst_row_start + self.size.0 * 4]
                        .copy_from_slice(&view[src_row_start..src_row_start + self.size.0 * 4]);
                }
            }
        });
        self.output_staging_buffer.unmap();

        self.output_texture_data = Some(texture_data);
        self.output_texture_updated = true;
    }

    pub(crate) fn get_render_texture(&self) -> &wgpu::Texture {
        &self.render_texture
    }

    pub(crate) fn get_rendered_texture_data(&mut self, ctx: &WgpuContext) -> &[u8] {
        if !self.output_texture_updated {
            // trace!("[Camera] Updating rendered texture data...");
            self.update_rendered_texture_data(ctx);
        }
        self.output_texture_data.as_ref().unwrap()
    }

    pub fn update_uniforms(&mut self, wgpu_ctx: &WgpuContext, camera_frame: &CameraFrame) {
        self.uniforms.proj_mat = camera_frame.projection_matrix();
        self.uniforms.view_mat = camera_frame.view_matrix();
        self.uniforms.half_frame_size = Vec2::new(
            camera_frame.size.0 as f32 / 2.0,
            camera_frame.size.1 as f32 / 2.0,
        );
        // trace!("Uniforms: {:?}", self.uniforms);
        // trace!("[Camera] uploading camera uniforms to buffer...");
        wgpu_ctx.queue.write_buffer(
            &self.uniforms_buffer,
            0,
            bytemuck::cast_slice(&[self.uniforms]),
        );
    }
}

/// Default pos is at the origin, looking to the negative z-axis
pub struct CameraFrame {
    pub fovy: f32,
    pub size: (usize, usize),
    pub pos: Vec3,
    pub up: Vec3,
    pub facing: Vec3,
    // pub rotation: Mat4,
}

impl CameraFrame {
    pub fn new_with_size(width: usize, height: usize) -> Self {
        let mut camera_frame = Self {
            size: (width, height),
            fovy: std::f32::consts::PI / 2.0,
            pos: Vec3::ZERO,
            up: Vec3::Y,
            facing: Vec3::NEG_Z,
            // rotation: Mat4::IDENTITY,
        };
        camera_frame.center_canvas_in_frame(
            Vec3::ZERO,
            width as f32,
            height as f32,
            Vec3::Y,
            Vec3::Z,
        );
        camera_frame
    }
}

impl CameraFrame {
    pub fn ratio(&self) -> f32 {
        self.size.0 as f32 / self.size.1 as f32
    }
    pub fn view_matrix(&self) -> Mat4 {
        // Mat4::look_at_rh(vec3(0.0, 0.0, 1080.0), Vec3::NEG_Z, Vec3::Y)
        Mat4::look_at_rh(self.pos, self.pos + self.facing, self.up)
    }
    pub fn frame_size(&self) -> Vec2 {
        vec2(self.size.0 as f32, self.size.1 as f32)
    }
    pub fn half_frame_size(&self) -> Vec2 {
        self.frame_size() / 2.0
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
}

impl CameraFrame {
    pub fn set_fovy(&mut self, fovy: f32) -> &mut Self {
        self.fovy = fovy;
        self
    }

    pub fn move_to(&mut self, pos: Vec3) -> &mut Self {
        self.pos = pos;
        self
    }

    pub fn center_canvas_in_frame(
        &mut self,
        center: Vec3,
        width: f32,
        height: f32,
        up: Vec3,
        normal: Vec3,
    ) -> &mut Self {
        let canvas_ratio = height / width;
        let up = up.normalize();
        let normal = normal.normalize();

        let height = if self.ratio() > canvas_ratio {
            height
        } else {
            width / self.ratio()
        };

        let distance = height * 0.5 / (0.5 * self.fovy).tan();

        self.up = up;
        self.pos = center + normal * distance;
        self.facing = -normal;
        // trace!(
        //     "[Camera] centered canvas in frame, pos: {:?}, facing: {:?}, up: {:?}",
        //     self.pos,
        //     self.facing,
        //     self.up
        // );

        self
    }
}

/// A render resource.
pub trait RenderResource {
    fn new(ctx: &WgpuContext) -> Self
    where
        Self: Sized;
}

#[cfg(test)]
mod test {
    use env_logger::Env;
    use glam::vec3;
    use image::ImageBuffer;

    use super::*;
    use crate::{
        context::RanimContext,
        items::vitem::{Arc, Square, VItem},
        // rabject::rabject3d::RabjectEntity3d,
        world::{Store, World},
    };

    #[test]
    fn test_render_vitem() {
        env_logger::Builder::from_env(Env::default().default_filter_or("ranim=trace")).init();
        let mut ctx = RanimContext::new();
        let mut world = World::new();
        // let vitem: RabjectEntity3d<VItem> = Square(100.0).build().into();
        // let id = world.insert(vitem);
        // let vitem: RabjectEntity3d<VItem> = Arc {
        //     angle: std::f32::consts::PI / 4.0,
        //     radius: 300.0,
        // }
        // .build()
        // .into();
        let vitem = VItem::from_vpoints(vec![
            vec3(0.0, 0.0, 0.0),
            vec3(50.0, 100.0, 0.0),
            vec3(100.0, 0.0, 0.0),
        ]);
        // let vitem: RabjectEntity3d<VItem> = VItem::square().into();
        let id = world.insert(vitem);
        world.extract();
        world.prepare(&ctx);
        world.prepare(&ctx);

        let mut renderer = Renderer::new(&ctx, 1920, 1080);
        renderer.render(&mut ctx, &mut world);
        let data = renderer.get_rendered_texture_data(&ctx.wgpu_ctx);
        let image = ImageBuffer::<image::Rgba<u8>, _>::from_raw(1920, 1080, data.to_vec()).unwrap();
        image.save("./output.png");
    }
}
