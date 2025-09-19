// MARK: Render api
use file_writer::{FileWriter, FileWriterBuilder};
use indicatif::{ProgressBar, ProgressState, ProgressStyle};
use log::{info, trace};
use ranim_core::color::{self, LinearSrgb};
use ranim_core::{Output, Scene, SceneConfig, SceneConstructor, SealedRanimScene, TimeMark};
use ranim_render::{
    RenderEval, Renderer, TimelineEvalResult, primitives::RenderInstances, utils::WgpuContext,
};
use std::time::Duration;
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    time::Instant,
};

mod file_writer;

#[cfg(feature = "profiling")]
use ranim_render::PUFFIN_GPU_PROFILER;

/// Render a scene
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
pub fn render_scene_output(
    constructor: impl SceneConstructor,
    name: String,
    scene_config: &SceneConfig,
    output: &Output,
) {
    use std::time::Instant;

    use log::trace;

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
    clear_color: wgpu::Color,

    width: u32,
    height: u32,
    fps: u32,
    save_frames: bool,
    output_dir: PathBuf,

    pub(crate) render_instances: RenderInstances,
}

impl RanimRenderApp {
    fn new(scene_name: String, scene_config: &SceneConfig, output: &Output) -> Self {
        info!("Checking ffmpeg...");
        let t = Instant::now();
        if let Ok(ffmpeg_path) = which::which("ffmpeg") {
            info!("ffmpeg found at {ffmpeg_path:?}");
        } else {
            use std::path::Path;

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
        let clear_color = color::try_color(scene_config.clear_color)
            .unwrap_or(color::color("#333333ff"))
            .convert::<LinearSrgb>();
        let [r, g, b, a] = clear_color.components.map(|x| x as f64);
        let clear_color = wgpu::Color { r, g, b, a };
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
            clear_color,
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
        let mut last_hash = HashMap::<usize, u64>::new();
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
                        .filter_map(|(id, res, id_hash)| {
                            // Some((*id, res))

                            use ranim_core::animation::EvalResult;
                            let is_same_hash = last_hash
                                .get(id)
                                .map(|hash| hash == id_hash)
                                .unwrap_or(false);

                            let renderable = match res {
                                EvalResult::Dynamic(res) => Some(res.as_ref()),
                                EvalResult::Static(res) => {
                                    if is_same_hash {
                                        Some(res.as_ref())
                                    } else {
                                        None
                                    }
                                }
                            };
                            last_hash.insert(*id, *id_hash);
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
                    .filter_map(|(id, _, _)| self.render_instances.get_render_instance_dyn(*id))
                    .collect::<Vec<_>>();
                // let camera_frame = match &camera_frame.0 {
                //     EvalResult::Dynamic(res) => res.as_ref(),
                //     EvalResult::Static(res) => res.as_ref(),
                // };
                // println!("{:?}", camera_frame);
                // println!("{}", render_primitives.len());
                self.renderer.update_uniforms(&self.ctx, &camera_frame.0);

                {
                    #[cfg(feature = "profiling")]
                    profiling::scope!("render");

                    self.renderer
                        .render(&self.ctx, self.clear_color, &render_primitives);
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
        let TimelineEvalResult {
            camera_frame,
            visual_items,
        } = timeline.eval_sec(sec);

        let extracted = visual_items
            .iter()
            .map(|(id, res, _)| {
                // let renderable = match res {
                //     EvalResult::Dynamic(res) => res.extract_renderable(),
                //     EvalResult::Static(res) => res.extract_renderable(),
                // };
                (*id, res)
            })
            .collect::<Vec<_>>();

        extracted.iter().for_each(|(id, renderable)| {
            renderable.prepare_for_id(&self.ctx, &mut self.render_instances, *id)
        });
        let render_primitives = visual_items
            .iter()
            .filter_map(|(id, _, _)| self.render_instances.get_render_instance_dyn(*id))
            .collect::<Vec<_>>();
        // let camera_frame = match &camera_frame.0 {
        //     EvalResult::Dynamic(res) => res,
        //     EvalResult::Static(res) => res,
        // };
        // println!("{:?}", camera_frame);
        // println!("{}", render_primitives.len());
        self.renderer.update_uniforms(&self.ctx, &camera_frame.0);
        self.renderer
            .render(&self.ctx, self.clear_color, &render_primitives);
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

// MARK: Download ffmpeg
const FFMPEG_RELEASE_URL: &str = "https://github.com/eugeneware/ffmpeg-static/releases/latest";

#[allow(unused)]
pub(crate) fn exe_dir() -> PathBuf {
    std::env::current_exe()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

/// Download latest release of ffmpeg from <https://github.com/eugeneware/ffmpeg-static/releases/latest> to <target_dir>/ffmpeg
pub fn download_ffmpeg(target_dir: impl AsRef<Path>) -> Result<PathBuf, anyhow::Error> {
    use anyhow::Context;
    use itertools::Itertools;
    use log::info;
    use std::io::Read;

    let target_dir = target_dir.as_ref();

    let res = reqwest::blocking::get(FFMPEG_RELEASE_URL).context("failed to get release url")?;
    let url = res.url().to_string();
    let url = url.split("tag").collect_array::<2>().unwrap();
    let url = format!("{}/download/{}", url[0], url[1]);
    info!("ffmpeg release url: {url:?}");

    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    let url = format!("{url}/ffmpeg-win32-x64.gz");
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    let url = format!("{url}/ffmpeg-linux-x64.gz");
    #[cfg(all(target_os = "linux", target_arch = "aarch64"))]
    let url = format!("{url}/ffmpeg-linux-arm64.gz");
    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    let url = format!("{url}/ffmpeg-darwin-x64.gz");
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    let url = format!("{url}/ffmpeg-darwin-arm64.gz");

    info!("downloading ffmpeg from {url:?}...");

    let res = reqwest::blocking::get(&url).context("get err")?;
    let mut decoder = flate2::bufread::GzDecoder::new(std::io::BufReader::new(
        std::io::Cursor::new(res.bytes().unwrap()),
    ));
    let mut bytes = Vec::new();
    decoder
        .read_to_end(&mut bytes)
        .context("GzDecoder decode err")?;
    let ffmpeg_path = target_dir.join("ffmpeg");
    std::fs::write(&ffmpeg_path, bytes).unwrap();

    #[cfg(target_family = "unix")]
    {
        use std::os::unix::fs::PermissionsExt;

        std::fs::set_permissions(&ffmpeg_path, std::fs::Permissions::from_mode(0o755))?;
    }
    info!("ffmpeg downloaded to {target_dir:?}");
    Ok(ffmpeg_path)
}
