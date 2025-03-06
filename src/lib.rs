//! Ranim is an animation engine written in rust based on [`wgpu`].

use std::{
    collections::HashMap,
    fmt::Write,
    path::{Path, PathBuf},
    time::Duration,
};

// use animation::AnimWithParams;
use context::RanimContext;
use eval::EvalResult;
use file_writer::{FileWriter, FileWriterBuilder};
pub use glam;
use image::{ImageBuffer, Rgba};
use indicatif::{ProgressBar, ProgressState, ProgressStyle};
use linkme::distributed_slice;
use log::{info, warn};
use timeline::Timeline;

use render::{
    primitives::{RenderInstance, RenderInstances},
    CameraFrame, Renderer,
};

pub mod prelude {
    pub use crate::timeline::{timeline, Timeline};
    pub use crate::{render_timeline, render_timeline_frame};

    pub use crate::color::prelude::*;
    pub use crate::interpolate::Interpolatable;

    pub use crate::animation::creation::{Color, Empty, Fill, Partial, Stroke};
    pub use crate::animation::fading::Opacity;
    pub use crate::animation::transform::Alignable;

    pub use crate::items::Blueprint;
    pub use crate::RenderScene;

    pub use crate::components::Transformable;
}

pub mod color;
mod file_writer;
mod interpolate;
pub mod timeline;
pub mod updater;

pub mod animation;
pub mod components;
pub mod context;
pub mod eval;
pub mod items;
pub mod render;
pub mod utils;

#[distributed_slice]
pub static TIMELINES: [(&'static str, fn(&Timeline), AppOptions<'static>)];

#[macro_export]
macro_rules! render_timeline {
    ($func:ident) => {
        let (name, func, options) = ::ranim::TIMELINES
            .iter()
            .find(|(name, ..)| *name == stringify!($func))
            .unwrap();

        println!("building timeline...");
        let timeline = Timeline::new();
        (func)(&timeline);
        println!("done");
        if timeline.duration_secs() == 0.0 {
            // timeline.forward(0.1);
        }
        let duration_secs = timeline.duration_secs();
        let mut app = ::ranim::RanimRenderApp::new(&options);
        app.render_timeline(timeline);
    };
}

#[macro_export]
macro_rules! render_timeline_frame {
    ($func:ident, $alpha:expr, $filepath:expr) => {
        let (name, func, options) = ::ranim::TIMELINES
            .iter()
            .find(|(name, ..)| *name == stringify!($func))
            .unwrap();

        let timeline = Timeline::new();
        (func)(&timeline);
        if timeline.duration_secs() == 0.0 {
            // timeline.forward(0.1);
        }
        let duration_secs = timeline.duration_secs();
        let mut app = ::ranim::RanimRenderApp::new(&options);
        // app.render_anim_frame(
        //     ::ranim::animation::AnimWithParams::new(timeline)
        //         .with_duration(duration_secs)
        //         .with_rate_func(::ranim::utils::rate_functions::linear),
        //     $alpha,
        //     $filepath,
        // );
    };
}

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
        let output_dir = format!("./output/{}", desc.name);
        if options.output_dir == default_options.output_dir {
            options.output_dir = output_dir.as_str()
        }

        let mut clip_contructor = self;
        let mut app = RanimRenderApp::new(&options);
        let mut timeline = Timeline::new();
        clip_contructor.construct(&mut timeline);
        if timeline.duration_secs() == 0.0 {
            warn!("Timeline's elapsed_secs is 0");
            // timeline.forward(0.1);
        }
        info!("Rendering {:?}", timeline);
        let duration_secs = timeline.duration_secs();
        app.render_timeline(timeline);
        // app.render_anim(
        //     AnimWithParams::new(timeline)
        //         .with_duration(duration_secs)
        //         .with_rate_func(linear),
        // );
    }
    fn render_frame_to_image(self, path: impl AsRef<Path>) {
        let mut clip_contructor = self;
        let desc = T::desc();
        let mut app = RanimRenderApp::new(&AppOptions {
            output_dir: format!("./output/{}", desc.name).as_str(),
            ..Default::default()
        });
        let mut timeline = Timeline::new();
        clip_contructor.construct(&mut timeline);
        if timeline.duration_secs() == 0.0 {
            warn!("Timeline's elapsed_secs is 0")
            // timeline.forward(0.1);
        }
        info!("Rendering {:?}", timeline);
        let duration_secs = timeline.duration_secs();
        // let mut anim = AnimWithParams::new(timeline)
        //     .with_duration(duration_secs)
        //     .with_rate_func(linear);
        // anim.prepare_alpha(0.0, &app.ctx.wgpu_ctx, &mut app.renderer.render_instances);
        // app.render_to_image(&mut anim, path);
    }
}

// MARK: AppOptions

pub static DEFAULT_APP_OPTIONS: AppOptions = AppOptions {
    frame_size: (1920, 1080),
    frame_rate: 60,
    save_frames: false,
    output_dir: "./output",
};

#[derive(Debug, Clone)]
pub struct AppOptions<'a> {
    pub frame_size: (u32, u32),
    pub frame_rate: u32,
    pub save_frames: bool,
    pub output_dir: &'a str,
}

impl Default for AppOptions<'_> {
    fn default() -> Self {
        DEFAULT_APP_OPTIONS.clone()
    }
}

/// MARK: RanimRenderApp
pub struct RanimRenderApp {
    ctx: RanimContext,

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

    pub(crate) render_instances: RenderInstances,
}

impl RanimRenderApp {
    pub fn new(options: &AppOptions) -> Self {
        let ctx = RanimContext::new();
        let camera_frame = CameraFrame::new_with_size(
            options.frame_size.0 as usize,
            options.frame_size.1 as usize,
        );
        let output_dir = PathBuf::from(options.output_dir);
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
                    .with_file_path(output_dir.join("output.mp4")),
            ),
            save_frames: options.save_frames,
            fps: options.frame_rate,
            frame_count: 0,
            ctx,
            output_dir,
            render_instances: RenderInstances::default(),
        }
    }
    // fn tick_duration(&self) -> Duration {
    //     Duration::from_secs_f32(1.0 / self.fps as f32)
    // }

    pub fn render_to_image<T: RenderInstance>(&mut self, anim: &mut T, filename: impl AsRef<Path>) {
        // let alpha = sec / anim.duration().as_secs_f32();
        // anim.update_alpha(alpha);
        self.renderer.render(&mut self.ctx, anim);
        let path = self.output_dir.join(filename);
        self.save_frame_to_image(path);
    }

    pub fn render_timeline(&mut self, timeline: Timeline) {
        let frames = (timeline.duration_secs() * self.fps as f32).ceil() as usize;
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
        let mut last_idx = HashMap::new();
        (0..frames)
            .map(|f| f as f32 / (frames - 1) as f32)
            .for_each(|alpha| {
                // TODO: eval camera_timeline -> Camera, eval every entity_timeline -> &[Entity]
                // TODO: update camera render instance, update every entity render instance
                let eval_results = timeline.eval_alpha(alpha);
                eval_results.iter().for_each(|(id, res, idx)| {
                    let last_idx = last_idx.entry(id.clone()).or_insert(-1);
                    let prev_last_idx = *last_idx;
                    *last_idx = *idx as i32;
                    match res {
                        EvalResult::Dynamic(res) => res.prepare_render_instance_for_entity(
                            &self.ctx.wgpu_ctx,
                            &mut self.render_instances,
                            *id,
                        ),
                        EvalResult::Static(res) => {
                            if prev_last_idx != *idx as i32 {
                                res.prepare_render_instance_for_entity(
                                    &self.ctx.wgpu_ctx,
                                    &mut self.render_instances,
                                    *id,
                                )
                            }
                        }
                    }
                });
                let render_primitives = eval_results
                    .iter()
                    .filter_map(|(id, res, _)| match res {
                        EvalResult::Dynamic(res) => {
                            res.get_render_instance_for_entity(&self.render_instances, *id)
                        }
                        EvalResult::Static(res) => {
                            res.get_render_instance_for_entity(&self.render_instances, *id)
                        }
                    })
                    .collect::<Vec<_>>();
                self.renderer.render(&mut self.ctx, &render_primitives);
                self.update_frame();
                pb.inc(1);
                pb.set_message(format!(
                    "rendering {:.1?}/{:.1?}",
                    Duration::from_secs_f32(alpha * timeline.duration_secs()),
                    Duration::from_secs_f32(timeline.duration_secs())
                ));
            });

        let msg = format!(
            "rendered {} frames({:?})",
            frames,
            Duration::from_secs_f32(timeline.duration_secs()),
        );
        pb.finish_with_message(msg);
    }

    // pub fn render_anim_frame<T: DynamicRenderable>(
    //     &mut self,
    //     mut anim: AnimWithParams<T>,
    //     alpha: f32,
    //     filepath: impl AsRef<Path>,
    // ) {
    //     anim.prepare_alpha(
    //         alpha,
    //         &self.ctx.wgpu_ctx,
    //         &mut self.renderer.render_instances,
    //     );
    //     self.render_to_image(&mut anim, self.output_dir.join(filepath));
    // }

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
