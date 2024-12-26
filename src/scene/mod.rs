/// 2d scene
pub mod canvas;
mod entity;
pub mod file_writer;
mod store;

pub use entity::*;
pub use store::*;

use std::{
    fs,
    ops::{Deref, DerefMut},
    path::{Path, PathBuf},
    time::Duration,
};

use crate::{
    animation::Animation,
    context::RanimContext,
    rabject::{rabject2d::RabjectEntity2d, rabject3d::RabjectEntity3d, Rabject},
};
use bevy_color::Color;
use glam::{vec3, Mat4, Vec3};
use wgpu::RenderPassDescriptor;

use crate::{
    context::WgpuContext, scene::canvas::pipeline::BlendPipeline, utils::wgpu::WgpuBuffer,
};

#[allow(unused)]
use log::{debug, error, info, trace};

use canvas::Canvas;
use file_writer::{FileWriter, FileWriterBuilder};
use image::{ImageBuffer, Rgba};

#[allow(unused_imports)]
use std::time::Instant;

/// A builder for [`Scene`]
pub struct SceneBuilder {
    /// The name of the scene (default: "scene")
    ///
    /// This will be used to name the output files
    pub name: String,
    /// The size of the scene (default: (1920, 1080))
    pub size: (usize, usize),
    /// The fps of the scene (default: 60)
    pub fps: u32,
    /// Interactive mode (WIP) (default: false)
    pub interactive: bool,
    /// Whether to output a video (default: true)
    ///
    /// If this is `true`, then the output video will be saved to `output/<name>/output.mp4`
    pub output_video: bool,
    /// Whether to save frames (default: false)
    ///
    /// If this is `true`, then the output frames will be saved to `output/<name>/frames/<frame_count>.png`
    pub save_frames: bool,
}

impl Default for SceneBuilder {
    fn default() -> Self {
        Self {
            name: "scene".to_string(),
            size: (1920, 1080),
            fps: 60,
            interactive: false,
            output_video: true,
            save_frames: false,
        }
    }
}

impl SceneBuilder {
    /// Create a new [`SceneBuilder`] with the scene name
    ///
    /// The name will be used to name the output files
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            ..Self::default()
        }
    }
    pub fn with_size(mut self, size: (usize, usize)) -> Self {
        self.size = size;
        self
    }
    pub fn with_fps(mut self, fps: u32) -> Self {
        self.fps = fps;
        self
    }
    pub fn enable_interactive(mut self) -> Self {
        self.interactive = true;
        self
    }
    pub fn with_output_video(mut self, output_video: bool) -> Self {
        self.output_video = output_video;
        self
    }
    pub fn with_save_frames(mut self, save_frames: bool) -> Self {
        self.save_frames = save_frames;
        self
    }
    pub fn build(self) -> Scene {
        let mut scene = Scene::new(self.name.clone(), self.size.0, self.size.1, self.fps);
        if self.output_video {
            scene.video_writer_builder = Some(
                FileWriter::builder()
                    .with_file_path(PathBuf::from(format!("output/{}/output.mp4", self.name)))
                    .with_size(self.size.0 as u32, self.size.1 as u32)
                    .with_fps(self.fps),
            );
        }
        scene.save_frames = self.save_frames;
        scene
    }
}

/// A 3d Scene
///
///
pub struct Scene {
    pub ctx: RanimContext,
    /// The name of the scene
    pub name: String,
    pub camera: SceneCamera,
    /// Entities in the scene
    pub entities: EntityStore<SceneCamera>,
    pub time: f32,
    pub frame_count: usize,

    /// The writer for the output.mp4 video
    pub video_writer: Option<FileWriter>,
    /// Whether to auto create a [`FileWriter`] to output the video
    video_writer_builder: Option<FileWriterBuilder>,
    /// Whether to save the frames
    pub save_frames: bool,
}

impl Deref for Scene {
    type Target = EntityStore<SceneCamera>;
    fn deref(&self) -> &Self::Target {
        &self.entities
    }
}

impl DerefMut for Scene {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.entities
    }
}

/// Entity management
impl Scene {
    pub fn insert_new_canvas(&mut self, width: u32, height: u32) -> EntityId<Canvas> {
        let canvas = Canvas::new(&self.ctx.wgpu_ctx, width, height);
        self.entities.insert(canvas)
    }
}

// Core phases
impl Scene {
    pub fn tick(&mut self, dt: f32) {
        // info!("[Scene]: TICK STAGE START");
        // let t = Instant::now();
        self.time += dt;
        for (_, entity) in self.entities.iter_mut() {
            entity.tick(dt);
        }
        // info!("[Scene]: TICK STAGE END, took {:?}", t.elapsed());
    }

    pub fn extract(&mut self) {
        // info!("[Scene]: EXTRACT STAGE START");
        // let t = Instant::now();
        for (_, entity) in self.entities.iter_mut() {
            entity.extract();
        }
        // info!("[Scene]: EXTRACT STAGE END, took {:?}", t.elapsed());
    }

    pub fn prepare(&mut self) {
        // info!("[Scene]: PREPARE STAGE START");
        // let t = Instant::now();
        for (_, entity) in self.entities.iter_mut() {
            entity.prepare(&mut self.ctx);
        }
        // info!("[Scene]: PREPARE STAGE END, took {:?}", t.elapsed());
    }

    pub fn render(&mut self) {
        // info!("[Scene]: RENDER STAGE START");
        // let t = Instant::now();
        self.camera.render(&mut self.ctx, &mut self.entities);
        // info!("[Scene]: RENDER STAGE END, took {:?}", t.elapsed());
    }
}

impl Default for Scene {
    fn default() -> Self {
        let ctx = RanimContext::new();

        Self {
            name: "scene".to_string(),

            camera: SceneCamera::new(&ctx, 1920, 1080, 60),
            entities: EntityStore::default(),
            time: 0.0,
            frame_count: 0,
            video_writer: None,
            video_writer_builder: Some(FileWriterBuilder::default()),
            save_frames: false,

            ctx,
        }
    }
}

impl Scene {
    pub fn builder() -> SceneBuilder {
        SceneBuilder::default()
    }

    /// With default [`FileWriterBuilder`]
    pub(crate) fn new(name: impl Into<String>, width: usize, height: usize, fps: u32) -> Self {
        let name = name.into();

        let mut scene = Self::default();
        scene.name = name;
        scene.camera = SceneCamera::new(&scene.ctx, width, height, fps);
        scene
    }

    /// The size of the camera frame
    ///
    /// for a `scene`, this is equal to `scene.camera.frame.size`
    pub fn size(&self) -> (usize, usize) {
        self.camera.frame.size
    }

    pub fn render_to_image(&mut self, filename: impl AsRef<str>) {
        let filename = filename.as_ref();
        self.extract();
        self.prepare();
        self.render();
        self.save_frame_to_image(PathBuf::from(format!("output/{}/{}", self.name, filename)));
    }

    pub fn update_frame(&mut self, update: bool) {
        // TODO: solve the problem that the new inserted rabjects needs update
        if update || true {
            self.extract();
            self.prepare();
        }
        self.render();

        // `output_video` is true
        if let Some(video_writer) = self.video_writer.as_mut() {
            video_writer.write_frame(self.camera.get_rendered_texture(&self.ctx.wgpu_ctx));
        } else if let Some(builder) = self.video_writer_builder.as_ref() {
            self.video_writer.get_or_insert(builder.clone().build());
        }

        // `save_frames` is true
        if self.save_frames {
            let path = format!("output/{}/frames/{:04}.png", self.name, self.frame_count);
            self.save_frame_to_image(path);
        }
        self.frame_count += 1;
    }

    pub fn save_frame_to_image(&mut self, path: impl AsRef<Path>) {
        let dir = path.as_ref().parent().unwrap();
        if !dir.exists() {
            fs::create_dir_all(dir).unwrap();
        }
        // info!("[Scene]: SAVE FRAME TO IMAGE START");
        // let t = Instant::now();
        let size = self.camera.frame.size;
        let texture_data = self.camera.get_rendered_texture(&self.ctx.wgpu_ctx);
        let buffer: ImageBuffer<Rgba<u8>, &[u8]> =
            ImageBuffer::from_raw(size.0 as u32, size.1 as u32, texture_data).unwrap();
        buffer.save(path).unwrap();
        // info!("[Scene]: SAVE FRAME TO IMAGE END, took {:?}", t.elapsed());
    }

    pub fn tick_duration(&self) -> Duration {
        Duration::from_secs_f32(1.0 / self.camera.fps as f32)
    }

    /// Play an animation
    ///
    /// This is equal to:
    /// ```rust
    /// let run_time = animation.config.run_time.clone();
    /// scene.insert_updater(target_id, animation);
    /// scene.advance(run_time);
    /// ```
    ///
    /// See [`Animation`] and [`Updater`].
    pub fn play<R: Rabject + 'static>(
        &mut self,
        target_id: &EntityId<RabjectEntity3d<R>>,
        animation: Animation<R>,
    ) {
        let run_time = animation.config.run_time;
        self.get_mut(target_id).insert_updater(animation);
        self.advance(run_time);
    }

    pub fn play_remove<R: Rabject + 'static>(
        &mut self,
        target_id: EntityId<RabjectEntity3d<R>>,
        animation: Animation<R>,
    ) {
        self.play(&target_id, animation);
        self.remove(target_id);
    }

    pub fn play_in_canvas<R: Rabject + 'static>(
        &mut self,
        canvas_id: &EntityId<Canvas>,
        target_id: &EntityId<RabjectEntity2d<R>>,
        animation: Animation<R>,
    ) {
        let run_time = animation.config.run_time;
        self.get_mut(canvas_id)
            .get_mut(target_id)
            .insert_updater(animation);
        self.advance(run_time);
    }

    pub fn play_remove_in_canvas<R: Rabject + 'static>(
        &mut self,
        canvas_id: &EntityId<Canvas>,
        target_id: EntityId<RabjectEntity2d<R>>,
        animation: Animation<R>,
    ) {
        self.play_in_canvas(canvas_id, &target_id, animation);
        self.get_mut(canvas_id).remove(target_id);
    }

    /// Advance the scene by a given duration
    ///
    /// this method writes frames
    pub fn advance(&mut self, duration: Duration) {
        let dt = self.tick_duration().as_secs_f32();
        let frames = (duration.as_secs_f32() / dt).ceil() as usize;

        for _ in 0..frames {
            self.tick(dt);
            self.update_frame(true);
        }
    }

    /// Keep the scene static for a given duration
    ///
    /// this method writes frames
    pub fn wait(&mut self, duration: Duration) {
        let dt = self.tick_duration().as_secs_f32();
        let frames = (duration.as_secs_f32() / dt).ceil() as usize;

        for _ in 0..frames {
            self.update_frame(false);
        }
    }
}

#[repr(C, align(16))]
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

pub struct SceneCamera {
    pub frame: CameraFrame,
    pub fps: u32,
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

impl Deref for SceneCamera {
    type Target = CameraFrame;
    fn deref(&self) -> &Self::Target {
        &self.frame
    }
}

impl DerefMut for SceneCamera {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.frame
    }
}

pub const OUTPUT_TEXTURE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;

impl SceneCamera {
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
        self.output_texture_updated = false;
    }

    pub fn update_uniforms(&mut self, wgpu_ctx: &WgpuContext) {
        self.refresh_uniforms();
        // debug!("[Camera]: Uniforms: {:?}", self.uniforms);
        // trace!("[Camera] uploading camera uniforms to buffer...");
        wgpu_ctx.queue.write_buffer(
            &self.uniforms_buffer,
            0,
            bytemuck::cast_slice(&[self.uniforms]),
        );
    }

    pub fn render(&mut self, ctx: &mut RanimContext, entities: &mut EntityStore<Self>) {
        self.update_uniforms(&ctx.wgpu_ctx);
        self.clear_screen(&ctx.wgpu_ctx);
        for (id, entity) in entities.iter_mut() {
            trace!("[Scene] Rendering entity {:?}", id);
            entity.render(ctx, self);
        }
        self.output_texture_updated = false;
    }

    pub fn blend(&mut self, ctx: &mut RanimContext, bind_group: &wgpu::BindGroup) {
        let pipeline = ctx.pipelines.get_or_init::<BlendPipeline>(&ctx.wgpu_ctx);
        let mut encoder =
            ctx.wgpu_ctx
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Blend"),
                });

        {
            let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
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
            pass.set_bind_group(0, bind_group, &[]);
            pass.set_pipeline(pipeline);
            pass.draw(0..6, 0..1);
        }

        ctx.wgpu_ctx.queue.submit(Some(encoder.finish()));
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
        self.output_texture_data.as_ref().unwrap()
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
    pub up: Vec3,
    pub facing: Vec3,
    // pub rotation: Mat4,
}

impl CameraFrame {
    pub fn new_with_size(width: usize, height: usize) -> Self {
        Self {
            size: (width, height),
            fovy: std::f32::consts::PI / 2.0,
            pos: Vec3::ZERO,
            up: Vec3::Y,
            facing: Vec3::NEG_Z,
            // rotation: Mat4::IDENTITY,
        }
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
        Vec3::new(
            2.0 / self.size.0 as f32,
            2.0 / self.size.1 as f32,
            1.0 / self.get_focal_distance(),
        )
    }

    pub fn get_focal_distance(&self) -> f32 {
        0.5 * self.size.1 as f32 / (0.5 * self.fovy).tan()
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

    pub fn center_canvas_in_frame(&mut self, canvas: &Canvas) -> &mut Self {
        let center = canvas.center();
        let canvas_ratio = canvas.height() / canvas.width();

        let height = if self.ratio() > canvas_ratio {
            canvas.height()
        } else {
            canvas.width() / self.ratio()
        };

        let distance = height * 0.5 / (0.5 * self.fovy).tan();

        self.up = canvas.up_normal();
        self.pos = center + canvas.unit_normal() * distance;
        self.facing = -canvas.unit_normal();
        trace!(
            "[Camera] centered canvas in frame, pos: {:?}, facing: {:?}, up: {:?}",
            self.pos,
            self.facing,
            self.up
        );

        self
    }
}
