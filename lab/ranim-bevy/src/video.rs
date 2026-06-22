//! Headless video capture helpers for Bevy-hosted Ranim scenes.
//!
//! This module intentionally mirrors Ranim's output settings without making
//! `ranim-bevy` depend on the top-level `ranim` crate.

use std::{
    collections::BTreeMap,
    io::Write,
    path::{Path, PathBuf},
    process::{Child, ChildStdin, Command, Stdio},
    str::FromStr,
    time::Duration,
};

use bevy::{
    app::{AppExit, ScheduleRunnerPlugin},
    asset::RenderAssetUsages,
    camera::{RenderTarget, ShadowLodOrigin, Viewport},
    ecs::schedule::SystemSet,
    prelude::*,
    render::{
        render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages},
        view::screenshot::{Screenshot, ScreenshotCaptured},
    },
    time::TimeUpdateStrategy,
};
use indicatif::{ProgressState, ProgressStyle};
use tracing::{Span, info_span};
use tracing_indicatif::span_ext::IndicatifSpanExt;

/// Video output format.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum BevyOutputFormat {
    /// H.264 in MP4 container.
    #[default]
    Mp4,
    /// VP9 with alpha in WebM container.
    Webm,
    /// ProRes 4444 in MOV container.
    Mov,
    /// GIF.
    Gif,
}

impl std::fmt::Display for BevyOutputFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Mp4 => write!(f, "mp4"),
            Self::Webm => write!(f, "webm"),
            Self::Mov => write!(f, "mov"),
            Self::Gif => write!(f, "gif"),
        }
    }
}

impl FromStr for BevyOutputFormat {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.to_ascii_lowercase().as_str() {
            "mp4" => Ok(Self::Mp4),
            "webm" => Ok(Self::Webm),
            "mov" => Ok(Self::Mov),
            "gif" => Ok(Self::Gif),
            other => Err(format!("unsupported video format `{other}`")),
        }
    }
}

/// Bevy-side port of Ranim's video output settings.
#[derive(Resource, Debug, Clone)]
pub struct BevyOutput {
    /// The width of the output texture in pixels.
    pub width: u32,
    /// The height of the output texture in pixels.
    pub height: u32,
    /// The frame rate of the output video.
    pub fps: u32,
    /// Whether to save individual frames next to the video.
    pub save_frames: bool,
    /// The name of the video.
    pub name: Option<String>,
    /// The directory to save the output.
    pub dir: String,
    /// The output video format.
    pub format: BevyOutputFormat,
}

impl Default for BevyOutput {
    fn default() -> Self {
        Self {
            width: 1920,
            height: 1080,
            fps: 60,
            save_frames: false,
            name: None,
            dir: "./output".to_string(),
            format: BevyOutputFormat::default(),
        }
    }
}

impl BevyOutput {
    /// Resolve the output directory relative to the current working directory.
    pub fn output_dir(&self) -> PathBuf {
        let dir = PathBuf::from(&self.dir);
        if dir.is_absolute() {
            dir
        } else {
            std::env::current_dir().unwrap().join(dir)
        }
    }

    /// Build the conventional Ranim output file path.
    pub fn file_path(&self, scene_name: impl AsRef<str>) -> PathBuf {
        let (_, _, ext) = self.format.encoding_params();
        self.output_dir().join(format!(
            "{}_{}x{}_{}.{}",
            self.name.as_deref().unwrap_or(scene_name.as_ref()),
            self.width,
            self.height,
            self.fps,
            ext
        ))
    }

    /// Build the conventional Ranim frame directory path.
    pub fn frame_dir(&self, scene_name: impl AsRef<str>) -> PathBuf {
        self.output_dir().join(format!(
            "{}_{}x{}_{}-frames",
            self.name.as_deref().unwrap_or(scene_name.as_ref()),
            self.width,
            self.height,
            self.fps
        ))
    }

    /// Override output settings from environment variables using `prefix`.
    ///
    /// With the prefix `RANIM_BEVY_VIDEO_`, this reads `WIDTH`, `HEIGHT`, `FPS`,
    /// `SAVE_FRAMES`, `NAME`, `DIR`, and `FORMAT`.
    pub fn apply_env_overrides(&mut self, prefix: &str) {
        if let Some(width) = env_u32(prefix, "WIDTH") {
            self.width = width;
        }
        if let Some(height) = env_u32(prefix, "HEIGHT") {
            self.height = height;
        }
        if let Some(fps) = env_u32(prefix, "FPS") {
            self.fps = fps;
        }
        if let Some(save_frames) = env_bool(prefix, "SAVE_FRAMES") {
            self.save_frames = save_frames;
        }
        if let Some(name) = env_string(prefix, "NAME") {
            self.name = Some(name);
        }
        if let Some(dir) = env_string(prefix, "DIR") {
            self.dir = dir;
        }
        if let Some(format) = env_string(prefix, "FORMAT") {
            self.format = format.parse().unwrap_or_else(|err| panic!("{err}"));
        }
    }
}

/// Configuration for headless video export.
#[derive(Resource, Debug, Clone)]
pub struct VideoExportConfig {
    /// Human-readable scene name used for output file naming.
    pub scene_name: String,
    /// Video output settings.
    pub output: BevyOutput,
    /// Total export duration.
    pub duration: Duration,
    /// Whether to stop the Bevy app when the export finishes.
    pub exit_on_finish: bool,
}

impl VideoExportConfig {
    /// Create a config with Ranim-compatible default output settings.
    pub fn new(scene_name: impl Into<String>, duration: Duration) -> Self {
        Self {
            scene_name: scene_name.into(),
            output: BevyOutput::default(),
            duration,
            exit_on_finish: true,
        }
    }

    /// Set output settings.
    pub fn with_output(mut self, output: BevyOutput) -> Self {
        self.output = output;
        self
    }

    /// Set whether the app exits automatically once encoding completes.
    pub fn with_exit_on_finish(mut self, exit_on_finish: bool) -> Self {
        self.exit_on_finish = exit_on_finish;
        self
    }

    /// Duration of a single exported frame.
    pub fn frame_step(&self) -> Duration {
        Duration::from_secs_f64(1.0 / self.output.fps as f64)
    }

    /// Total number of frames to capture.
    pub fn total_frames(&self) -> u32 {
        let exact = self.duration.as_secs_f64() * self.output.fps as f64;
        let rounded = exact.round();
        if (exact - rounded).abs() < 1e-6 {
            rounded as u32
        } else {
            exact.ceil() as u32
        }
    }

    /// Override export settings from environment variables using `prefix`.
    ///
    /// In addition to [`BevyOutput::apply_env_overrides`], this reads `DURATION`
    /// in seconds.
    pub fn apply_env_overrides(&mut self, prefix: &str) {
        self.output.apply_env_overrides(prefix);
        if let Some(duration) = env_f32(prefix, "DURATION") {
            self.duration = Duration::from_secs_f32(duration);
        }
    }
}

/// Headless renderer and ffmpeg encoder for existing Bevy scenes.
///
/// Add this after the scene has registered its normal camera setup systems. The plugin retargets
/// every active camera to an offscreen image in [`PostStartup`], captures that image every update,
/// and streams frames to ffmpeg.
#[derive(Debug, Clone)]
pub struct VideoExportPlugin {
    config: VideoExportConfig,
}

/// System sets used by [`VideoExportPlugin`].
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum VideoExportSystems {
    /// Requests a readback for the current frame.
    RequestFrame,
    /// Exits the app once all requested frames have been written.
    ExitWhenFinished,
}

impl VideoExportPlugin {
    /// Create a plugin from explicit export config.
    pub fn new(config: VideoExportConfig) -> Self {
        Self { config }
    }

    /// Create a plugin with default output settings and a duration in seconds.
    pub fn for_seconds(scene_name: impl Into<String>, duration_secs: f32) -> Self {
        Self::new(VideoExportConfig::new(
            scene_name,
            Duration::from_secs_f32(duration_secs),
        ))
    }
}

impl Plugin for VideoExportPlugin {
    fn build(&self, app: &mut App) {
        let frame_step = self.config.frame_step();

        app.insert_resource(self.config.output.clone())
            .insert_resource(self.config.clone())
            .insert_resource(TimeUpdateStrategy::ManualDuration(frame_step))
            .add_plugins(ScheduleRunnerPlugin::run_loop(frame_step))
            .add_systems(Startup, setup_capture_target)
            .add_systems(
                PostStartup,
                (retarget_cameras, setup_video_writer).chain(),
            )
            .configure_sets(
                Last,
                (
                    VideoExportSystems::RequestFrame,
                    VideoExportSystems::ExitWhenFinished,
                )
                    .chain(),
            )
            .add_systems(
                Last,
                (
                    request_frame_capture.in_set(VideoExportSystems::RequestFrame),
                    exit_when_finished.in_set(VideoExportSystems::ExitWhenFinished),
                ),
            );
    }
}

#[derive(Resource, Clone)]
struct CaptureTarget {
    image: Handle<Image>,
}

#[derive(Resource)]
struct CaptureState {
    requested_frames: u32,
    next_frame_to_write: u32,
    total_frames: u32,
    scene_name: String,
    output: BevyOutput,
    pending_frames: BTreeMap<u32, Vec<u8>>,
    video_writer: Option<FfmpegVideoWriter>,
    progress_span: Span,
    exit_on_finish: bool,
}

/// Extension trait providing ffmpeg encoding parameters for [`BevyOutputFormat`].
pub trait BevyOutputFormatExt {
    /// Returns `(video_codec, pixel_format, file_extension)`.
    fn encoding_params(&self) -> (&'static str, &'static str, &'static str);
    /// Returns extra codec arguments for ffmpeg.
    fn extra_args(&self) -> &'static [&'static str];
    /// Whether this format has an alpha channel.
    fn has_alpha(&self) -> bool;
    /// Whether the `eq` video filter is compatible with this format.
    fn supports_eq_filter(&self) -> bool;
}

impl BevyOutputFormatExt for BevyOutputFormat {
    fn encoding_params(&self) -> (&'static str, &'static str, &'static str) {
        match self {
            Self::Mp4 => ("libx264", "yuv420p", "mp4"),
            Self::Webm => ("libvpx-vp9", "yuva420p", "webm"),
            Self::Mov => ("prores_ks", "yuva444p10le", "mov"),
            Self::Gif => ("gif", "rgb8", "gif"),
        }
    }

    fn extra_args(&self) -> &'static [&'static str] {
        match self {
            Self::Mov => &["-profile:v", "4444"],
            _ => &[],
        }
    }

    fn has_alpha(&self) -> bool {
        matches!(self, Self::Webm | Self::Mov)
    }

    fn supports_eq_filter(&self) -> bool {
        !self.has_alpha()
    }
}

/// Builder for an ffmpeg raw RGBA writer.
#[derive(Debug, Clone)]
pub struct FfmpegVideoWriterBuilder {
    /// Output file path.
    pub file_path: PathBuf,
    /// Output width.
    pub width: u32,
    /// Output height.
    pub height: u32,
    /// Output fps.
    pub fps: u32,
    /// ffmpeg video filters.
    pub vf_args: Vec<String>,
    /// ffmpeg video codec.
    pub video_codec: String,
    /// ffmpeg pixel format.
    pub pixel_format: String,
    /// Extra codec args.
    pub extra_codec_args: Vec<String>,
}

impl Default for FfmpegVideoWriterBuilder {
    fn default() -> Self {
        Self {
            file_path: PathBuf::from("output.mp4"),
            width: 1920,
            height: 1080,
            fps: 60,
            vf_args: vec!["eq=saturation=1.0:gamma=1.0".to_string()],
            video_codec: "libx264".to_string(),
            pixel_format: "yuv420p".to_string(),
            extra_codec_args: Vec::new(),
        }
    }
}

impl FfmpegVideoWriterBuilder {
    /// Create a writer builder from Bevy/Ranim-style output settings.
    pub fn from_output(scene_name: impl AsRef<str>, output: &BevyOutput) -> Self {
        Self::default()
            .with_fps(output.fps)
            .with_size(output.width, output.height)
            .with_file_path(output.file_path(scene_name))
            .with_output_format(output.format)
    }

    /// Set output file path.
    pub fn with_file_path(mut self, file_path: PathBuf) -> Self {
        self.file_path = file_path;
        self
    }

    /// Set output size.
    pub fn with_size(mut self, width: u32, height: u32) -> Self {
        self.width = width;
        self.height = height;
        self
    }

    /// Set output fps.
    pub fn with_fps(mut self, fps: u32) -> Self {
        self.fps = fps;
        self
    }

    /// Set output format and matching ffmpeg parameters.
    pub fn with_output_format(mut self, format: BevyOutputFormat) -> Self {
        let (codec, pix_fmt, ext) = format.encoding_params();
        self.video_codec = codec.to_string();
        self.pixel_format = pix_fmt.to_string();
        self.extra_codec_args = format.extra_args().iter().map(|s| s.to_string()).collect();
        self.file_path = self.file_path.with_extension(ext);
        if !format.supports_eq_filter() {
            self.vf_args.clear();
        }
        if format == BevyOutputFormat::Gif && self.fps > 50 {
            self.fps = 50;
        }
        self
    }

    /// Build the ffmpeg process.
    pub fn build(self) -> FfmpegVideoWriter {
        if let Some(parent) = self.file_path.parent()
            && !parent.exists()
        {
            std::fs::create_dir_all(parent).unwrap();
        }

        let mut command = if let Ok(ffmpeg_path) = which::which("ffmpeg") {
            Command::new(ffmpeg_path)
        } else if Path::new("./ffmpeg").exists() {
            Command::new("./ffmpeg")
        } else {
            panic!("ffmpeg not found in PATH or current working directory");
        };

        let size = format!("{}x{}", self.width, self.height);
        let fps = self.fps.to_string();
        let file_path = self.file_path.to_string_lossy().to_string();

        command.args([
            "-y", "-f", "rawvideo", "-s", &size, "-pix_fmt", "rgba", "-r", &fps, "-i", "-",
        ]);
        command.args(["-an", "-loglevel", "error", "-vcodec", &self.video_codec]);
        command.args(&self.extra_codec_args);
        command.args(["-pix_fmt", &self.pixel_format]);
        if !self.vf_args.is_empty() {
            let vf = self.vf_args.join(",");
            command.args(["-vf", &vf]);
        }
        command.arg(&file_path);
        command.stdin(Stdio::piped());

        let mut child = command.spawn().expect("Failed to spawn ffmpeg");
        FfmpegVideoWriter {
            child_in: child.stdin.take(),
            child,
        }
    }
}

/// A streaming ffmpeg writer that accepts RGBA frames.
pub struct FfmpegVideoWriter {
    child: Child,
    child_in: Option<ChildStdin>,
}

impl Drop for FfmpegVideoWriter {
    fn drop(&mut self) {
        if let Some(child_in) = self.child_in.as_mut() {
            child_in.flush().expect("Failed to flush ffmpeg");
        }
        drop(self.child_in.take());
        self.child.wait().expect("Failed to wait ffmpeg");
    }
}

impl FfmpegVideoWriter {
    /// Write a single RGBA frame.
    pub fn write_frame(&mut self, frame: &[u8]) {
        self.child_in
            .as_mut()
            .unwrap()
            .write_all(frame)
            .expect("Failed to write frame");
    }
}

fn setup_capture_target(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    config: Res<VideoExportConfig>,
) {
    let mut target = Image::new_uninit(
        Extent3d {
            width: config.output.width,
            height: config.output.height,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    target.texture_descriptor.usage |= TextureUsages::RENDER_ATTACHMENT;

    commands.insert_resource(CaptureTarget {
        image: images.add(target),
    });
}

fn retarget_cameras(
    mut commands: Commands,
    capture_target: Res<CaptureTarget>,
    config: Res<VideoExportConfig>,
    mut cameras: Query<(Entity, &mut Camera, &mut RenderTarget, Has<Camera3d>)>,
) {
    let mut count = 0;
    for (entity, mut camera, mut target, is_camera_3d) in &mut cameras {
        if !camera.is_active {
            continue;
        }

        *target = RenderTarget::Image(capture_target.image.clone().into());
        camera.viewport = Some(Viewport {
            physical_position: UVec2::ZERO,
            physical_size: UVec2::new(config.output.width, config.output.height),
            ..default()
        });
        if is_camera_3d {
            commands.entity(entity).insert(ShadowLodOrigin);
        }
        count += 1;
    }

    assert!(
        count > 0,
        "VideoExportPlugin expected at least one active camera to retarget"
    );
}

fn setup_video_writer(mut commands: Commands, config: Res<VideoExportConfig>) {
    let file_path = config.output.file_path(&config.scene_name);
    let total_frames = config.total_frames();
    info!("writing video to {}", file_path.display());
    let progress_span = info_span!(
        "bevy_video_export",
        scene = %config.scene_name,
        output = %file_path.display()
    );
    let progress_style = ProgressStyle::with_template(
        "[{elapsed_precise}] [{wide_bar:.cyan/blue}] frame {human_pos}/{human_len} (eta {eta}) {msg}",
    )
    .unwrap()
    .with_key("eta", |state: &ProgressState, w: &mut dyn std::fmt::Write| {
        write!(w, "{:.1}s", state.eta().as_secs_f64()).unwrap()
    })
    .progress_chars("#>-");
    progress_span.pb_set_style(&progress_style);
    progress_span.pb_set_length(total_frames as u64);
    progress_span.pb_set_message(file_path.to_string_lossy().as_ref());
    progress_span.pb_start();

    commands.insert_resource(CaptureState {
        requested_frames: 0,
        next_frame_to_write: 0,
        total_frames,
        scene_name: config.scene_name.clone(),
        output: config.output.clone(),
        pending_frames: BTreeMap::new(),
        video_writer: Some(
            FfmpegVideoWriterBuilder::from_output(&config.scene_name, &config.output).build(),
        ),
        progress_span,
        exit_on_finish: config.exit_on_finish,
    });
}

fn request_frame_capture(
    mut commands: Commands,
    capture_target: Res<CaptureTarget>,
    mut capture: ResMut<CaptureState>,
) {
    if capture.requested_frames >= capture.total_frames {
        return;
    }

    let frame = capture.requested_frames;
    capture.requested_frames += 1;

    commands
        .spawn(Screenshot::image(capture_target.image.clone()))
        .observe(move |captured: On<ScreenshotCaptured>, mut state: ResMut<CaptureState>| {
            let rgba = screenshot_to_rgba(&captured.image);

            if state.output.save_frames {
                let dir = state.output.frame_dir(&state.scene_name);
                std::fs::create_dir_all(&dir).unwrap();
                let path = dir.join(format!("{frame:04}.png"));
                captured
                    .image
                    .clone()
                    .try_into_dynamic()
                    .unwrap()
                    .to_rgba8()
                    .save(path)
                    .unwrap();
            }

            state.pending_frames.insert(frame, rgba);
            flush_ready_frames(&mut state);
        });
}

fn flush_ready_frames(state: &mut CaptureState) {
    while let Some(frame) = state.pending_frames.remove(&state.next_frame_to_write) {
        state.video_writer.as_mut().unwrap().write_frame(&frame);
        state.next_frame_to_write += 1;
        state.progress_span.pb_inc(1);
        state.progress_span.pb_set_message(
            format!(
                "encoding {:.1?}/{:.1?}",
                Duration::from_secs_f64(state.next_frame_to_write as f64 / state.output.fps as f64),
                Duration::from_secs_f64(state.total_frames as f64 / state.output.fps as f64),
            )
            .as_str(),
        );
    }
}

fn exit_when_finished(mut capture: ResMut<CaptureState>, mut exit: MessageWriter<AppExit>) {
    if capture.next_frame_to_write < capture.total_frames {
        return;
    }

    drop(capture.video_writer.take());
    capture.progress_span.pb_set_finish_message(
        format!(
            "wrote {} frames to {}",
            capture.total_frames,
            capture.output.file_path(&capture.scene_name).display()
        )
        .as_str(),
    );
    if capture.exit_on_finish {
        exit.write(AppExit::Success);
    }
}

/// Convert a captured Bevy screenshot image into tightly packed RGBA bytes.
pub fn screenshot_to_rgba(image: &Image) -> Vec<u8> {
    let dynamic = image
        .clone()
        .try_into_dynamic()
        .expect("Failed to convert captured screenshot image");
    dynamic.to_rgba8().into_raw()
}

fn env_string(prefix: &str, key: &str) -> Option<String> {
    let value = std::env::var(format!("{prefix}{key}")).ok()?;
    (!value.is_empty()).then_some(value)
}

fn env_u32(prefix: &str, key: &str) -> Option<u32> {
    env_string(prefix, key)?.parse().ok()
}

fn env_f32(prefix: &str, key: &str) -> Option<f32> {
    env_string(prefix, key)?.parse().ok()
}

fn env_bool(prefix: &str, key: &str) -> Option<bool> {
    match env_string(prefix, key)?.to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => Some(true),
        "0" | "false" | "no" | "off" => Some(false),
        _ => None,
    }
}
