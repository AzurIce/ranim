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
#![feature(downcast_unchecked)]

use animation::EvalResult;
use log::{info, trace};
use timeline::{RanimScene, SealedRanimScene};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(not(target_arch = "wasm32"))]
use file_writer::{FileWriter, FileWriterBuilder};
#[cfg(not(target_arch = "wasm32"))]
use indicatif::{ProgressBar, ProgressState, ProgressStyle};
#[cfg(not(target_arch = "wasm32"))]
use render::{Renderer, primitives::RenderInstances};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};
#[cfg(not(target_arch = "wasm32"))]
use utils::wgpu::WgpuContext;

pub mod animation;
/// The preview app
#[cfg(feature = "app")]
pub mod app;
/// Colors and Palettes
pub mod color;
/// Basic data representation
pub mod components;
#[cfg(not(target_arch = "wasm32"))]
mod file_writer;
/// Builtin items
pub mod items;
/// Rendering stuff
pub mod render;
/// The core structure to encode animations
pub mod timeline;
/// The basic traits for items
pub mod traits;
/// Utils
pub mod utils;

pub use glam;

// ANCHOR: SceneConstructor
/// A scene constructor
///
/// It can be a simple fn pointer of `fn(&mut RanimScene)`,
/// or any type implements `Fn(&mut RanimScene) + Send + Sync`.
pub trait SceneConstructor: Send + Sync {
    /// The construct logic
    fn construct(&self, r: &mut RanimScene);

    /// Use the constructor to build a [`SealedRanimScene`]
    fn build_scene(&self) -> SealedRanimScene {
        let mut scene = RanimScene::new();
        self.construct(&mut scene);
        scene.seal()
    }
}
// ANCHOR_END: SceneConstructor

impl<F: Fn(&mut RanimScene) + Send + Sync> SceneConstructor for F {
    fn construct(&self, r: &mut RanimScene) {
        self(r);
    }
}

// MARK: Dylib part
#[doc(hidden)]
#[derive(Clone)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub struct Scene {
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen(skip))]
    pub name: &'static str,
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen(skip))]
    pub constructor: fn(&mut RanimScene),
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen(skip))]
    pub config: SceneConfig,
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen(skip))]
    pub outputs: &'static [Output],
    pub preview: bool,
}

pub use inventory;

inventory::collect!(Scene);

#[doc(hidden)]
#[unsafe(no_mangle)]
pub extern "C" fn get_scene(idx: usize) -> *const Scene {
    inventory::iter::<Scene>().skip(idx).take(1).next().unwrap()
}

#[doc(hidden)]
#[unsafe(no_mangle)]
pub extern "C" fn scene_cnt() -> usize {
    inventory::iter::<Scene>().count()
}

#[cfg(target_arch = "wasm32")]
unsafe extern "C" {
    fn __wasm_call_ctors();
}

/// Return a scene with matched name
#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub fn find_scene(name: &str) -> Option<Scene> {
    inventory::iter::<Scene>().find(|s| s.name == name).cloned()
}

#[cfg(target_arch = "wasm32")]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen(start))]
fn wasm_start() {
    unsafe {
        __wasm_call_ctors();
    }
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));
    console_log::init().expect("Failed to initialize console_log");
}

/// Scene config
#[derive(Debug, Clone)]
pub struct SceneConfig {
    /// The height of the frame
    ///
    /// This will be the coordinate in the scene. The width is calculated by the aspect ratio from [`Output::width`] and [`Output::height`].
    pub frame_height: f64,
}

impl Default for SceneConfig {
    fn default() -> Self {
        Self { frame_height: 8.0 }
    }
}

/// The output of a scene
#[derive(Debug, Clone)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub struct Output {
    /// The width of the output texture in pixels.
    pub width: u32,
    /// The height of the output texture in pixels.
    pub height: u32,
    /// The frame rate of the output video.
    pub fps: u32,
    /// Whether to save the frames.
    pub save_frames: bool,
    /// The directory to save the output
    ///
    /// Related to the `output` folder, Or absolute.
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen(skip))]
    pub dir: &'static str,
}

impl Default for Output {
    fn default() -> Self {
        Self::DEFAULT
    }
}

impl Output {
    /// 1920x1080 60fps save_frames=false dir="./"
    pub const DEFAULT: Self = Self {
        width: 1920,
        height: 1080,
        fps: 60,
        save_frames: false,
        dir: "./",
    };
}

// MARK: Prelude
/// The preludes
pub mod prelude {
    #[cfg(feature = "app")]
    pub use crate::app::{preview, run_app, run_scene_app};
    #[cfg(not(target_arch = "wasm32"))]
    pub use crate::{render, render_scene, render_scene_output};

    pub use ranim_macros::{output, preview, wasm_demo_doc, scene};

    pub use crate::items::{ItemId, camera_frame::CameraFrame};
    pub use crate::timeline::{RanimScene, TimelineFunc, TimelinesFunc};

    pub use crate::color::prelude::*;
    pub use crate::traits::*;
}

#[cfg(feature = "profiling")]
// Since the timing information we get from WGPU may be several frames behind the CPU, we can't report these frames to
// the singleton returned by `puffin::GlobalProfiler::lock`. Instead, we need our own `puffin::GlobalProfiler` that we
// can be several frames behind puffin's main global profiler singleton.
pub(crate) static PUFFIN_GPU_PROFILER: std::sync::LazyLock<
    std::sync::Mutex<puffin::GlobalProfiler>,
> = std::sync::LazyLock::new(|| std::sync::Mutex::new(puffin::GlobalProfiler::default()));

/// Render a scene
#[cfg(not(target_arch = "wasm32"))]
pub fn render_scene(scene: &Scene) {
    for output in scene.outputs {
        render_scene_output(
            scene.constructor,
            scene.name.to_string(),
            &scene.config,
            output,
        );
    }
}

/// Render a scene output
#[cfg(not(target_arch = "wasm32"))]
pub fn render_scene_output(
    constructor: impl SceneConstructor,
    name: String,
    scene_config: &SceneConfig,
    output: &Output,
) {
    let t = Instant::now();
    let scene = constructor.build_scene();
    trace!("Build timeline cost: {:?}", t.elapsed());

    let mut app = RanimRenderApp::new(name, scene_config, output);
    app.render_timeline(&scene);
    if !scene.time_marks().is_empty() {
        app.render_capture_marks(&scene);
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
    frame_count: u32,
    scene_name: String,

    width: u32,
    height: u32,
    fps: u32,
    save_frames: bool,
    output_dir: PathBuf,

    pub(crate) render_instances: RenderInstances,
}

#[cfg(not(target_arch = "wasm32"))]
impl RanimRenderApp {
    fn new(scene_name: String, scene_config: &SceneConfig, output: &Output) -> Self {
        use std::time::Instant;

        info!("Checking ffmpeg...");
        let t = Instant::now();
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
        trace!("Check ffmmpeg cost: {:?}", t.elapsed());

        let t = Instant::now();
        info!("Creating wgpu context...");
        let ctx = pollster::block_on(WgpuContext::new());
        trace!("Create wgpu context cost: {:?}", t.elapsed());

        let mut output_dir = PathBuf::from(output.dir);
        if !output_dir.is_absolute() {
            output_dir = std::env::current_dir()
                .unwrap()
                .join("./output")
                .join(output_dir);
        }
        let renderer = Renderer::new(
            &ctx,
            scene_config.frame_height,
            output.width as usize,
            output.height as usize,
        );
        Self {
            // world: World::new(),
            renderer,
            // frame_size: options.frame_size,
            video_writer: None,
            video_writer_builder: Some(
                FileWriterBuilder::default()
                    .with_fps(output.fps)
                    .with_size(output.width, output.height)
                    .with_file_path(output_dir.join(format!(
                        "{scene_name}_{}x{}_{}.mp4",
                        output.width, output.height, output.fps
                    ))),
            ),
            frame_count: 0,
            ctx,
            width: output.width,
            height: output.height,
            fps: output.fps,
            save_frames: output.save_frames,
            output_dir,
            scene_name,
            render_instances: RenderInstances::default(),
        }
    }

    fn render_timeline(&mut self, timeline: &SealedRanimScene) {
        let start = Instant::now();
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
        let frames = frames.max(2);
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
                use crate::timeline::TimelineEvalResult;

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
        trace!("render timeline cost: {:?}", start.elapsed());
    }

    fn render_capture_marks(&mut self, timeline: &SealedRanimScene) {
        use crate::timeline::TimeMark;

        let start = Instant::now();
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
            self.render_timeline_frame(timeline, *sec, filename);
            pb.inc(1);
        }
        pb.finish_with_message(format!(
            "saved {} capture frames from time marks",
            timemarks.len()
        ));
        trace!("save capture frames cost: {:?}", start.elapsed());
    }

    fn render_timeline_frame(
        &mut self,
        timeline: &SealedRanimScene,
        sec: f64,
        filename: impl AsRef<Path>,
    ) {
        use crate::timeline::TimelineEvalResult;

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
        self.save_frame_to_image(
            self.output_dir
                .join(format!(
                    "{}_{}x{}_{}",
                    self.scene_name, self.width, self.height, self.fps
                ))
                .join(filename),
        );
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
                .join(format!(
                    "{}_{}x{}_{}-frames",
                    self.scene_name, self.width, self.height, self.fps
                ))
                .join(format!("{:04}.png", self.frame_count));
            self.save_frame_to_image(path);
        }
        self.frame_count += 1;
    }

    pub fn save_frame_to_image(&mut self, path: impl AsRef<Path>) {
        let dir = path.as_ref().parent().unwrap();
        if !dir.exists() || !dir.is_dir() {
            std::fs::create_dir_all(dir).unwrap();
        }
        let buffer = self.renderer.get_rendered_texture_img_buffer(&self.ctx);
        buffer.save(path).unwrap();
    }
}
