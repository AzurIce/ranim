//! Ranim is an animation engine written in rust based on [`wgpu`].

use std::{
    cell::RefCell,
    fmt::Write,
    path::{Path, PathBuf},
    rc::Rc,
    time::{Duration, Instant},
};

use animation::{Animation, Animator, Timeline};
use components::{HasTransform3d, Transform3d};
use context::{RanimContext, WgpuContext};
use file_writer::{FileWriter, FileWriterBuilder};
pub use glam;
use image::{ImageBuffer, Rgba};
use indicatif::{ProgressBar, ProgressState, ProgressStyle};
use items::Entity;
use render::{
    primitives::{Extract, Primitive},
    CameraFrame, Renderable, Renderer,
};
use utils::{rate_functions::linear, Id, RenderResourceStorage};

pub mod prelude {
    pub use crate::interpolate::Interpolatable;

    pub use crate::animation::entity::creation::{Empty, Fill, Partial, Stroke};
    pub use crate::animation::entity::fading::Opacity;
    pub use crate::animation::entity::interpolate::Alignable;

    pub use crate::items::Blueprint;
    pub use crate::RenderScene;
}

pub mod color;
mod file_writer;
mod interpolate;
pub mod updater;

pub mod animation;
pub mod components;
pub mod context;
pub mod items;
pub mod render;
pub mod utils;
// pub mod world;

/// An `Rabject` is a wrapper of an entity that can be rendered.
///
/// The `Rabject`s with same `Id` will use the same `EntityTimeline` to animate.
///
/// The cloned `Rabject` has same Id and shared render_instance, but with seperate data.
pub struct Rabject<T: Entity> {
    id: Id,
    inner: T,
    render_instance: Rc<RefCell<Box<dyn Extract<T>>>>,
}

impl<T: Entity> Renderable for Rabject<T> {
    fn update_clip_info(&mut self, ctx: &WgpuContext, camera: &CameraFrame) {
        let clip_box = self.inner.clip_box(camera);
        self.render_instance
            .borrow_mut()
            .update_clip_box(ctx, &clip_box);
    }
    fn render(
        &mut self,
        ctx: &WgpuContext,
        pipelines: &mut RenderResourceStorage,
        encoder: &mut wgpu::CommandEncoder,
        uniforms_bind_group: &wgpu::BindGroup,
        multisample_view: &wgpu::TextureView,
        target_view: &wgpu::TextureView,
    ) {
        let mut render_instance = self.render_instance.borrow_mut();
        render_instance.update(ctx, &self.inner);
        render_instance.encode_render_command(
            ctx,
            pipelines,
            encoder,
            uniforms_bind_group,
            multisample_view,
            target_view,
        );
    }
}

impl<T: Entity + Clone> Clone for Rabject<T> {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            inner: self.inner.clone(),
            render_instance: self.render_instance.clone(),
        }
    }
}

impl<T: Entity + 'static> Rabject<T> {
    pub fn id(&self) -> Id {
        self.id
    }
    pub fn new(entity: T) -> Self {
        let render_instance = T::Primitive::default();
        let render_instance = Rc::new(RefCell::new(
            Box::new(render_instance) as Box<dyn Extract<T>>
        ));
        Self {
            id: Id::new(),
            inner: entity,
            render_instance,
        }
    }
}

pub struct SceneDesc {
    pub name: String,
}

pub trait TimelineConstructor {
    fn desc() -> SceneDesc;
    fn construct(&mut self, timeline: &mut Timeline);
}

pub trait RenderScene {
    fn render(self)
    where
        Self: Sized;
    fn render_frame_to_image(self, path: impl AsRef<Path>)
    where
        Self: Sized;
}

impl<T: TimelineConstructor> RenderScene for T {
    fn render(self)
    where
        Self: Sized,
    {
        let mut clip_contructor = self;
        let desc = T::desc();
        let mut app = RanimRenderApp::new(AppOptions {
            output_dir: PathBuf::from(format!("./output/{}", desc.name)),
            ..Default::default()
        });
        let mut timeline = Timeline::new();
        clip_contructor.construct(&mut timeline);
        if timeline.elapsed_secs() == 0.0 {
            timeline.forward(Duration::from_secs_f32(0.1));
        }
        let duration_secs = timeline.elapsed_secs();
        app.render_anim(
            Animation::new(timeline)
                .with_rate_func(linear)
                .with_duration(duration_secs),
        );
    }
    fn render_frame_to_image(self, path: impl AsRef<Path>) {
        let mut clip_contructor = self;
        let desc = T::desc();
        let mut app = RanimRenderApp::new(AppOptions {
            output_dir: PathBuf::from(format!("./output/{}", desc.name)),
            ..Default::default()
        });
        let mut timeline = Timeline::new();
        clip_contructor.construct(&mut timeline);
        if timeline.elapsed_secs() == 0.0 {
            timeline.forward(Duration::from_secs_f32(0.1));
        }
        let duration_secs = timeline.elapsed_secs();
        let mut anim = Animation::new(timeline)
            .with_rate_func(linear)
            .with_duration(duration_secs);
        app.render_anim_frame_to_image(&mut anim, path);
    }
}

/// MARK: RanimRenderApp
pub struct RanimRenderApp {
    ctx: RanimContext,

    // world: World,
    // anim: Box<dyn Animation>,
    renderer: Renderer,

    camera_frame: CameraFrame,

    /// The writer for the output.mp4 video
    video_writer: Option<FileWriter>,
    /// Whether to auto create a [`FileWriter`] to output the video
    video_writer_builder: Option<FileWriterBuilder>,
    /// Whether to save the frames
    save_frames: bool,
    /// fps
    fps: u32,

    frame_count: u32,
    output_dir: PathBuf,
}

pub struct AppOptions {
    pub frame_size: (u32, u32),
    pub frame_rate: u32,
    pub save_frames: bool,
    pub output_dir: PathBuf,
}

impl Default for AppOptions {
    fn default() -> Self {
        Self {
            frame_size: (1920, 1080),
            frame_rate: 60,
            save_frames: false,
            output_dir: PathBuf::from("./output"),
        }
    }
}

impl RanimRenderApp {
    pub fn new(options: AppOptions) -> Self {
        let ctx = RanimContext::new();
        let camera_frame = CameraFrame::new_with_size(
            options.frame_size.0 as usize,
            options.frame_size.1 as usize,
        );
        let mut renderer = Renderer::new(
            &ctx,
            options.frame_size.0 as usize,
            options.frame_size.1 as usize,
        );
        renderer.update_uniforms(&ctx.wgpu_ctx, &camera_frame);
        Self {
            // world: World::new(),
            renderer,
            camera_frame,
            video_writer: None,
            video_writer_builder: Some(
                FileWriterBuilder::default()
                    .with_fps(options.frame_rate)
                    .with_size(options.frame_size.0, options.frame_size.1)
                    .with_file_path(options.output_dir.join("output.mp4")),
            ),
            save_frames: options.save_frames,
            fps: options.frame_rate,
            frame_count: 0,
            ctx,
            output_dir: options.output_dir,
        }
    }
    fn tick_duration(&self) -> Duration {
        Duration::from_secs_f32(1.0 / self.fps as f32)
    }

    pub fn render_anim_frame_to_image(&mut self, anim: &mut Animation, filename: impl AsRef<Path>) {
        // let alpha = sec / anim.duration().as_secs_f32();
        // anim.update_alpha(alpha);
        self.renderer.render(&mut self.ctx, anim);
        let path = self.output_dir.join(filename);
        self.save_frame_to_image(path);
    }

    pub fn render_anim(&mut self, mut anim: Animation) {
        let frames = (anim.duration_secs() * self.fps as f32).ceil() as usize;
        let t = Instant::now();
        let pb = ProgressBar::new(frames as u64);
        pb.set_style(
            ProgressStyle::with_template(
                "[{elapsed_precise}] [{wide_bar:.cyan/blue}] frame {human_pos}/{human_len} ({eta}) {msg}",
            )
            .unwrap()
            .with_key("eta", |state: &ProgressState, w: &mut dyn Write| {
                write!(w, "{:.1}s", state.eta().as_secs_f64()).unwrap()
            })
            .progress_chars("#>-"),
        );
        (0..frames)
            .map(|f| f as f32 / frames as f32)
            .for_each(|alpha| {
                // trace!("rendering frame at alpha = {}", alpha);
                anim.update_alpha(alpha);
                self.renderer.render(&mut self.ctx, &mut anim);
                self.update_frame();
                pb.inc(1);
                pb.set_message(format!(
                    "rendering {:?}/{:?}",
                    Duration::from_secs_f32(alpha * anim.duration_secs()),
                    Duration::from_secs_f32(anim.duration_secs())
                ));
            });

        let msg = format!(
            "rendered {} frames({:?}) in {:?}",
            frames,
            anim.duration_secs(),
            t.elapsed()
        );
        pb.finish_with_message(msg);
    }

    fn update_frame(&mut self) {
        // `output_video` is true
        if let Some(video_writer) = self.video_writer.as_mut() {
            video_writer.write_frame(self.renderer.get_rendered_texture_data(&self.ctx.wgpu_ctx));
        } else if let Some(builder) = self.video_writer_builder.as_ref() {
            self.video_writer
                .get_or_insert(builder.clone().build())
                .write_frame(self.renderer.get_rendered_texture_data(&self.ctx.wgpu_ctx));
        }

        // `save_frames` is true
        if self.save_frames {
            let path = self
                .output_dir
                .join(format!("frames/{:04}.png", self.frame_count));
            self.save_frame_to_image(path);
        }
        self.frame_count += 1;
    }

    // // pub fn render_to_image(&mut self, world: &mut World, filename: impl AsRef<str>) {
    // //     let filename = filename.as_ref();
    // //     world.extract();
    // //     world.prepare(&self.ctx);
    // //     self.renderer.render(&mut self.ctx, &mut world.entities);
    // //     self.save_frame_to_image(PathBuf::from(format!("output/{}/{}", "world", filename)));
    // // }

    pub fn save_frame_to_image(&mut self, path: impl AsRef<Path>) {
        let dir = path.as_ref().parent().unwrap();
        if !dir.exists() {
            std::fs::create_dir_all(dir).unwrap();
        }
        // info!("[Scene]: SAVE FRAME TO IMAGE START");
        // let t = Instant::now();
        let size = self.camera_frame.size;
        let texture_data = self.renderer.get_rendered_texture_data(&self.ctx.wgpu_ctx);
        let buffer: ImageBuffer<Rgba<u8>, &[u8]> =
            ImageBuffer::from_raw(size.0 as u32, size.1 as u32, texture_data).unwrap();
        buffer.save(path).unwrap();
        // info!("[Scene]: SAVE FRAME TO IMAGE END, took {:?}", t.elapsed());
    }
}
