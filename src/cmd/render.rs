// MARK: Render api
use file_writer::{FileWriter, FileWriterBuilder};
use indicatif::{ProgressBar, ProgressState, ProgressStyle};
use log::{info, trace};
use ranim_core::color::{self, LinearSrgb};
use ranim_core::store::CoreItemStore;
use ranim_core::{Output, Scene, SceneConfig, SceneConstructor, SealedRanimScene, TimeMark};
use ranim_render::primitives::RenderPool;
use ranim_render::{Renderer, utils::WgpuContext};
use std::time::Duration;
use std::{
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
    // frame_size: (u32, u32),
    renderer: Renderer,
    pool: RenderPool,
    clear_color: wgpu::Color,
    // video writer
    video_writer: Option<FileWriter>,
    video_writer_builder: Option<FileWriterBuilder>,
    frame_count: u32,
    save_frames: bool,
    output_dir: PathBuf,
    scene_name: String,
    width: u32,
    height: u32,
    fps: u32,
}

impl RenderWorker {
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
            ctx,
            renderer,
            pool: RenderPool::new(),
            clear_color,
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
            save_frames: output.save_frames,
            output_dir,
            scene_name,
            width: output.width,
            height: output.height,
            fps: output.fps,
        }
    }

    fn yeet(self) -> RenderThreadHandle {
        let (submit_frame_tx, submit_frame_rx) = async_channel::bounded(1);
        let (back_tx, back_rx) = async_channel::bounded(1);
        let (worker_tx, worker_rx) = async_channel::bounded(1);

        back_tx.send_blocking(CoreItemStore::default()).unwrap();
        std::thread::spawn(move || {
            let mut worker = self;
            let save_path = if worker.save_frames {
                Some(
                    worker
                        .output_dir
                        .join(format!(
                            "{}_{}x{}_{}-frames",
                            worker.scene_name, worker.width, worker.height, worker.fps
                        ))
                        .join(format!("{:04}.png", worker.frame_count)),
                )
            } else {
                None
            };
            let save_path = save_path.as_deref();

            while let Ok(store) = submit_frame_rx.recv_blocking() {
                worker.render_store(&store, save_path);

                back_tx.send_blocking(store).unwrap();
            }

            worker_tx.send_blocking(worker).unwrap();
        });
        RenderThreadHandle {
            submit_frame_tx,
            back_rx,
            worker_rx,
        }
    }

    fn render_store(&mut self, store: &CoreItemStore, save_path: Option<impl AsRef<Path>>) {
        #[cfg(feature = "profiling")]
        profiling::scope!("frame");

        {
            #[cfg(feature = "profiling")]
            profiling::scope!("render");

            self.renderer.render_store_with_pool(
                &self.ctx,
                self.clear_color,
                store,
                &mut self.pool,
            );
        }
        self.pool.clean();
        self.write_frame(save_path);

        #[cfg(feature = "profiling")]
        profiling::finish_frame!();
    }
    fn write_frame(&mut self, save_path: Option<impl AsRef<Path>>) {
        // `output_video` is true
        if let Some(video_writer) = self.video_writer.as_mut() {
            video_writer.write_frame(self.renderer.get_rendered_texture_data(&self.ctx));
        } else if let Some(builder) = self.video_writer_builder.as_ref() {
            self.video_writer
                .get_or_insert(builder.clone().build())
                .write_frame(self.renderer.get_rendered_texture_data(&self.ctx));
        }

        if let Some(save_path) = save_path {
            self.save_frame_to_image(save_path);
        }
        self.frame_count += 1;
    }

    pub fn save_frame_to_image(&mut self, path: impl AsRef<Path>) {
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
        let buffer = self.renderer.get_rendered_texture_img_buffer(&self.ctx);
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
    fn new(scene_name: String, scene_config: &SceneConfig, output: &Output) -> Self {
        let render_worker = RenderWorker::new(scene_name, scene_config, output);
        Self {
            render_worker: Some(render_worker),
            fps: output.fps,
            store: CoreItemStore::default(),
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

        let worker_thread = self.render_worker.take().unwrap().yeet();

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

        (0..frames)
            .map(|f| f as f64 / (frames - 1) as f64)
            .for_each(|alpha| {
                worker_thread.sync_and_submit(|store| {
                    store.update(timeline.eval_at_alpha(alpha));
                });

                pb.inc(1);
                pb.set_message(format!(
                    "rendering {:.1?}/{:.1?}",
                    Duration::from_secs_f64(alpha * timeline.total_secs()),
                    Duration::from_secs_f64(timeline.total_secs())
                ));
            });
        self.render_worker.replace(worker_thread.retrive());

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
            let alpha = *sec / timeline.total_secs();

            self.store.update(timeline.eval_at_alpha(alpha));
            self.render_worker
                .as_mut()
                .unwrap()
                .render_store(&self.store, Some(filename));
            pb.inc(1);
        }
        pb.finish_with_message(format!(
            "saved {} capture frames from time marks",
            timemarks.len()
        ));
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
