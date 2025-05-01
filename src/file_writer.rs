use std::{
    io::Write,
    path::PathBuf,
    process::{Child, ChildStdin, Command, Stdio},
};

#[cfg(feature = "video-rs")]
use ndarray;
#[cfg(feature = "video-rs")]
use ndarray::Array3;
use ndarray::{Array4, s};
#[cfg(feature = "video-rs")]
use video_rs::Encoder;
#[cfg(feature = "video-rs")]
use video_rs::{Time, encode::Settings};

pub trait VideoWriter {
    fn write_frame(&mut self, frame: &[u8]);
}

#[cfg(feature = "video-rs")]
pub struct VideoRsFileWriter {
    width: u32,
    height: u32,
    encoder: Encoder,
    position: Time,
    duration: Time,
}

#[cfg(feature = "video-rs")]
impl VideoRsFileWriter {
    pub fn new(width: u32, height: u32, fps: u32, path: PathBuf) -> Self {
        let settings = Settings::preset_h264_yuv420p(width as usize, height as usize, true);
        let encoder = Encoder::new(path, settings).expect("failed to create encoder");
        Self {
            width,
            height,
            encoder,
            position: Time::zero(),
            duration: Time::from_nth_of_a_second(fps as usize),
        }
    }
}

#[cfg(feature = "video-rs")]
impl VideoWriter for VideoRsFileWriter {
    fn write_frame(&mut self, frame_bytes: &[u8]) {
        let mut frame = Array3::<u8>::zeros((self.height as usize, self.width as usize, 3));

        frame_bytes
            .chunks_exact(4)
            .zip(frame.exact_chunks_mut((1, 1, 3)))
            .for_each(|(src, mut dst)| {
                dst.as_slice_mut().unwrap().copy_from_slice(&src[..3]);
            });
        // let rgba_array = Array3::<u8>::from_shape_vec(
        //     (self.height as usize, self.width as usize, 4),
        //     frame.to_vec(),
        // );
        // let mut rgb_array = Array3::<u8>::zeros((self.height as usize, self.width as usize, 3));
        // // 复制前3个通道
        // rgb_array.assign(&rgba_array.slice(s![.., .., 0..3]));
        self.encoder
            .encode(&frame, self.position)
            .expect("failed to encode frame");

        self.position = self.position.aligned_with(self.duration).add();
    }
}

#[cfg(feature = "video-rs")]
impl Drop for VideoRsFileWriter {
    fn drop(&mut self) {
        self.encoder.finish().expect("failed to finish encoder");
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
        // let tmp_file_path = self.file_path.with_file_name(format!(
        //     "{}_tmp.{}",
        //     self.file_path.file_stem().unwrap().to_string_lossy(),
        //     self.file_path
        //         .extension()
        //         .map(|s| s.to_string_lossy())
        //         .unwrap_or("mp4".into())
        // ));
        let parent = self.file_path.parent().unwrap();
        if !parent.exists() {
            std::fs::create_dir_all(parent).unwrap();
        }

        let mut command = Command::new("ffmpeg");
        #[rustfmt::skip]
        let command = command.args([
            "-y",
            "-f", "rawvideo",
            "-s", format!("{}x{}", self.width, self.height).as_str(),
            "-pix_fmt", "rgba",
            "-r", self.fps.to_string().as_str(),
            "-i", "-",
            "-an",
            "-loglevel", "error",
            "-vcodec", self.video_codec.as_str(),
            "-pix_fmt", self.pixel_format.as_str(),
            &self.file_path.to_string_lossy(),
        ]).stdin(Stdio::piped());
        if !self.vf_args.is_empty() {
            command.args(["-vf", self.vf_args.join(",").as_str()]);
        }

        let mut child = command.spawn().expect("Failed to spawn ffmpeg");
        FileWriter {
            child_in: child.stdin.take(),
            child,
        }
    }
}

pub struct FileWriter {
    child: Child,
    child_in: Option<ChildStdin>,
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
    }
}

impl VideoWriter for FileWriter {
    fn write_frame(&mut self, frame: &[u8]) {
        self.child_in
            .as_mut()
            .unwrap()
            .write_all(frame)
            .expect("Failed to write frame");
    }
}
