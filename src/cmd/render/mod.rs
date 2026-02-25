// MARK: Render api
use std::collections::VecDeque;

use crate::cmd::render::file_writer::OutputFormatExt;
use crate::{Output, Scene, SceneConfig, SceneConstructor};
use file_writer::{FileWriter, FileWriterBuilder};
use indicatif::{ProgressState, ProgressStyle};
use ranim_core::color::{self, LinearSrgb};
use ranim_core::store::CoreItemStore;
use ranim_core::{SealedRanimScene, TimeMark};
use ranim_render::resource::{RenderPool, RenderTextures};
use ranim_render::{Renderer, utils::WgpuContext};
use std::time::Duration;
use std::{
    path::{Path, PathBuf},
    time::Instant,
};
use tracing::{Span, info, instrument, trace};
use tracing_indicatif::span_ext::IndicatifSpanExt;

mod file_writer;

#[cfg(feature = "profiling")]
use ranim_render::PUFFIN_GPU_PROFILER;

/// Render a scene with all its outputs
pub fn render_scene(scene: &Scene, buffer_count: usize) {
    for (i, output) in scene.outputs.iter().enumerate() {
        info!(
            "Rendering output {}/{} ({})",
            i + 1,
            scene.outputs.len(),
            output.format
        );
        render_scene_output(
            scene.constructor,
            scene.name.to_string(),
            &scene.config,
            output,
            buffer_count,
        );
    }
}

/// Render a scene output
pub fn render_scene_output(
    constructor: impl SceneConstructor,
    name: String,
    scene_config: &SceneConfig,
    output: &Output,
    buffer_count: usize,
) {
    use std::time::Instant;

    let t = Instant::now();
    let scene = constructor.build_scene();
    trace!("Build timeline cost: {:?}", t.elapsed());

    let mut app = RanimRenderApp::new(name, scene_config, output, buffer_count);
    app.render_timeline(&scene);
    if !scene.time_marks().is_empty() {
        app.render_capture_marks(&scene);
    }
}

/// drop it will close the channel and the thread loop will be terminated
struct RenderThreadHandle {
    submit_frame_tx: async_channel::Sender<CoreItemStore>,
    back_rx: async_channel::Receiver<CoreItemStore>,
    worker_rx: async_channel::Receiver<RenderWorker>,
}

impl RenderThreadHandle {
    fn sync_and_submit(&self, f: impl FnOnce(&mut CoreItemStore)) {
        let mut store = self.get_store();
        f(&mut store);
        self.submit_frame_tx.send_blocking(store).unwrap();
    }
    fn get_store(&self) -> CoreItemStore {
        self.back_rx.recv_blocking().unwrap()
    }
    fn retrive(&self) -> RenderWorker {
        self.submit_frame_tx.close(); // This terminates the worker thread loop
        self.worker_rx.recv_blocking().unwrap()
    }
}

struct RenderWorker {
    ctx: WgpuContext,
    renderer: Renderer,
    render_textures: Vec<RenderTextures>,
    pool: RenderPool,
    clear_color: wgpu::Color,
    // video writer
    video_writer: Option<FileWriter>,
    video_writer_builder: Option<FileWriterBuilder>,
    save_frames: bool,
    output_dir: PathBuf,
    scene_name: String,
    width: u32,
    height: u32,
    fps: u32,
}

impl RenderWorker {
    fn new(
        scene_name: String,
        scene_config: &SceneConfig,
        output: &Output,
        buffer_count: usize,
    ) -> Self {
        assert!(buffer_count >= 1, "buffer_count must be at least 1");
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

        let mut output_dir = PathBuf::from(&output.dir);
        if !output_dir.is_absolute() {
            output_dir = std::env::current_dir()
                .unwrap()
                .join("./output")
                .join(output_dir);
        }
        let renderer = Renderer::new(&ctx, output.width, output.height, 8);
        let render_textures: Vec<RenderTextures> = (0..buffer_count)
            .map(|_| renderer.new_render_textures(&ctx))
            .collect();
        let clear_color = color::try_color(&scene_config.clear_color)
            .unwrap_or(color::color("#333333ff"))
            .convert::<LinearSrgb>();
        let [r, g, b, a] = clear_color.components.map(|x| x as f64);
        let clear_color = wgpu::Color { r, g, b, a };

        let (_, _, ext) = output.format.encoding_params();
        Self {
            ctx,
            renderer,
            render_textures,
            pool: RenderPool::new(),
            clear_color,
            video_writer: None,
            video_writer_builder: Some(
                FileWriterBuilder::default()
                    .with_fps(output.fps)
                    .with_size(output.width, output.height)
                    .with_file_path(output_dir.join(format!(
                        "{scene_name}_{}x{}_{}.{ext}",
                        output.width, output.height, output.fps
                    )))
                    .with_output_format(output.format),
            ),
            save_frames: output.save_frames,
            output_dir,
            scene_name,
            width: output.width,
            height: output.height,
            fps: output.fps,
        }
    }

    fn save_frame_dir(&self) -> PathBuf {
        self.output_dir.join(format!(
            "{}_{}x{}_{}-frames",
            self.scene_name, self.width, self.height, self.fps
        ))
    }

    fn yeet(self) -> RenderThreadHandle {
        let (submit_frame_tx, submit_frame_rx) = async_channel::bounded(1);
        let (back_tx, back_rx) = async_channel::bounded(1);
        let (worker_tx, worker_rx) = async_channel::bounded(1);

        back_tx.send_blocking(CoreItemStore::default()).unwrap();
        std::thread::spawn(move || {
            let mut worker = self;
            let n = worker.render_textures.len();
            let mut frame_count = 0u64;
            let mut cur = 0usize;
            let mut pending: VecDeque<(usize, u64)> = VecDeque::new();

            while let Ok(store) = submit_frame_rx.recv_blocking() {
                // Drain oldest pending readback if all targets are occupied
                if pending.len() >= n {
                    let (prev, prev_fc) = pending.pop_front().unwrap();
                    worker.render_textures[prev].finish_readback(&worker.ctx);
                    worker.output_frame_from(prev, prev_fc);
                }

                // Render current frame and start async readback
                worker.renderer.render_store_with_pool(
                    &worker.ctx,
                    &mut worker.render_textures[cur],
                    worker.clear_color,
                    &store,
                    &mut worker.pool,
                );
                worker.render_textures[cur].start_readback(&worker.ctx);
                worker.pool.clean();

                pending.push_back((cur, frame_count));
                frame_count += 1;
                cur = (cur + 1) % n;

                // Return store early so main thread can eval next frame
                // while GPU processes the readback
                back_tx.send_blocking(store).unwrap();

                // Now try to drain any completed readbacks while we wait
                // for the next frame from the main thread
                while let Some(&(prev, _)) = pending.front() {
                    // Non-blocking: check if the oldest readback is ready
                    if !worker.render_textures[prev].try_finish_readback(&worker.ctx) {
                        break;
                    }
                    let (prev, prev_fc) = pending.pop_front().unwrap();
                    worker.output_frame_from(prev, prev_fc);
                }
            }

            // Flush all remaining pending frames
            while let Some((prev, prev_fc)) = pending.pop_front() {
                worker.render_textures[prev].finish_readback(&worker.ctx);
                worker.output_frame_from(prev, prev_fc);
            }

            worker_tx.send_blocking(worker).unwrap();
        });
        RenderThreadHandle {
            submit_frame_tx,
            back_rx,
            worker_rx,
        }
    }

    fn render_store(&mut self, store: &CoreItemStore) {
        #[cfg(feature = "profiling")]
        profiling::scope!("frame");

        {
            #[cfg(feature = "profiling")]
            profiling::scope!("render");

            self.renderer.render_store_with_pool(
                &self.ctx,
                &mut self.render_textures[0],
                self.clear_color,
                store,
                &mut self.pool,
            );
        }
        self.pool.clean();

        #[cfg(feature = "profiling")]
        profiling::finish_frame!();
    }

    /// Write and save (if [`Self::save_frames`] is true)
    fn output_frame_from(&mut self, target_idx: usize, frame_number: u64) {
        self.write_frame_from(target_idx);
        if self.save_frames {
            self.save_frame_from(target_idx, frame_number);
        }
    }

    /// Write frame data from the given target to the video file.
    fn write_frame_from(&mut self, target_idx: usize) {
        let data = self.render_textures[target_idx]
            .render_texture
            .texture_data();
        if let Some(video_writer) = self.video_writer.as_mut() {
            video_writer.write_frame(data);
        } else if let Some(builder) = self.video_writer_builder.as_ref() {
            self.video_writer
                .get_or_insert(builder.clone().build())
                .write_frame(data);
        }
    }

    /// Save frame from the given target as a PNG image.
    fn save_frame_from(&mut self, target_idx: usize, frame_number: u64) {
        let path = self.save_frame_dir().join(format!("{frame_number:04}.png"));
        let dir = path.parent().unwrap();
        if !dir.exists() || !dir.is_dir() {
            std::fs::create_dir_all(dir).unwrap();
        }
        // Data is already in cpu buffer after finish_readback, this won't trigger GPU work
        let buffer = self.render_textures[target_idx].get_rendered_texture_img_buffer(&self.ctx);
        buffer.save(path).unwrap();
    }

    /// Capture frame to image file (sync path, uses target 0).
    pub fn capture_frame(&mut self, path: impl AsRef<Path>) {
        let path = path.as_ref();
        let path = if !path.is_absolute() {
            self.output_dir
                .join(format!(
                    "{}_{}x{}_{}",
                    self.scene_name, self.width, self.height, self.fps
                ))
                .join(path)
        } else {
            path.to_path_buf()
        };
        let dir = path.parent().unwrap();
        if !dir.exists() || !dir.is_dir() {
            std::fs::create_dir_all(dir).unwrap();
        }
        let buffer = self.render_textures[0].get_rendered_texture_img_buffer(&self.ctx);
        buffer.save(path).unwrap();
    }
}

/// MARK: RanimRenderApp
struct RanimRenderApp {
    render_worker: Option<RenderWorker>,
    fps: u32,
    store: CoreItemStore,
}

impl RanimRenderApp {
    fn new(
        scene_name: String,
        scene_config: &SceneConfig,
        output: &Output,
        buffer_count: usize,
    ) -> Self {
        let render_worker = RenderWorker::new(scene_name, scene_config, output, buffer_count);
        Self {
            render_worker: Some(render_worker),
            fps: output.fps,
            store: CoreItemStore::default(),
        }
    }

    #[instrument(skip_all)]
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

        let worker_thread = self.render_worker.take().unwrap().yeet();

        let total_secs = timeline.total_secs();
        let fps = self.fps as f64;
        let raw_frames = total_secs * fps;
        // Add an extra frame to sample the final state exactly,
        // unless total_secs * fps is already an integer (last frame lands on total_secs).
        let n = raw_frames.ceil() as u64;
        let num_frames = if (raw_frames - raw_frames.round()).abs() < 1e-9 {
            n
        } else {
            n + 1
        };
        let style =             ProgressStyle::with_template(
                "[{elapsed_precise}] [{wide_bar:.cyan/blue}] frame {human_pos}/{human_len} (eta {eta}) {msg}",
            )
            .unwrap()
            .with_key("eta", |state: &ProgressState, w: &mut dyn std::fmt::Write| {
                write!(w, "{:.1}s", state.eta().as_secs_f64()).unwrap()
            })
            .progress_chars("#>-");

        let span = Span::current();
        span.pb_set_style(&style);
        span.pb_set_length(num_frames);

        (0..num_frames)
            .map(|f| (f as f64 / fps).min(total_secs))
            .for_each(|sec| {
                worker_thread.sync_and_submit(|store| {
                    store.update(timeline.eval_at_sec(sec));
                });

                span.pb_inc(1);
                span.pb_set_message(
                    format!(
                        "rendering {:.1?}/{:.1?}",
                        Duration::from_secs_f64(sec),
                        Duration::from_secs_f64(total_secs)
                    )
                    .as_str(),
                );
            });
        self.render_worker.replace(worker_thread.retrive());

        info!(
            "rendered {} frames({:?}) in {:?}",
            num_frames,
            Duration::from_secs_f64(timeline.total_secs()),
            start.elapsed(),
        );
        trace!("render timeline cost: {:?}", start.elapsed());
    }

    #[instrument(skip_all)]
    fn render_capture_marks(&mut self, timeline: &SealedRanimScene) {
        let start = Instant::now();
        let timemarks = timeline
            .time_marks()
            .iter()
            .filter(|mark| matches!(mark.1, TimeMark::Capture(_)))
            .collect::<Vec<_>>();

        let style =             ProgressStyle::with_template(
                "[{elapsed_precise}] [{wide_bar:.cyan/blue}] frame {human_pos}/{human_len} (eta {eta}) {msg}",
            )
            .unwrap()
            .with_key("eta", |state: &ProgressState, w: &mut dyn std::fmt::Write| {
                write!(w, "{:.1}s", state.eta().as_secs_f64()).unwrap()
            })
            .progress_chars("#>-");

        let span = Span::current();
        span.pb_set_style(&style);
        span.pb_set_length(timemarks.len() as u64);
        let _enter = span.enter();

        for (sec, TimeMark::Capture(filename)) in &timemarks {
            let alpha = *sec / timeline.total_secs();

            self.store.update(timeline.eval_at_alpha(alpha));
            let worker = self.render_worker.as_mut().unwrap();
            worker.render_store(&self.store);
            worker.capture_frame(filename);
            span.pb_inc(1);
        }
        info!("saved {} capture frames from time marks", timemarks.len());
        trace!("save capture frames cost: {:?}", start.elapsed());
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
    use std::io::Read;
    use tracing::info;

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
