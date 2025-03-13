//! Ranim is an animation engine written in rust based on [`wgpu`], inspired by [3b1b/manim](https://github.com/3b1b/manim/) and [jkjkil4/JAnim](https://github.com/jkjkil4/JAnim).
//! 
//! 

use std::{
    collections::HashMap,
    fmt::Write,
    path::{Path, PathBuf},
    time::Duration,
};

use animation::EvalResult;
use context::RanimContext;
use file_writer::{FileWriter, FileWriterBuilder};
pub use glam;
use image::{ImageBuffer, Rgba};
use indicatif::{ProgressBar, ProgressState, ProgressStyle};
use items::{camera_frame::CameraFrame, Rabject};
use linkme::distributed_slice;
use timeline::{RanimTimeline, TimeMark, TimelineEvalResult};

use render::{
    primitives::{RenderInstance, RenderInstances},
    Renderer,
};

// MARK: Prelude
pub mod prelude {
    pub use crate::Ranim;

    pub use crate::timeline::{timeline, RanimTimeline};
    pub use crate::{render_timeline, render_timeline_frame};

    pub use crate::color::prelude::*;
    pub use crate::interpolate::Interpolatable;

    pub use crate::animation::creation::{Color, Empty, Fill, Partial, Stroke};
    pub use crate::animation::fading::Opacity;
    pub use crate::animation::transform::Alignable;

    pub use crate::items::Blueprint;

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
pub mod items;
pub mod render;
pub mod utils;

pub struct Ranim<'t, 'r>(pub &'t RanimTimeline, pub &'r mut Rabject<'t, CameraFrame>);

#[distributed_slice]
pub static TIMELINES: [(&'static str, fn(Ranim), AppOptions<'static>)];

pub fn build_timeline(func: &fn(Ranim), options: &AppOptions) -> RanimTimeline {
    println!("building timeline...");
    let timeline = RanimTimeline::new();
    let mut camera = timeline.insert(items::camera_frame::CameraFrame::new_with_size(
        options.frame_size.0 as usize,
        options.frame_size.1 as usize,
    ));
    (func)(Ranim(&timeline, &mut camera));
    timeline.sync();
    drop(camera);
    println!("done");
    timeline
}

#[macro_export]
macro_rules! render_timeline {
    ($func:ident) => {
        let (name, func, options) = ::ranim::TIMELINES
            .iter()
            .find(|(name, ..)| *name == stringify!($func))
            .unwrap();

        let timeline = ::ranim::build_timeline(func, options);
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
    ($func:ident, $sec:expr, $filepath:expr) => {
        let (name, func, options) = ::ranim::TIMELINES
            .iter()
            .find(|(name, ..)| *name == stringify!($func))
            .unwrap();

        let timeline = ::ranim::build_timeline(func, options);
        if timeline.duration_secs() == 0.0 {
            // timeline.forward(0.1);
        }
        let duration_secs = timeline.duration_secs();
        let mut app = ::ranim::RanimRenderApp::new(&options);
        app.render_timeline_frame(&timeline, $sec, $filepath)
        // app.render_anim_frame(
        //     ::ranim::animation::AnimWithParams::new(timeline)
        //         .with_duration(duration_secs)
        //         .with_rate_func(::ranim::utils::rate_functions::linear),
        //     $alpha,
        //     $filepath,
        // );
    };
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
    frame_size: (u32, u32),

    renderer: Renderer,

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
        let output_dir = PathBuf::from(options.output_dir);
        let renderer = Renderer::new(
            &ctx,
            options.frame_size.0 as usize,
            options.frame_size.1 as usize,
        );
        Self {
            // world: World::new(),
            renderer,
            frame_size: options.frame_size,
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

    pub fn render_timeline(&mut self, timeline: RanimTimeline) {
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
                let TimelineEvalResult {
                    camera_frame,
                    items,
                } = timeline.eval_alpha(alpha);
                // println!("eval_results: {}", eval_results.len());
                items.iter().for_each(|(id, res, idx)| {
                    let last_idx = last_idx.entry(*id).or_insert(-1);
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
                let render_primitives = items
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
                let camera_frame = match &camera_frame.0 {
                    EvalResult::Dynamic(res) => res,
                    EvalResult::Static(res) => res,
                };
                // println!("{:?}", camera_frame);
                // println!("{}", render_primitives.len());
                self.renderer
                    .update_uniforms(&self.ctx.wgpu_ctx, camera_frame);
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

        let timemarks = timeline
            .time_marks()
            .into_iter()
            .filter(|mark| matches!(mark.1, TimeMark::Capture(_)))
            .collect::<Vec<_>>();

        let pb = ProgressBar::new(timemarks.len() as u64)
            .with_message("saving capture frames from time marks...");
        pb.set_style(
            ProgressStyle::with_template(
                "[{elapsed_precise}] [{wide_bar:.cyan/blue}] capture frame {human_pos}/{human_len} (eta {eta}) {msg}",
            )
            .unwrap()
            .with_key("eta", |state: &ProgressState, w: &mut dyn Write| {
                write!(w, "{:.1}s", state.eta().as_secs_f64()).unwrap()
            })
            .progress_chars("#>-"),
        );
        for (sec, TimeMark::Capture(filename)) in &timemarks {
            self.render_timeline_frame(&timeline, *sec, filename);
            pb.inc(1);
        }
        pb.finish_with_message(format!(
            "saved {} capture frames from time marks",
            timemarks.len()
        ));
    }

    pub fn render_timeline_frame(
        &mut self,
        timeline: &RanimTimeline,
        sec: f32,
        filename: impl AsRef<Path>,
    ) {
        let TimelineEvalResult {
            camera_frame,
            items,
        } = timeline.eval_sec(sec);
        items.iter().for_each(|(id, res, _)| match res {
            EvalResult::Dynamic(res) => res.prepare_render_instance_for_entity(
                &self.ctx.wgpu_ctx,
                &mut self.render_instances,
                *id,
            ),
            EvalResult::Static(res) => res.prepare_render_instance_for_entity(
                &self.ctx.wgpu_ctx,
                &mut self.render_instances,
                *id,
            ),
        });
        let render_primitives = items
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
        let camera_frame = match &camera_frame.0 {
            EvalResult::Dynamic(res) => res,
            EvalResult::Static(res) => res,
        };
        // println!("{:?}", camera_frame);
        // println!("{}", render_primitives.len());
        self.renderer
            .update_uniforms(&self.ctx.wgpu_ctx, camera_frame);
        self.renderer.render(&mut self.ctx, &render_primitives);
        self.save_frame_to_image(self.output_dir.join(filename));
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
        let size = self.frame_size;
        let texture_data = self.renderer.get_rendered_texture_data(&self.ctx.wgpu_ctx);
        let buffer: ImageBuffer<Rgba<u8>, &[u8]> =
            ImageBuffer::from_raw(size.0, size.1, texture_data).unwrap();
        buffer.save(path).unwrap();
        // info!("[Scene]: SAVE FRAME TO IMAGE END, took {:?}", t.elapsed());
    }
}
