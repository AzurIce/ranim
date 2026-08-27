use std::{
    io::Write,
    path::PathBuf,
    process::{Child, ChildStdin, Command, Stdio},
};

use ranim_core::audio::StereoFrame;

use crate::OutputFormat;
use tracing::info;

/// Extension trait providing ffmpeg encoding parameters for [`OutputFormat`].
pub(crate) trait OutputFormatExt {
    /// Returns `(video_codec, pixel_format, file_extension)`.
    fn encoding_params(&self) -> (&'static str, &'static str, &'static str);
    /// Returns extra codec arguments for ffmpeg.
    fn extra_args(&self) -> &'static [&'static str];
    /// Whether this format has an alpha channel.
    fn has_alpha(&self) -> bool;
    /// Whether the `eq` video filter is compatible with this format.
    fn supports_eq_filter(&self) -> bool;
}

impl OutputFormatExt for OutputFormat {
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

/// A mixed master audio track to mux into the output container.
#[derive(Debug, Clone)]
pub struct AudioTrack {
    /// Interleaved stereo frames at 48 kHz, exactly the master length.
    pub pcm: Vec<StereoFrame>,
}

/// The audio codec used per container format: `(codec, bitrate_args)`.
fn audio_codec(format: OutputFormat) -> Option<(&'static str, &'static [&'static str])> {
    match format {
        OutputFormat::Mp4 => Some(("aac", &["-b:a", "192k"])),
        OutputFormat::Webm => Some(("libopus", &["-b:a", "160k"])),
        OutputFormat::Mov => Some(("pcm_s16le", &[])),
        // Gif containers cannot carry audio.
        OutputFormat::Gif => None,
    }
}

#[derive(Debug, Clone)]
pub struct FileWriterBuilder {
    pub file_path: PathBuf,
    pub width: u32,
    pub height: u32,
    pub fps: u32,
    pub vf_args: Vec<String>,

    pub video_codec: String,
    pub pixel_format: String,
    pub extra_codec_args: Vec<String>,

    /// Output container format; decides audio codec support.
    pub format: OutputFormat,
    /// Master audio track to mux alongside the video, if the scene has sound.
    pub audio: Option<AudioTrack>,
}

impl Default for FileWriterBuilder {
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
            format: OutputFormat::default(),
            audio: None,
        }
    }
}

#[allow(unused)]
impl FileWriterBuilder {
    pub fn with_file_path(mut self, file_path: PathBuf) -> Self {
        self.file_path = file_path;
        self
    }

    pub fn with_size(mut self, width: u32, height: u32) -> Self {
        self.width = width;
        self.height = height;
        self
    }

    pub fn with_fps(mut self, fps: u32) -> Self {
        self.fps = fps;
        self
    }

    pub fn with_output_format(mut self, format: OutputFormat) -> Self {
        let (codec, pix_fmt, ext) = format.encoding_params();
        self.format = format;
        self.video_codec = codec.to_string();
        self.pixel_format = pix_fmt.to_string();
        self.extra_codec_args = format.extra_args().iter().map(|s| s.to_string()).collect();
        // Update file extension to match the format
        self.file_path = self.file_path.with_extension(ext);
        // The eq filter doesn't support alpha pixel formats
        if !format.supports_eq_filter() {
            self.vf_args.clear();
        }
        // GIF timing uses centiseconds (10ms units), so fps above 50
        // gets rounded and causes incorrect playback speed.
        if format == OutputFormat::Gif && self.fps > 50 {
            self.fps = 50;
        }
        self
    }

    pub fn enable_fast_encoding(mut self) -> Self {
        self.video_codec = "libx264rgb".to_string();
        self.pixel_format = "rgb32".to_string();
        self
    }

    pub fn output_gif(mut self) -> Self {
        // TODO: use palette to improve gif quality
        self.file_path = self.file_path.with_file_name(format!(
            "{}.gif",
            self.file_path.file_stem().unwrap().to_string_lossy()
        ));
        self.fps = 30;
        self.video_codec = "gif".to_string();
        self.pixel_format = "rgb8".to_string();
        self
    }

    pub fn build(self) -> FileWriter {
        let parent = self.file_path.parent().unwrap();
        if !parent.exists() {
            std::fs::create_dir_all(parent).unwrap();
        }

        let mut command = if which::which("ffmpeg").is_ok() {
            info!("using ffmpeg found from path env");
            Command::new("ffmpeg")
        } else {
            info!("using ffmpeg from current working dir");
            Command::new("./ffmpeg")
        };

        let size = format!("{}x{}", self.width, self.height);
        let fps = self.fps.to_string();
        let file_path = self.file_path.to_string_lossy().to_string();

        // Input options (video on stdin; the audio track goes through a temp
        // file because std only pipelines one stdin).
        command.args([
            "-y", "-f", "rawvideo", "-s", &size, "-pix_fmt", "rgba", "-r", &fps, "-i", "-",
        ]);
        let mut audio_temp: Option<PathBuf> = None;
        let audio_output_args: Vec<&str>;
        match (&self.audio, audio_codec(self.format)) {
            (Some(track), Some((codec, bitrate))) => {
                let temp = self.file_path.with_extension("pcm");
                let bytes: Vec<u8> = track
                    .pcm
                    .iter()
                    .flat_map(|f| [f.l.to_le_bytes(), f.r.to_le_bytes()])
                    .flatten()
                    .collect();
                if let Err(e) = std::fs::write(&temp, &bytes) {
                    panic!("failed to write audio temp file {}: {e}", temp.display());
                }
                info!(
                    "muxing audio track ({} frames) from {:?}",
                    track.pcm.len(),
                    temp
                );
                command.args([
                    "-f",
                    "f32le",
                    "-ar",
                    "48000",
                    "-ac",
                    "2",
                    "-i",
                    temp.to_string_lossy().as_ref(),
                ]);
                let mut args = vec!["-map", "0:v", "-map", "1:a", "-c:a", codec];
                args.extend_from_slice(bitrate);
                audio_output_args = args;
                audio_temp = Some(temp);
            }
            (Some(_), None) => {
                info!(
                    "format {} carries no audio; dropping the audio track",
                    self.format
                );
                audio_output_args = vec!["-an"];
            }
            (None, _) => audio_output_args = vec!["-an"],
        }

        // Output options (before output file)
        command.args(["-loglevel", "error", "-vcodec", &self.video_codec]);
        command.args(&self.extra_codec_args);
        command.args(["-pix_fmt", &self.pixel_format]);
        command.args(&audio_output_args);
        if !self.vf_args.is_empty() {
            let vf = self.vf_args.join(",");
            command.args(["-vf", &vf]);
        }
        // Output file must be last
        command.arg(&file_path);
        command.stdin(Stdio::piped());

        let mut child = command.spawn().expect("Failed to spawn ffmpeg");
        FileWriter {
            child_in: child.stdin.take(),
            child,
            audio_temp,
        }
    }
}

pub struct FileWriter {
    child: Child,
    child_in: Option<ChildStdin>,
    audio_temp: Option<PathBuf>,
}

impl Drop for FileWriter {
    fn drop(&mut self) {
        self.child_in
            .as_mut()
            .unwrap()
            .flush()
            .expect("Failed to flush ffmpeg");
        drop(self.child_in.take());
        self.child.wait().expect("Failed to wait ffmpeg");
        if let Some(temp) = self.audio_temp.take() {
            let _ = std::fs::remove_file(temp);
        }
    }
}

impl FileWriter {
    // pub fn builder() -> FileWriterBuilder {
    //     FileWriterBuilder::default()
    // }

    pub fn write_frame(&mut self, frame: &[u8]) {
        self.child_in
            .as_mut()
            .unwrap()
            .write_all(frame)
            .expect("Failed to write frame");
    }
}
