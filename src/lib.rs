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
use indicatif::{ProgressBar, ProgressState, ProgressStyle};
use items::{Rabject, camera_frame::CameraFrame};
use timeline::{RanimTimeline, TimeMark, TimelineEvalResult};

use render::{Renderer, primitives::RenderInstances};

// MARK: Prelude
pub mod prelude {
    pub use crate::Ranim;

    #[cfg(feature = "app")]
    pub use crate::app::run_scene_app;
    pub use crate::{AppOptions, render_scene, render_scene_at_sec};
    pub use crate::{SceneMetaTrait, TimelineConstructor};
    pub use ranim_macros::scene;

    pub use crate::items::{Rabject, camera_frame::CameraFrame};
    pub use crate::timeline::RanimTimeline;

    pub use crate::color::prelude::*;
    pub use crate::traits::*;

    pub use crate::items::Blueprint;
    pub use crate::items::LaggedAnim;

    pub use crate::components::Transformable;
}

pub mod color;
mod file_writer;
pub mod timeline;
pub mod traits;

pub mod animation;
#[cfg(feature = "app")]
pub mod app;
pub mod components;
pub mod context;
pub mod items;
pub mod render;
pub mod utils;

#[cfg(feature = "profiling")]
// Since the timing information we get from WGPU may be several frames behind the CPU, we can't report these frames to
// the singleton returned by `puffin::GlobalProfiler::lock`. Instead, we need our own `puffin::GlobalProfiler` that we
// can be several frames behind puffin's main global profiler singleton.
pub(crate) static PUFFIN_GPU_PROFILER: std::sync::LazyLock<
    std::sync::Mutex<puffin::GlobalProfiler>,
> = std::sync::LazyLock::new(|| std::sync::Mutex::new(puffin::GlobalProfiler::default()));

/// A simple wrapper struct
///
/// This is used temporally for avoiding writing lifetime annotations like
/// `fn foo<'t, 'r>(timeline: &'t RanimTimeline, camera: &'r mut Rabject<'t, CameraFrame>)`.
pub struct Ranim<'t, 'r>(pub &'t RanimTimeline, pub &'r mut Rabject<'t, CameraFrame>);

/// The metadata of the Timeline
#[derive(Debug, Clone)]
pub struct SceneMeta {
    pub name: String,
}

pub trait SceneMetaTrait {
    fn meta(&self) -> SceneMeta;
}

/// A [`Scene`] builds a [`RanimTimeline`]
///
/// This trait is automatically implemented for types that implements [`TimelineConstructor`] and [`SceneMetaTrait`].
///
/// A struct with [`ranim_macros::scene`] attribute implements [`SceneMetaTrait`], which is basically a type with [`SceneMeta`].
/// - `#[scene]`: use the *snake_case* of the struct's name (Without the `Scene` suffix) as [`SceneMeta::name`].
/// - `#[scene(name = "<NAME>"])`: use the given name as [`SceneMeta::name`].
///
/// [`render_scene`] and [`render_scene_at_sec`] will output to `<output_dir>/<NAME>/` directory.
///
/// # Examples
/// ```rust
/// use ranim::prelude::*;
///
/// #[scene]
/// struct HelloWorld;
///
/// impl Scene for HelloWorld {
///     fn construct(self, timeline: &RanimTimeline, camera: &Rabject<CameraFrame>) {
///         // ...
///     }
/// }
/// ```
pub trait Scene: TimelineConstructor + SceneMetaTrait {}
impl<T: TimelineConstructor + SceneMetaTrait> Scene for T {}

impl<C: TimelineConstructor, M> TimelineConstructor for (C, M) {
    fn construct<'t: 'r, 'r>(
        self,
        timeline: &'t RanimTimeline,
        camera: &'r mut Rabject<'t, CameraFrame>,
    ) {
        self.0.construct(timeline, camera);
    }
}

impl<C, M: SceneMetaTrait> SceneMetaTrait for (C, M) {
    fn meta(&self) -> SceneMeta {
        self.1.meta()
    }
}

/// A constructor of a [`RanimTimeline`]
pub trait TimelineConstructor {
    /// Construct the timeline
    ///
    /// The `camera` is always the first `Rabject` inserted to the `timeline`, and keeps alive until the end of the timeline.
    fn construct<'t: 'r, 'r>(
        self,
        timeline: &'t RanimTimeline,
        camera: &'r mut Rabject<'t, CameraFrame>,
    );
}

pub fn build_timeline(constructor: impl TimelineConstructor) -> RanimTimeline {
    let timeline = RanimTimeline::new();
    {
        let mut camera = timeline.insert(items::camera_frame::CameraFrame::new());
        constructor.construct(&timeline, &mut camera);
        timeline.sync();
    }
    timeline
}

/// Build the timeline with the scene, and render it
pub fn render_scene(scene: impl Scene, options: &AppOptions) {
    let meta = scene.meta();
    let timeline = build_timeline(scene);
    let mut app = RanimRenderApp::new(options, meta.name);
    app.render_timeline(timeline);
}

/// Build the timeline with the scene, and render it at a given timestamp
pub fn render_scene_at_sec(
    scene: impl Scene,
    sec: f64,
    output_file: impl AsRef<Path>,
    options: &AppOptions,
) {
    let meta = scene.meta();
    let timeline = build_timeline(scene);
    let mut app = RanimRenderApp::new(options, meta.name);
    app.render_timeline_frame(&timeline, sec, output_file);
}

// MARK: AppOptions

/// The options of ranim's rendering app
#[derive(Debug, Clone)]
pub struct AppOptions<'a> {
    /// The height of the frame
    ///
    /// This will be the coordinate in the scene. The width is calculated by the aspect ratio from [`AppOptions::pixel_size`].
    pub frame_height: f64,
    /// The size of the output texture in pixels.
    pub pixel_size: (u32, u32),
    /// The frame rate of the output video.
    pub frame_rate: u32,
    /// Whether to save the frames.
    pub save_frames: bool,
    /// The directory to save the output.
    pub output_dir: &'a str,
    /// The filename of the output video.
    pub output_filename: &'a str,
}

impl Default for AppOptions<'_> {
    fn default() -> Self {
        AppOptions {
            frame_height: 8.0,
            pixel_size: (1920, 1080),
            frame_rate: 60,
            save_frames: false,
            output_dir: "./output",
            output_filename: "output.mp4",
        }
    }
}

/// MARK: RanimRenderApp
struct RanimRenderApp {
    ctx: RanimContext,
    // frame_size: (u32, u32),
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
    scene_name: String,

    pub(crate) render_instances: RenderInstances,
}

impl RanimRenderApp {
    fn new(options: &AppOptions, scene_name: String) -> Self {
        let ctx = RanimContext::new();
        let output_dir = PathBuf::from(options.output_dir);
        let renderer = Renderer::new(
            &ctx,
            options.frame_height,
            options.pixel_size.0 as usize,
            options.pixel_size.1 as usize,
        );
        Self {
            // world: World::new(),
            renderer,
            // frame_size: options.frame_size,
            video_writer: None,
            video_writer_builder: Some(
                FileWriterBuilder::default()
                    .with_fps(options.frame_rate)
                    .with_size(options.pixel_size.0, options.pixel_size.1)
                    .with_file_path(output_dir.join(&scene_name).join(options.output_filename)),
            ),
            save_frames: options.save_frames,
            fps: options.frame_rate,
            frame_count: 0,
            ctx,
            output_dir,
            scene_name,
            render_instances: RenderInstances::default(),
        }
    }

    fn render_timeline(&mut self, timeline: RanimTimeline) {
        #[cfg(feature = "profiling")]
        let (_cpu_server, _gpu_server) = {
            puffin::set_scopes_on(true);
            // default global profiler
            let cpu_server =
                puffin_http::Server::new(&format!("0.0.0.0:{}", puffin_http::DEFAULT_PORT))
                    .unwrap();
            // custom gpu profiler in `PUFFIN_GPU_PROFILER`
            let gpu_server = puffin_http::Server::new_custom(
                &format!("0.0.0.0:{}", puffin_http::DEFAULT_PORT + 1),
                |sink| PUFFIN_GPU_PROFILER.lock().unwrap().add_sink(sink),
                |id| _ = PUFFIN_GPU_PROFILER.lock().unwrap().remove_sink(id),
            )
            .unwrap();
            (cpu_server, gpu_server)
        };

        let frames = timeline.duration_secs() * self.fps as f64;
        let frames = frames.ceil() as usize;
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
            .map(|f| f as f64 / (frames - 1) as f64)
            .for_each(|alpha| {
                #[cfg(feature = "profiling")]
                profiling::scope!("frame");

                let TimelineEvalResult {
                    // EvalResult<CameraFrame>, idx
                    camera_frame,
                    // Vec<(rabject_id, EvalResult<Item>, idx)>
                    items,
                } = {
                    #[cfg(feature = "profiling")]
                    profiling::scope!("eval");

                    timeline.eval_alpha(alpha)
                };

                {
                    #[cfg(feature = "profiling")]
                    profiling::scope!("prepare");
                    items.iter().for_each(|(id, res, idx)| {
                        let last_idx = last_idx.entry(*id).or_insert(-1);
                        let prev_last_idx = *last_idx;
                        *last_idx = *idx as i32;
                        match res {
                            EvalResult::Dynamic(res) => res.prepare_for_id(
                                &self.ctx.wgpu_ctx,
                                &mut self.render_instances,
                                *id,
                            ),
                            EvalResult::Static(res) => {
                                if prev_last_idx != *idx as i32 {
                                    res.prepare_for_id(
                                        &self.ctx.wgpu_ctx,
                                        &mut self.render_instances,
                                        *id,
                                    )
                                }
                            }
                        }
                    });
                    self.ctx.wgpu_ctx.queue.submit([]);
                }

                let render_primitives = items
                    .iter()
                    .filter_map(|(id, res, _)| match res {
                        EvalResult::Dynamic(res) => {
                            res.renderable_of_id(&self.render_instances, *id)
                        }
                        EvalResult::Static(res) => {
                            res.renderable_of_id(&self.render_instances, *id)
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

                {
                    #[cfg(feature = "profiling")]
                    profiling::scope!("render");

                    self.renderer.render(&mut self.ctx, &render_primitives);
                }

                self.update_frame();

                #[cfg(feature = "profiling")]
                profiling::finish_frame!();

                pb.inc(1);
                pb.set_message(format!(
                    "rendering {:.1?}/{:.1?}",
                    Duration::from_secs_f64(alpha * timeline.duration_secs()),
                    Duration::from_secs_f64(timeline.duration_secs())
                ));
            });

        let msg = format!(
            "rendered {} frames({:?})",
            frames,
            Duration::from_secs_f64(timeline.duration_secs()),
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

    fn render_timeline_frame(
        &mut self,
        timeline: &RanimTimeline,
        sec: f64,
        filename: impl AsRef<Path>,
    ) {
        let TimelineEvalResult {
            camera_frame,
            items,
        } = timeline.eval_sec(sec);
        items.iter().for_each(|(id, res, _)| match res {
            EvalResult::Dynamic(res) => {
                res.prepare_for_id(&self.ctx.wgpu_ctx, &mut self.render_instances, *id)
            }
            EvalResult::Static(res) => {
                res.prepare_for_id(&self.ctx.wgpu_ctx, &mut self.render_instances, *id)
            }
        });
        let render_primitives = items
            .iter()
            .filter_map(|(id, res, _)| match res {
                EvalResult::Dynamic(res) => res.renderable_of_id(&self.render_instances, *id),
                EvalResult::Static(res) => res.renderable_of_id(&self.render_instances, *id),
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
        self.save_frame_to_image(self.output_dir.join(&self.scene_name).join(filename));
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
        let buffer = self
            .renderer
            .get_rendered_texture_img_buffer(&self.ctx.wgpu_ctx);
        buffer.save(path).unwrap();
    }
}
