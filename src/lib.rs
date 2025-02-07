//! Ranim is an animation engine written in rust based on [`wgpu`].

use std::{
    fmt::Write,
    path::{Path, PathBuf},
    time::Duration,
};

use animation::{entity::AnimWithParams, Animator, Timeline};
use context::RanimContext;
use file_writer::{FileWriter, FileWriterBuilder};
pub use glam;
use image::{ImageBuffer, Rgba};
use indicatif::{ProgressBar, ProgressState, ProgressStyle};
use log::info;
use render::{CameraFrame, Renderable, Renderer};
use utils::rate_functions::linear;

pub mod prelude {
    pub use crate::interpolate::Interpolatable;

    pub use crate::animation::entity::creation::{Empty, Fill, Partial, Stroke};
    pub use crate::animation::entity::fading::Opacity;
    pub use crate::animation::entity::interpolate::Alignable;

    pub use crate::items::Blueprint;
    pub use crate::RenderScene;

    pub use crate::components::Transformable;
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

pub struct SceneDesc {
    pub name: String,
}

pub trait TimelineConstructor {
    fn desc() -> SceneDesc;
    fn construct(&mut self, timeline: &mut Timeline);
}

pub trait RenderScene {
    fn render(self, options: &AppOptions)
    where
        Self: Sized;
    fn render_frame_to_image(self, path: impl AsRef<Path>)
    where
        Self: Sized;
}

impl<T: TimelineConstructor> RenderScene for T {
    fn render(self, options: &AppOptions)
    where
        Self: Sized,
    {
        let desc = T::desc();
        let mut options = options.clone();
        let default_options = AppOptions::default();
        if options.output_dir == default_options.output_dir {
            options.output_dir = PathBuf::from(format!("./output/{}", desc.name))
        }

        let mut clip_contructor = self;
        let mut app = RanimRenderApp::new(&options);
        let mut timeline = Timeline::new();
        clip_contructor.construct(&mut timeline);
        if timeline.elapsed_secs() == 0.0 {
            timeline.forward(0.1);
        }
        info!("Rendering {:?}", timeline);
        let duration_secs = timeline.elapsed_secs();
        app.render_anim(
            AnimWithParams::new(timeline)
                .with_duration(duration_secs)
                .with_rate_func(linear),
        );
    }
    fn render_frame_to_image(self, path: impl AsRef<Path>) {
        let mut clip_contructor = self;
        let desc = T::desc();
        let mut app = RanimRenderApp::new(&AppOptions {
            output_dir: PathBuf::from(format!("./output/{}", desc.name)),
            ..Default::default()
        });
        let mut timeline = Timeline::new();
        clip_contructor.construct(&mut timeline);
        if timeline.elapsed_secs() == 0.0 {
            timeline.forward(0.1);
        }
        let duration_secs = timeline.elapsed_secs();
        let mut anim = AnimWithParams::new(timeline)
            .with_rate_func(linear)
            .with_duration(duration_secs);
        app.render_to_image(&mut anim, path);
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

#[derive(Debug, Clone)]
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
    pub fn new(options: &AppOptions) -> Self {
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
            output_dir: options.output_dir.clone(),
        }
    }
    // fn tick_duration(&self) -> Duration {
    //     Duration::from_secs_f32(1.0 / self.fps as f32)
    // }

    pub fn render_to_image<T: Renderable>(&mut self, anim: &mut T, filename: impl AsRef<Path>) {
        // let alpha = sec / anim.duration().as_secs_f32();
        // anim.update_alpha(alpha);
        self.renderer.render(&mut self.ctx, anim);
        let path = self.output_dir.join(filename);
        self.save_frame_to_image(path);
    }

    pub fn render_anim<T: Animator>(&mut self, mut anim: AnimWithParams<T>) {
        let frames = (anim.params.duration_secs * self.fps as f32).ceil() as usize;
        let pb = ProgressBar::new(frames as u64);
        pb.set_style(
            ProgressStyle::with_template(
                "[{elapsed_precise}] [{wide_bar:.cyan/blue}] frame {human_pos}/{human_len} (eta {eta}) {msg}",
            )
            .unwrap()
            .with_key("eta", |state: &ProgressState, w: &mut dyn Write| {
                write!(w, "{:.1}s", state.eta().as_secs_f64()).unwrap()
            })
            .progress_chars("#>-"),
        );
        (0..frames)
            .map(|f| f as f32 / (frames - 1) as f32)
            .for_each(|alpha| {
                anim.update_alpha(alpha);
                self.renderer.render(&mut self.ctx, &mut anim);
                self.update_frame();
                pb.inc(1);
                pb.set_message(format!(
                    "rendering {:.1?}/{:.1?}",
                    Duration::from_secs_f32(alpha * anim.params.duration_secs),
                    Duration::from_secs_f32(anim.params.duration_secs)
                ));
            });

        let msg = format!(
            "rendered {} frames({:?})",
            frames,
            Duration::from_secs_f32(anim.params.duration_secs),
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
