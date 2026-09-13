//! Audio muxing for rendered outputs.
//!
//! The scene's audio plane is mixed in one offline pass, written to a temporary
//! WAV, and merged into the finished video by a second `ffmpeg` pass (video
//! stream copied, audio encoded per container format).

use std::path::Path;

use anyhow::{Context, anyhow, bail};
use ranim_core::audio::{MASTER_CHANNELS, MASTER_SAMPLE_RATE, write_wav};
use tracing::info;

use crate::OutputFormat;

/// The audio codec for a container format; `None` where audio cannot be
/// stored (GIF).
pub(crate) fn audio_codec(format: OutputFormat) -> Option<&'static str> {
    match format {
        OutputFormat::Mp4 | OutputFormat::Mov => Some("aac"),
        OutputFormat::Webm => Some("libopus"),
        OutputFormat::Gif => None,
    }
}

/// Mix and mux `pcm` into the finished video at `video_path`.
///
/// The WAV is written next to the video, merged with the video stream copied
/// (no re-encode), and removed afterwards; the muxed result replaces the
/// original file atomically via rename.
pub(crate) fn mux_audio_into_video(
    video_path: &Path,
    pcm: &[f32],
    format: OutputFormat,
) -> anyhow::Result<()> {
    let Some(codec) = audio_codec(format) else {
        bail!("format {format:?} cannot store audio");
    };
    let wav_path = video_path.with_extension(format!("ranim-audio-{}.wav", std::process::id()));
    write_wav(&wav_path, pcm, MASTER_SAMPLE_RATE, MASTER_CHANNELS)
        .with_context(|| format!("failed to write {}", wav_path.display()))?;

    let muxed_path = video_path.with_extension(format!(
        "ranim-muxed-{}.{}",
        std::process::id(),
        video_path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("mp4"),
    ));

    let ffmpeg = ["ffmpeg", "./ffmpeg"]
        .into_iter()
        .find(|bin| Path::new(bin).exists() || which::which(bin).is_ok())
        .ok_or_else(|| anyhow!("ffmpeg not found"))?;
    let status = std::process::Command::new(ffmpeg)
        .args(["-y", "-v", "error", "-i"])
        .arg(video_path)
        .args(["-i"])
        .arg(&wav_path)
        .args([
            "-map",
            "0:v:0",
            "-map",
            "1:a:0",
            "-c:v",
            "copy",
            "-c:a",
            codec,
            "-shortest",
        ])
        .arg(&muxed_path)
        .status()
        .context("failed to spawn ffmpeg for audio muxing")?;

    let _ = std::fs::remove_file(&wav_path);
    if !status.success() {
        let _ = std::fs::remove_file(&muxed_path);
        bail!("ffmpeg audio muxing failed with {status}");
    }
    std::fs::rename(&muxed_path, video_path).with_context(|| {
        format!(
            "failed to replace {} with muxed output",
            video_path.display()
        )
    })?;
    info!("muxed audio into {}", video_path.display());
    Ok(())
}
