//! Audio decoding and mixing helpers for the render pipeline.

use std::{
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use ranim_core::audio::{PcmBuffer, Sound, StereoFrame};

/// Decode any ffmpeg-readable audio file (mp3, flac, ogg, ...) into a
/// [`Sound`] leaf backed by a [`PcmBuffer`].
///
/// Shells out to the same ffmpeg binary the video writer already requires,
/// normalizing to 48 kHz stereo `f32` on the way through, so no decoding
/// crates are added to the dependency tree.
pub struct FfmpegSource;

impl FfmpegSource {
    /// Decode `path` into a sound leaf.
    ///
    /// # Panics
    ///
    /// Panics if ffmpeg cannot be found or fails to decode the file.
    pub fn from_path(path: impl AsRef<Path>) -> anyhow::Result<Sound<PcmBuffer>> {
        let path: PathBuf = path.as_ref().to_path_buf();
        let output = ffmpeg_command()
            .arg("-i")
            .arg(&path)
            .args(["-vn", "-ac", "2", "-ar", "48000", "-f", "f32le", "pipe:1"])
            .stderr(Stdio::null())
            .output()?;
        if !output.status.success() {
            anyhow::bail!("ffmpeg failed to decode {}", path.display());
        }
        let bytes = output.stdout;
        let frames = bytes
            .as_chunks::<8>()
            .0
            .iter()
            .map(|c| StereoFrame {
                l: f32::from_le_bytes(c[0..4].try_into().unwrap()),
                r: f32::from_le_bytes(c[4..8].try_into().unwrap()),
            })
            .collect::<Vec<_>>();
        let secs = frames.len() as f64 / 48000.0;
        Ok(Sound::new(PcmBuffer::new(frames.into(), secs)))
    }
}

/// Resolve an ffmpeg command, preferring the one on `PATH`, falling back to
/// `./ffmpeg` (the location the renderer auto-downloads to).
pub(crate) fn ffmpeg_command() -> Command {
    if which::which("ffmpeg").is_ok() {
        Command::new("ffmpeg")
    } else {
        Command::new("./ffmpeg")
    }
}
