//! Ranim is an animation engine written in rust based on [`wgpu`], inspired by [3b1b/manim](https://github.com/3b1b/manim/) and [jkjkil4/JAnim](https://github.com/jkjkil4/JAnim).
//!
//!
//! ## Coordinate System
//!
//! Ranim's coordinate system is right-handed coordinate:
//!
//! ```text
//!      +Y
//!      |
//!      |
//!      +----- +X
//!    /
//! +Z
//! ```
//!
#![warn(missing_docs)]
#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![allow(rustdoc::private_intra_doc_links)]
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/AzurIce/ranim/refs/heads/main/assets/ranim.svg",
    html_favicon_url = "https://raw.githubusercontent.com/AzurIce/ranim/refs/heads/main/assets/ranim.svg"
)]

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    time::Duration,
};

use animation::EvalResult;
#[cfg(not(target_arch = "wasm32"))]
use file_writer::{FileWriter, FileWriterBuilder};
pub use glam;
use indicatif::{ProgressBar, ProgressState, ProgressStyle};
use items::{ItemId, camera_frame::CameraFrame};
use log::info;
use timeline::{RanimScene, SealedRanimScene, TimeMark, TimelineEvalResult};

use render::{Renderer, primitives::RenderInstances};

use crate::utils::wgpu::WgpuContext;

// MARK: Prelude
/// The preludes
pub mod prelude {
    pub use crate::AppOptions;
    #[cfg(feature = "app")]
    pub use crate::app::run_scene_app;
    pub use crate::{SceneConstructor, SceneMetaTrait};
    #[cfg(not(target_arch = "wasm32"))]
    pub use crate::{render_scene, render_scene_at_sec};
    pub use ranim_macros::scene;

    pub use crate::items::{ItemId, camera_frame::CameraFrame};
    pub use crate::timeline::{RanimScene, TimelineFunc, TimelinesFunc};

    pub use crate::color::prelude::*;
    pub use crate::traits::*;
}

/// Colors and Palettes
pub mod color;
#[cfg(not(target_arch = "wasm32"))]
mod file_writer;
/// The core structure to encode animations
pub mod timeline;
/// The basic traits for items
pub mod traits;

pub mod animation;
/// The preview app
#[cfg(feature = "app")]
pub mod app;
/// Basic data representation
pub mod components;
/// Builtin items
pub mod items;
/// Rendering stuff
pub mod render;
/// Utils
pub mod utils;

#[cfg(feature = "profiling")]
// Since the timing information we get from WGPU may be several frames behind the CPU, we can't report these frames to
// the singleton returned by `puffin::GlobalProfiler::lock`. Instead, we need our own `puffin::GlobalProfiler` that we
// can be several frames behind puffin's main global profiler singleton.
pub(crate) static PUFFIN_GPU_PROFILER: std::sync::LazyLock<
    std::sync::Mutex<puffin::GlobalProfiler>,
> = std::sync::LazyLock::new(|| std::sync::Mutex::new(puffin::GlobalProfiler::default()));

// ANCHOR: SceneMeta
/// The metadata of the Timeline
#[derive(Debug, Clone)]
pub struct SceneMeta {
    /// The name of the Scene, it is used for output path `<output_dir>/<name>/`
    pub name: String,
}
// ANCHOR_END: SceneMeta

// ANCHOR: SceneMetaTrait
/// A trait for getting [`SceneMeta`]
pub trait SceneMetaTrait {
    /// Get [`SceneMeta`]
    fn meta(&self) -> SceneMeta;
}
// ANCHOR_END: SceneMetaTrait

/// The scene passed to [`render_scene`], simply [`SceneMetaTrait`] + [`SceneConstructor`].
///
/// This trait is automatically implemented for types that implements [`SceneConstructor`] and [`SceneMetaTrait`].
///
/// A struct with [`ranim_macros::scene`] attribute implements [`SceneMetaTrait`], which is basically a type with [`SceneMeta`].
/// - `#[scene]`: use the *snake_case* of the struct's name (Without the `Scene` suffix) as [`SceneMeta::name`].
/// - `#[scene(name = "<name>"])`: use the given name as [`SceneMeta::name`].
///
/// [`render_scene`] and [`render_scene_at_sec`] will output to `<output_dir>/<name>/` directory.
///
/// # Examples
/// ```rust,no_run
/// use ranim::prelude::*;
///
/// #[scene]
/// struct HelloWorld;
///
/// impl SceneConstructor for HelloWorld {
///     fn construct(self, r: &mut RanimScene, r_cam: TimelineId<CameraFrame>) {
///         // ...
///     }
/// }
/// ```
pub trait Scene: SceneConstructor + SceneMetaTrait {}
impl<T: SceneConstructor + SceneMetaTrait> Scene for T {}

impl<C: SceneConstructor, M> SceneConstructor for (C, M) {
    fn construct(self, r: &mut RanimScene, r_cam: ItemId<CameraFrame>) {
        self.0.construct(r, r_cam);
    }
}

impl<C, M: SceneMetaTrait> SceneMetaTrait for (C, M) {
    fn meta(&self) -> SceneMeta {
        self.1.meta()
    }
}

// ANCHOR: SceneConstructor
/// A constructor of a [`RanimScene`]
pub trait SceneConstructor {
    /// Construct the timeline
    ///
    /// The `camera` is always the first `Rabject` inserted to the `timeline`, and keeps alive until the end of the timeline.
    fn construct(self, r: &mut RanimScene, r_cam: ItemId<CameraFrame>);
}
// ANCHOR_END: SceneConstructor

/// A helper function to build a [`SealedRanimScene`] from a [`SceneConstructor`]
pub fn build_timeline(constructor: impl SceneConstructor) -> SealedRanimScene {
    let mut timeline = RanimScene::new();
    {
        let cam = items::camera_frame::CameraFrame::new();
        let r_cam = timeline.insert(cam);
        timeline.timeline_mut(&r_cam).show();
        constructor.construct(&mut timeline, r_cam);
    }
    timeline.seal()
}

/// Build the timeline with the scene, and render it
#[cfg(not(target_arch = "wasm32"))]
pub fn render_scene(scene: impl Scene, options: &AppOptions) {
    let meta = scene.meta();
    let timeline = build_timeline(scene);
    let mut app = RanimRenderApp::new(options, meta.name);
    app.render_timeline(timeline);
}

/// Build the timeline with the scene, and render it at a given timestamp
#[cfg(not(target_arch = "wasm32"))]
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
#[cfg(not(target_arch = "wasm32"))]
struct RanimRenderApp {
    ctx: WgpuContext,
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

#[cfg(not(target_arch = "wasm32"))]
impl RanimRenderApp {
    fn new(options: &AppOptions, scene_name: String) -> Self {
        info!("Checking ffmpeg...");
        if let Ok(ffmpeg_path) = which::which("ffmpeg") {
            info!("ffmpeg found at {ffmpeg_path:?}");
        } else {
            use crate::utils::download_ffmpeg;
            info!(
                "ffmpeg not found from path env, searching in {:?}...",
                Path::new("./").canonicalize().unwrap()
            );
            if Path::new("./ffmpeg").exists() {
                info!("ffmpeg found at current working directory")
            } else {
                info!("ffmpeg not found at current working directory, downloading...");
                download_ffmpeg("./").expect("failed to download ffmpeg");
            }
        }

        info!("Creating wgpu context...");
        let ctx = pollster::block_on(WgpuContext::new());
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

    fn render_timeline(&mut self, timeline: SealedRanimScene) {
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

        let frames = (timeline.total_secs() * self.fps as f64).ceil() as usize;
        let pb = ProgressBar::new(frames as u64);
        pb.set_style(
            ProgressStyle::with_template(
                "[{elapsed_precise}] [{wide_bar:.cyan/blue}] frame {human_pos}/{human_len} (eta {eta}) {msg}",
            )
            .unwrap()
            .with_key("eta", |state: &ProgressState, w: &mut dyn std::fmt::Write| {
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
                    visual_items,
                } = {
                    #[cfg(feature = "profiling")]
                    profiling::scope!("eval");

                    timeline.eval_alpha(alpha)
                };

                let extracted_updates = {
                    #[cfg(feature = "profiling")]
                    profiling::scope!("extract");
                    visual_items
                        .iter()
                        .filter_map(|(id, res, timeline_idx, idx)| {
                            let idx = (*timeline_idx as i32, *idx as i32);
                            let last_idx = last_idx.entry(*id).or_insert((-1, -1));
                            let prev_last_idx = *last_idx;
                            *last_idx = idx;
                            let renderable = match res {
                                EvalResult::Dynamic(res) => Some(res.extract_renderable()),
                                EvalResult::Static(res) => {
                                    if prev_last_idx != idx {
                                        Some(res.extract_renderable())
                                    } else {
                                        None
                                    }
                                }
                            };
                            renderable.map(|renderable| (*id, renderable))
                        })
                        .collect::<Vec<_>>()
                };

                {
                    #[cfg(feature = "profiling")]
                    profiling::scope!("prepare");
                    extracted_updates.iter().for_each(|(id, res)| {
                        res.prepare_for_id(&self.ctx, &mut self.render_instances, *id);
                    });
                    self.ctx.queue.submit([]);
                }

                let render_primitives = visual_items
                    .iter()
                    .filter_map(|(id, _, _, _)| self.render_instances.get_render_instance_dyn(*id))
                    .collect::<Vec<_>>();
                let camera_frame = match &camera_frame.0 {
                    EvalResult::Dynamic(res) => res,
                    EvalResult::Static(res) => res,
                };
                // println!("{:?}", camera_frame);
                // println!("{}", render_primitives.len());
                self.renderer.update_uniforms(&self.ctx, camera_frame);

                {
                    #[cfg(feature = "profiling")]
                    profiling::scope!("render");

                    self.renderer.render(&self.ctx, &render_primitives);
                }

                self.update_frame();

                #[cfg(feature = "profiling")]
                profiling::finish_frame!();

                pb.inc(1);
                pb.set_message(format!(
                    "rendering {:.1?}/{:.1?}",
                    Duration::from_secs_f64(alpha * timeline.total_secs()),
                    Duration::from_secs_f64(timeline.total_secs())
                ));
            });

        let msg = format!(
            "rendered {} frames({:?})",
            frames,
            Duration::from_secs_f64(timeline.total_secs()),
        );
        pb.finish_with_message(msg);

        let timemarks = timeline
            .time_marks()
            .iter()
            .filter(|mark| matches!(mark.1, TimeMark::Capture(_)))
            .collect::<Vec<_>>();

        let pb = ProgressBar::new(timemarks.len() as u64)
            .with_message("saving capture frames from time marks...");
        pb.set_style(
            ProgressStyle::with_template(
                "[{elapsed_precise}] [{wide_bar:.cyan/blue}] capture frame {human_pos}/{human_len} (eta {eta}) {msg}",
            )
            .unwrap()
            .with_key("eta", |state: &ProgressState, w: &mut dyn std::fmt::Write| {
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
        timeline: &SealedRanimScene,
        sec: f64,
        filename: impl AsRef<Path>,
    ) {
        let TimelineEvalResult {
            camera_frame,
            visual_items,
        } = timeline.eval_sec(sec);

        let extracted = visual_items
            .iter()
            .map(|(id, res, _, _)| {
                let renderable = match res {
                    EvalResult::Dynamic(res) => res.extract_renderable(),
                    EvalResult::Static(res) => res.extract_renderable(),
                };
                (*id, renderable)
            })
            .collect::<Vec<_>>();

        extracted.iter().for_each(|(id, renderable)| {
            renderable.prepare_for_id(&self.ctx, &mut self.render_instances, *id)
        });
        let render_primitives = visual_items
            .iter()
            .filter_map(|(id, _, _, _)| self.render_instances.get_render_instance_dyn(*id))
            .collect::<Vec<_>>();
        let camera_frame = match &camera_frame.0 {
            EvalResult::Dynamic(res) => res,
            EvalResult::Static(res) => res,
        };
        // println!("{:?}", camera_frame);
        // println!("{}", render_primitives.len());
        self.renderer.update_uniforms(&self.ctx, camera_frame);
        self.renderer.render(&self.ctx, &render_primitives);
        self.save_frame_to_image(self.output_dir.join(&self.scene_name).join(filename));
    }

    fn update_frame(&mut self) {
        // `output_video` is true
        if let Some(video_writer) = self.video_writer.as_mut() {
            video_writer.write_frame(self.renderer.get_rendered_texture_data(&self.ctx));
        } else if let Some(builder) = self.video_writer_builder.as_ref() {
            self.video_writer
                .get_or_insert(builder.clone().build())
                .write_frame(self.renderer.get_rendered_texture_data(&self.ctx));
        }

        // `save_frames` is true
        if self.save_frames {
            let path = self
                .output_dir
                .join(&self.scene_name)
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
        let buffer = self.renderer.get_rendered_texture_img_buffer(&self.ctx);
        buffer.save(path).unwrap();
    }
}
