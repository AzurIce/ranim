//! Ready-made [`SoundSource`](ranim_core::audio::SoundSource) implementations
//! available without extra features.
//!
//! Decoding here is std-only (wav). For other containers (mp3, flac, ogg,
//! ...), see [`crate::cmd::render::audio::FfmpegSource`], which shells out to
//! the same ffmpeg binary the renderer already depends on.

use std::path::{Path, PathBuf};

use ranim_core::audio::{PcmBuffer, Sound, StereoFrame};

/// Decode a wav file into a [`Sound`] leaf backed by a [`PcmBuffer`].
///
/// Supports uncompressed PCM wav (integer 16/24/32 bit and 32-bit float), mono
/// or stereo, at any sample rate. Playback rate is self-correcting: the leaf
/// contract is normalized progress, and real-world duration comes from the
/// file header, so a 44.1 kHz file plays at 1x speed on the 48 kHz master
/// grid without explicit resampling.
///
/// # Example
///
/// ```no_run
/// use ranim::prelude::*;
/// use ranim::WavSource;
///
/// # fn main() -> std::io::Result<()> {
/// let pop = WavSource::from_path("assets/pop.wav")?;
/// # let mut scene = RanimScene::new();
/// scene.play(pop.at(1.0));
/// # Ok(())
/// # }
/// ```
pub struct WavSource;

impl WavSource {
    /// Decode `path` into a sound leaf.
    pub fn from_path(path: impl AsRef<Path>) -> std::io::Result<Sound<PcmBuffer>> {
        let path: PathBuf = path.as_ref().to_path_buf();
        let bytes = std::fs::read(&path)?;
        let (frames, secs) = decode_wav(&bytes)
            .map_err(|e| std::io::Error::new(e.kind(), format!("{}: {e}", path.display())))?;
        Ok(Sound::new(PcmBuffer::new(frames.into(), secs)))
    }
}

/// Minimal RIFF/wav parse errors (kept as an enum for `kind()` mapping).
#[derive(Debug)]
enum WavError {
    NotWav,
    UnsupportedFormat(u16, u16),
    Truncated,
}

impl WavError {
    fn kind(&self) -> std::io::ErrorKind {
        match self {
            WavError::NotWav | WavError::Truncated => std::io::ErrorKind::InvalidData,
            WavError::UnsupportedFormat(..) => std::io::ErrorKind::Unsupported,
        }
    }
}

impl std::fmt::Display for WavError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WavError::NotWav => write!(f, "not a RIFF/wav file"),
            WavError::UnsupportedFormat(format, bits) => {
                write!(f, "unsupported wav format {format} with {bits} bits")
            }
            WavError::Truncated => write!(f, "wav data chunk is truncated"),
        }
    }
}

type FramesAndSecs = (Vec<StereoFrame>, f64);

fn decode_wav(bytes: &[u8]) -> Result<FramesAndSecs, WavError> {
    if bytes.len() < 12 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return Err(WavError::NotWav);
    }
    let mut fmt: Option<(u16, u16, u32, u16)> = None; // (format, channels, rate, bits)
    let mut data: &[u8] = &[];
    let mut pos = 12;
    while pos + 8 <= bytes.len() {
        let tag = &bytes[pos..pos + 4];
        let size = u32::from_le_bytes(bytes[pos + 4..pos + 8].try_into().unwrap()) as usize;
        let body_end = (pos + 8 + size).min(bytes.len());
        match tag {
            b"fmt " => {
                let b = &bytes[pos + 8..body_end];
                if b.len() < 16 {
                    return Err(WavError::Truncated);
                }
                fmt = Some((
                    u16::from_le_bytes(b[0..2].try_into().unwrap()),
                    u16::from_le_bytes(b[2..4].try_into().unwrap()),
                    u32::from_le_bytes(b[4..8].try_into().unwrap()),
                    u16::from_le_bytes(b[14..16].try_into().unwrap()),
                ));
            }
            b"data" => data = &bytes[pos + 8..body_end],
            _ => {}
        }
        pos += 8 + size + (size & 1); // chunks are word-aligned
    }
    let (format, channels, rate, bits) = fmt.ok_or(WavError::Truncated)?;
    if !matches!(channels, 1 | 2) {
        return Err(WavError::UnsupportedFormat(format, channels));
    }
    let sample = match (format, bits) {
        (1, 16) => |b: &[u8]| f32::from(i16::from_le_bytes(b.try_into().unwrap())) / 32768.0,
        (1, 24) => |b: &[u8]| {
            let v = (b[0] as i32) | ((b[1] as i32) << 8) | ((b[2] as i8 as i32) << 16);
            v as f32 / 8388608.0
        },
        (1, 32) => |b: &[u8]| i32::from_le_bytes(b.try_into().unwrap()) as f32 / 2147483648.0,
        (3, 32) => |b: &[u8]| f32::from_le_bytes(b.try_into().unwrap()),
        _ => return Err(WavError::UnsupportedFormat(format, bits)),
    };
    let width = (bits / 8) as usize * channels as usize;
    if data.len() < width || !data.len().is_multiple_of(width) {
        return Err(WavError::Truncated);
    }
    let frames = data
        .chunks_exact(width)
        .map(|chunk| match channels {
            1 => StereoFrame::splat(sample(&chunk[..bits as usize / 8])),
            _ => StereoFrame {
                l: sample(&chunk[..bits as usize / 8]),
                r: sample(&chunk[bits as usize / 8..]),
            },
        })
        .collect::<Vec<_>>();
    let secs = if rate > 0 {
        frames.len() as f64 / rate as f64
    } else {
        0.0
    };
    Ok((frames, secs))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a minimal 16-bit PCM wav in memory.
    fn wav_bytes(samples: &[i16], rate: u32, channels: u16) -> Vec<u8> {
        let bits = 16u16;
        let byte_rate = rate * channels as u32 * 2;
        let block_align = channels * 2;
        let data_len = samples.len() * 2;
        let mut out = Vec::new();
        out.extend_from_slice(b"RIFF");
        out.extend_from_slice(&(36 + data_len as u32).to_le_bytes());
        out.extend_from_slice(b"WAVE");
        out.extend_from_slice(b"fmt ");
        out.extend_from_slice(&16u32.to_le_bytes());
        out.extend_from_slice(&1u16.to_le_bytes());
        out.extend_from_slice(&channels.to_le_bytes());
        out.extend_from_slice(&rate.to_le_bytes());
        out.extend_from_slice(&byte_rate.to_le_bytes());
        out.extend_from_slice(&block_align.to_le_bytes());
        out.extend_from_slice(&bits.to_le_bytes());
        out.extend_from_slice(b"data");
        out.extend_from_slice(&(data_len as u32).to_le_bytes());
        for s in samples {
            out.extend_from_slice(&s.to_le_bytes());
        }
        out
    }

    #[test]
    fn decodes_mono_16bit() {
        let bytes = wav_bytes(&[-32768, 0, 16384], 8000, 1);
        let (frames, secs) = decode_wav(&bytes).unwrap();
        assert_eq!(frames.len(), 3);
        assert!((secs - 3.0 / 8000.0).abs() < 1e-12);
        assert!(frames[0].l < -0.99 && (frames[1].l - 0.0).abs() < 1e-6);
        assert!((frames[2].l - 0.5).abs() < 1e-3);
        // Mono sources feed both channels.
        assert!((frames[0].l - frames[0].r).abs() < 1e-6);
    }

    #[test]
    fn decodes_stereo_16bit() {
        let bytes = wav_bytes(&[-16384, 16384, 0, 0], 48000, 2);
        let (frames, _) = decode_wav(&bytes).unwrap();
        assert_eq!(frames.len(), 2);
        assert!(frames[0].l < -0.49 && frames[0].r > 0.49);
    }

    #[test]
    fn rejects_non_wav() {
        assert!(matches!(decode_wav(b"not a wav"), Err(WavError::NotWav)));
    }
}
