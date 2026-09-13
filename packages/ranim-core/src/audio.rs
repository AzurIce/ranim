//! Audio model: clips, tracks, and the leaf mixer.
//!
//! Audio never enters the per-frame evaluation pipeline. Tracks are declared
//! next to visual animations and share the same absolute scene seconds, but
//! their content is baked once at seal time —
//! [`RanimScene::seal`](crate::RanimScene::seal) walks the tree and mixes
//! leaf by leaf into one interleaved stereo buffer — instead of per-frame
//! pulls. Consumers (video muxing, preview playback) read the baked buffer;
//! the visual evaluation stack is untouched.

#[cfg(feature = "audio-decode")]
use std::io::Cursor;
use std::ops::Range;
use std::{f64::consts::TAU, fmt, io::Write, path::Path, sync::Arc};
#[cfg(feature = "audio-decode")]
use symphonia::core::{
    codecs::{CODEC_TYPE_NULL, DecoderOptions},
    errors::Error as SymphoniaError,
    formats::FormatOptions,
    io::MediaSourceStream,
    meta::MetadataOptions,
    probe::Hint,
};

/// Master sample rate every mixed buffer lives on.
pub const MASTER_SAMPLE_RATE: u32 = 48_000;

/// Master channel count; mixed buffers are interleaved stereo.
pub const MASTER_CHANNELS: u16 = 2;

/// An error produced while decoding an [`AudioClip`] from a file.
#[derive(Debug)]
pub struct AudioError(String);

impl fmt::Display for AudioError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "audio error: {}", self.0)
    }
}

impl std::error::Error for AudioError {}

impl From<std::io::Error> for AudioError {
    fn from(err: std::io::Error) -> Self {
        Self(err.to_string())
    }
}

/// Decoded audio: interleaved `f32` PCM samples.
///
/// Clips are immutable and cheap to clone (sample data is shared). Decoding a
/// file normalizes to [`MASTER_SAMPLE_RATE`] and [`MASTER_CHANNELS`]; clips
/// built with [`AudioClip::from_pcm`] may carry any rate/channels and the
/// mixer adapts them on the fly.
#[derive(Debug, Clone)]
pub struct AudioClip {
    sample_rate: u32,
    channels: u16,
    pcm: Arc<[f32]>,
}

impl AudioClip {
    /// Build a clip from interleaved PCM samples.
    pub fn from_pcm(pcm: impl Into<Arc<[f32]>>, sample_rate: u32, channels: u16) -> Self {
        assert!(sample_rate > 0, "sample rate must be positive");
        assert!(channels > 0, "channels must be positive");
        Self {
            sample_rate,
            channels,
            pcm: pcm.into(),
        }
    }

    /// A mono sine tone, mainly for tests and self-contained examples.
    pub fn sine(freq: f64, secs: f64, amplitude: f64) -> Self {
        let sample_rate = MASTER_SAMPLE_RATE;
        let len = (secs * sample_rate as f64) as usize;
        let pcm: Vec<f32> = (0..len)
            .map(|i| (amplitude * (TAU * freq * i as f64 / sample_rate as f64).sin()) as f32)
            .collect();
        Self::from_pcm(pcm, sample_rate, 1)
    }

    /// Decode audio bytes in memory (requires the `audio-decode` feature).
    ///
    /// Any format symphonia supports decodes: WAV, MP3, FLAC, AAC/M4A, Ogg
    /// Vorbis. Output is normalized to [`MASTER_SAMPLE_RATE`] and
    /// [`MASTER_CHANNELS`] — other sample rates are resampled with rubato's
    /// FFT resampler, mono is duplicated to stereo, and beyond-stereo channel
    /// layouts keep their front left/right pair.
    ///
    /// This is the whole-file entry point that works everywhere `std::io`
    /// does, including wasm; [`AudioClip::from_file`] is a thin wrapper.
    #[cfg(feature = "audio-decode")]
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, AudioError> {
        decode(bytes)
    }

    /// Decode an audio file (requires the `audio-decode` feature).
    ///
    /// Pure-Rust decoding via symphonia — no ffmpeg binary on `PATH`. See
    /// [`AudioClip::from_bytes`] for the supported formats and the
    /// normalization applied to the output.
    #[cfg(feature = "audio-decode")]
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, AudioError> {
        Self::from_bytes(&std::fs::read(path)?)
    }

    /// Sample rate of the PCM data.
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// Channel count of the interleaved PCM data.
    pub fn channels(&self) -> u16 {
        self.channels
    }

    /// The interleaved PCM samples.
    pub fn pcm(&self) -> &Arc<[f32]> {
        &self.pcm
    }

    /// Duration in seconds.
    pub fn duration_secs(&self) -> f64 {
        self.frames() as f64 / self.sample_rate as f64
    }

    fn frames(&self) -> usize {
        self.pcm.len() / self.channels as usize
    }
}

/// Decode interleaved `f32` PCM from encoded audio bytes.
///
/// Malformed packets are skipped (matching rodio's leniency); a stream that
/// changes signal spec mid-way is rejected instead of silently misaligned.
#[cfg(feature = "audio-decode")]
fn decode(bytes: &[u8]) -> Result<AudioClip, AudioError> {
    use symphonia::core::audio::SampleBuffer;
    use symphonia::core::units::Duration;

    let mss = MediaSourceStream::new(
        // MediaSourceStream owns its source; keep the bytes via an owned copy.
        Box::new(Cursor::new(bytes.to_vec())),
        Default::default(),
    );
    let mut probed = symphonia::default::get_probe()
        .format(
            &Hint::new(),
            mss,
            &FormatOptions::default(),
            &MetadataOptions::default(),
        )
        .map_err(|err| AudioError(format!("failed to probe audio: {err}")))?;
    let format = &mut probed.format;
    let track = format
        .tracks()
        .iter()
        .find(|track| track.codec_params.codec != CODEC_TYPE_NULL)
        .ok_or_else(|| AudioError("no decodable audio track".to_string()))?;
    let track_id = track.id;
    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &DecoderOptions::default())
        .map_err(|err| AudioError(format!("unsupported codec: {err}")))?;

    let mut pcm = Vec::new();
    let mut spec = None;
    loop {
        let packet = match format.next_packet() {
            Ok(packet) => packet,
            // Symphonia signals end of stream as an io error.
            Err(SymphoniaError::IoError(_)) => break,
            Err(err) => return Err(AudioError(format!("failed to read packet: {err}"))),
        };
        if packet.track_id() != track_id {
            continue;
        }
        let decoded = match decoder.decode(&packet) {
            Ok(decoded) => decoded,
            Err(SymphoniaError::DecodeError(_)) => continue,
            Err(SymphoniaError::IoError(_)) => break,
            Err(err) => return Err(AudioError(format!("failed to decode packet: {err}"))),
        };
        if spec.unwrap_or(*decoded.spec()) != *decoded.spec() {
            return Err(AudioError("stream changes format mid-way".to_string()));
        }
        spec = Some(*decoded.spec());
        let mut buffer = SampleBuffer::<f32>::new(decoded.capacity() as Duration, *decoded.spec());
        buffer.copy_interleaved_ref(decoded);
        pcm.extend_from_slice(buffer.samples());
    }
    let spec = spec.ok_or_else(|| AudioError("no audio frames decoded".to_string()))?;

    // Adapt to master stereo: mono is duplicated, beyond-stereo layouts keep
    // their front left/right pair.
    let src_channels = spec.channels.count();
    let frames = pcm.len() / src_channels;
    let mut stereo = vec![0.0; frames * MASTER_CHANNELS as usize];
    for (frame, out) in stereo
        .as_chunks_mut::<{ MASTER_CHANNELS as usize }>()
        .0
        .iter_mut()
        .enumerate()
    {
        for (channel, slot) in out.iter_mut().enumerate() {
            let src = frame * src_channels + channel.min(src_channels - 1);
            *slot = pcm[src];
        }
    }
    let pcm = if spec.rate != MASTER_SAMPLE_RATE {
        resample_to_master(stereo, spec.rate)?
    } else {
        stereo
    };
    Ok(AudioClip {
        sample_rate: MASTER_SAMPLE_RATE,
        channels: MASTER_CHANNELS,
        pcm: pcm.into(),
    })
}

/// Resample interleaved master-stereo PCM to [`MASTER_SAMPLE_RATE`] with
/// rubato's FFT (synchronous) resampler in one offline pass.
#[cfg(feature = "audio-decode")]
fn resample_to_master(pcm: Vec<f32>, src_rate: u32) -> Result<Vec<f32>, AudioError> {
    use rubato::audioadapter_buffers::owned::InterleavedOwned;
    use rubato::{Fft, FixedSync, Resampler};

    let channels = MASTER_CHANNELS as usize;
    let frames = pcm.len() / channels;
    let input = InterleavedOwned::<f32>::new_from(pcm, channels, frames)
        .map_err(|err| AudioError(format!("resampler input: {err}")))?;
    let mut resampler = Fft::<f32>::new(
        src_rate as usize,
        MASTER_SAMPLE_RATE as usize,
        1024,
        channels,
        FixedSync::Both,
    )
    .map_err(|err| AudioError(format!("failed to init resampler: {err}")))?;
    let output = resampler
        .process_all(&input, frames, None)
        .map_err(|err| AudioError(format!("resampling failed: {err}")))?;
    Ok(output.take_data())
}

/// A sound's content data: the clip plus how to play it.
///
/// Purely substance — clip trim and gain envelope. Everything time-positional
/// (placement, window duration, enable, rate warps — including linear speed
/// changes via `with_duration`) lives on the cell layer (`Unplaced::at`,
/// `with_duration`, `with_rate_func`, `with_enabled`), exactly like a visual
/// item's data versus its [`AnimNode`](crate::animation::node::AnimNode).
#[derive(Debug, Clone)]
pub struct AudioTrack {
    clip: AudioClip,
    gain: f64,
    fade_in_secs: f64,
    fade_out_secs: f64,
    play_range: Option<Range<f64>>,
}

impl AudioTrack {
    /// A track playing the whole clip at unit gain.
    pub fn new(clip: AudioClip) -> Self {
        Self {
            clip,
            gain: 1.0,
            fade_in_secs: 0.0,
            fade_out_secs: 0.0,
            play_range: None,
        }
    }

    /// Set the track's linear gain.
    pub fn with_gain(mut self, gain: f64) -> Self {
        self.gain = gain;
        self
    }

    /// Fade in linearly over the first `secs` of the track.
    pub fn with_fade_in(mut self, secs: f64) -> Self {
        self.fade_in_secs = secs;
        self
    }

    /// Fade out linearly over the last `secs` of the track.
    pub fn with_fade_out(mut self, secs: f64) -> Self {
        self.fade_out_secs = secs;
        self
    }

    /// Play only the clip's seconds within `range` — a content trim, unlike
    /// the cell layer's `with_duration`, which resamples the sound to fit a
    /// new window length.
    ///
    /// The range is clamped to the clip: a start before the clip pulls to
    /// zero, an end past the clip's duration truncates, and an empty or
    /// reversed span collapses to silence. Content-time zero always plays
    /// `range.start` seconds into the clip.
    pub fn with_play_secs(mut self, range: Range<f64>) -> Self {
        self.play_range = Some(range);
        self
    }

    /// The clip-second span actually played: `play_range` clamped to the
    /// clip's extent.
    fn source_span(&self) -> (f64, f64) {
        let duration = self.clip.duration_secs();
        match &self.play_range {
            None => (0.0, duration),
            Some(range) => {
                let start = range.start.max(0.0);
                (start, range.end.min(duration).max(start))
            }
        }
    }

    /// The track's play length (its whole content axis).
    pub(crate) fn play_window_secs(&self) -> f64 {
        let (start, end) = self.source_span();
        end - start
    }
}

impl AudioTrack {
    /// The stereo sample this track produces at content-time `own`, or
    /// silence when `own` falls outside the play window.
    ///
    /// A pure function of content time — the mixing walk calls it once per
    /// output sample. Fades are measured on the same content axis.
    pub(crate) fn sample_at(&self, own: f64) -> [f32; 2] {
        const SILENCE: [f32; 2] = [0.0; 2];
        let clip_frames = self.clip.frames();
        if clip_frames == 0 {
            return SILENCE;
        }
        let (start, end) = self.source_span();
        let play_len = end - start;
        if !(0.0..play_len).contains(&own) {
            return SILENCE;
        }
        let mut envelope = self.gain;
        if self.fade_in_secs > 0.0 && own < self.fade_in_secs {
            envelope *= own / self.fade_in_secs;
        }
        if self.fade_out_secs > 0.0 {
            envelope *= ((play_len - own) / self.fade_out_secs).min(1.0);
        }
        let envelope = envelope as f32;
        // One clip second per content second, offset into the trimmed span.
        let src_pos = (start + own) * self.clip.sample_rate as f64;
        let f0 = src_pos.floor() as usize;
        if f0 >= clip_frames {
            return SILENCE;
        }
        let frac = (src_pos - src_pos.floor()) as f32;
        let f1 = (f0 + 1).min(clip_frames - 1);
        let clip_channels = self.clip.channels as usize;
        let sample = |frame: usize, channel: usize| {
            self.clip.pcm[frame * clip_channels + channel.min(clip_channels - 1)]
        };
        let mut out = SILENCE;
        for (channel, slot) in out.iter_mut().enumerate() {
            let value = sample(f0, channel) + (sample(f1, channel) - sample(f0, channel)) * frac;
            *slot = value * envelope;
        }
        out
    }

    /// Add this track's samples for global frames `[g_lo, g_hi)` into `pcm`
    /// (the whole timeline's interleaved stereo buffer, absolute frame
    /// indices), where content time is the affine map
    /// `t(g) = a + b·(g / sample_rate)`.
    ///
    /// The seal-time bake's linear fast path: the per-sample math is
    /// identical to [`AudioTrack::sample_at`] (window, fades and gain all
    /// measured on the content axis), arranged as one tight loop over the
    /// frames the bake proved audible.
    pub(crate) fn mix_span_into(
        &self,
        a: f64,
        b: f64,
        g_lo: usize,
        g_hi: usize,
        sample_rate: f64,
        pcm: &mut [f32],
    ) {
        let (start, end) = self.source_span();
        let play_len = end - start;
        let clip_frames = self.clip.frames();
        if clip_frames == 0 || play_len <= 0.0 {
            return;
        }
        let clip_rate = self.clip.sample_rate as f64;
        let clip_channels = self.clip.channels as usize;
        let sample = |frame: usize, channel: usize| {
            self.clip.pcm[frame * clip_channels + channel.min(clip_channels - 1)]
        };
        for g in g_lo..g_hi {
            let own = a + b * (g as f64 / sample_rate);
            // Exact bake bounds keep this check almost always true; it stays
            // so float drift at a window edge cannot leak a sample.
            if !(0.0..play_len).contains(&own) {
                continue;
            }
            let mut envelope = self.gain;
            if self.fade_in_secs > 0.0 && own < self.fade_in_secs {
                envelope *= own / self.fade_in_secs;
            }
            if self.fade_out_secs > 0.0 {
                envelope *= ((play_len - own) / self.fade_out_secs).min(1.0);
            }
            let envelope = envelope as f32;
            let src_pos = (start + own) * clip_rate;
            let f0 = src_pos.floor() as usize;
            if f0 >= clip_frames {
                continue;
            }
            let frac = (src_pos - src_pos.floor()) as f32;
            let f1 = (f0 + 1).min(clip_frames - 1);
            let value = |channel: usize| {
                sample(f0, channel) + (sample(f1, channel) - sample(f0, channel)) * frac
            };
            pcm[g * 2] += value(0) * envelope;
            pcm[g * 2 + 1] += value(1) * envelope;
        }
    }
}

/// Encode interleaved `f32` PCM as a RIFF/WAVE file (IEEE float format).
pub fn write_wav(
    path: impl AsRef<Path>,
    pcm: &[f32],
    sample_rate: u32,
    channels: u16,
) -> std::io::Result<()> {
    let data_len = (pcm.len() * 4) as u32;
    let mut file = std::fs::File::create(path)?;
    let write_chunk =
        |file: &mut std::fs::File, id: &[u8; 4], payload: &[u8]| -> std::io::Result<()> {
            file.write_all(id)?;
            file.write_all(&(payload.len() as u32).to_le_bytes())?;
            file.write_all(payload)
        };
    // RIFF header: "RIFF" + size + "WAVE", then fmt + data chunks.
    file.write_all(b"RIFF")?;
    file.write_all(&(4 + 8 + 16 + 8 + data_len).to_le_bytes())?;
    file.write_all(b"WAVE")?;
    let mut fmt_chunk = Vec::with_capacity(16);
    fmt_chunk.extend_from_slice(&3u16.to_le_bytes()); // IEEE float
    fmt_chunk.extend_from_slice(&channels.to_le_bytes());
    fmt_chunk.extend_from_slice(&sample_rate.to_le_bytes());
    fmt_chunk.extend_from_slice(&(sample_rate * channels as u32 * 4).to_le_bytes()); // byte rate
    fmt_chunk.extend_from_slice(&(channels * 4).to_le_bytes()); // block align
    fmt_chunk.extend_from_slice(&32u16.to_le_bytes()); // bits per sample
    write_chunk(&mut file, b"fmt ", &fmt_chunk)?;
    let mut data = Vec::with_capacity(pcm.len() * 4);
    for sample in pcm {
        data.extend_from_slice(&sample.to_le_bytes());
    }
    write_chunk(&mut file, b"data", &data)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPS: f32 = 1e-6;

    fn constant(value: f32, secs: f64) -> AudioClip {
        let pcm = vec![value; (secs * MASTER_SAMPLE_RATE as f64) as usize * 2];
        AudioClip::from_pcm(pcm, MASTER_SAMPLE_RATE, 2)
    }

    fn sample_of(buf: &[f32], sec: f64) -> f32 {
        buf[(sec * MASTER_SAMPLE_RATE as f64) as usize * 2]
    }

    /// The track's stereo sample at scene time `sec` (identity placement).
    fn sampled(track: &AudioTrack, sec: f64) -> [f32; 2] {
        track.sample_at(sec)
    }

    /// Sum of the tracks' stereo samples at scene time `sec`.
    fn summed(tracks: &[AudioTrack], sec: f64) -> [f32; 2] {
        let mut acc = [0.0_f32; 2];
        for track in tracks {
            let [l, r] = track.sample_at(sec);
            acc[0] += l;
            acc[1] += r;
        }
        acc
    }

    /// Mix tracks under the identity map over `[0, total]` — the direct
    /// programmatic placement path.
    fn mixed(tracks: &[AudioTrack], total_secs: f64) -> Vec<f32> {
        let out_frames = (total_secs * MASTER_SAMPLE_RATE as f64).ceil() as usize;
        let mut out = vec![0.0; out_frames * MASTER_CHANNELS as usize];
        for frame in 0..out_frames {
            let t = frame as f64 / MASTER_SAMPLE_RATE as f64;
            let [l, r] = summed(tracks, t);
            out[frame * 2] = l;
            out[frame * 2 + 1] = r;
        }
        out
    }

    #[test]
    fn overlapping_tracks_sum() {
        let buf = mixed(
            &[
                AudioTrack::new(constant(0.25, 2.0)),
                AudioTrack::new(constant(0.25, 1.0)),
            ],
            3.0,
        );
        assert!((sample_of(&buf, 0.5) - 0.5).abs() < EPS);
        assert!((sample_of(&buf, 1.5) - 0.25).abs() < EPS);
        assert!(sample_of(&buf, 2.5).abs() < EPS);
    }

    #[test]
    fn fades_ramp_linearly() {
        let buf = mixed(
            &[AudioTrack::new(constant(1.0, 2.0))
                .with_fade_in(1.0)
                .with_fade_out(1.0)],
            2.0,
        );
        assert!(sample_of(&buf, 0.0).abs() < EPS);
        assert!((sample_of(&buf, 0.5) - 0.5).abs() < EPS);
        assert!((sample_of(&buf, 1.0) - 1.0).abs() < EPS);
        assert!((sample_of(&buf, 1.5) - 0.5).abs() < EPS);
        assert!(sample_of(&buf, 1.999).abs() < 2e-3);
    }

    #[test]
    fn gain_and_play_window_modify_the_track() {
        let buf = mixed(&[AudioTrack::new(constant(0.5, 1.0)).with_gain(0.2)], 1.0);
        assert!((sample_of(&buf, 0.5) - 0.1).abs() < EPS);

        let buf = mixed(
            &[AudioTrack::new(constant(0.5, 4.0)).with_play_secs(0.0..1.0)],
            3.0,
        );
        assert!((sample_of(&buf, 0.6) - 0.5).abs() < EPS);
        assert!(sample_of(&buf, 1.6).abs() < EPS);
    }

    #[test]
    fn play_range_skips_into_the_clip() {
        // A 4 Hz ramp whose frame values encode their position; playing
        // 1.0..3.0 makes content-time zero read frame 4 and the window end
        // at 2 content seconds.
        let ramp: Vec<f32> = (0..16).flat_map(|i| [i as f32, i as f32]).collect();
        let clip = AudioClip::from_pcm(ramp, 4, 2);
        let track = AudioTrack::new(clip).with_play_secs(1.0..3.0);
        assert!((sampled(&track, 0.0)[0] - 4.0).abs() < EPS);
        assert!((sampled(&track, 1.0)[0] - 8.0).abs() < EPS);
        assert!(sampled(&track, 2.0)[0].abs() < EPS);
        assert!((track.play_window_secs() - 2.0).abs() < f64::from(EPS));
    }

    #[test]
    fn play_range_is_clamped_to_the_clip() {
        let clip = AudioClip::from_pcm(vec![0.5; 8], 4, 1);
        // End past the duration truncates; the window is the whole clip.
        let track = AudioTrack::new(clip.clone()).with_play_secs(0.0..10.0);
        assert!((track.play_window_secs() - 2.0).abs() < f64::from(EPS));
        // A negative start pulls to zero.
        let track = AudioTrack::new(clip.clone()).with_play_secs(-1.0..2.0);
        assert!((track.play_window_secs() - 2.0).abs() < f64::from(EPS));
        // Reversed or past-the-end spans are silence.
        let track = AudioTrack::new(clip.clone()).with_play_secs(3.0..1.0);
        assert_eq!(track.play_window_secs(), 0.0);
        assert!(sampled(&track, 0.0)[0].abs() < EPS);
        let track = AudioTrack::new(clip).with_play_secs(5.0..9.0);
        assert_eq!(track.play_window_secs(), 0.0);
    }

    #[test]
    fn track_is_trimmed_to_buffer_end() {
        let buf = mixed(&[AudioTrack::new(constant(0.5, 10.0))], 1.0);
        assert_eq!(buf.len(), MASTER_SAMPLE_RATE as usize * 2);
        assert!((sample_of(&buf, 0.99) - 0.5).abs() < EPS);
    }

    #[test]
    fn mono_clip_duplicates_to_stereo() {
        let clip = AudioClip::from_pcm(vec![0.5; 48], 48, 1);
        let buf = mixed(&[AudioTrack::new(clip)], 1.0);
        assert!((buf[0] - 0.5).abs() < EPS && (buf[1] - 0.5).abs() < EPS);
    }

    #[test]
    fn clip_is_resampled_by_linear_interpolation() {
        // A 24 Hz clip sampled on the 48 Hz grid: its two frames land one
        // output frame apart, with the midpoint interpolated. t=1/48 reads
        // clip position 0.5 -> 1.0; t=2/48 reads position 1.0 -> 2.0;
        // t=3/48 runs past the last frame and clamps; the play window ends
        // at t=1/12, beyond which the track is silent.
        let clip = AudioClip::from_pcm(vec![0.0, 2.0], 24, 1);
        let track = AudioTrack::new(clip);
        assert!((sampled(&track, 0.0 / 48.0)[0] - 0.0).abs() < EPS);
        assert!((sampled(&track, 1.0 / 48.0)[0] - 1.0).abs() < EPS);
        assert!((sampled(&track, 2.0 / 48.0)[0] - 2.0).abs() < EPS);
        assert!((sampled(&track, 3.0 / 48.0)[0] - 2.0).abs() < EPS);
        assert!(sampled(&track, 4.0 / 48.0)[0].abs() < EPS);
    }

    #[test]
    fn wav_roundtrip_has_a_sound_header() {
        let path = std::env::temp_dir().join(format!("ranim-wav-test-{}.wav", std::process::id()));
        let pcm = vec![0.25f32; 480];
        write_wav(&path, &pcm, MASTER_SAMPLE_RATE, MASTER_CHANNELS).unwrap();
        let bytes = std::fs::read(&path).unwrap();
        std::fs::remove_file(&path).unwrap();
        assert_eq!(&bytes[0..4], b"RIFF");
        assert_eq!(&bytes[8..12], b"WAVE");
        assert_eq!(&bytes[12..16], b"fmt ");
        let channels = u16::from_le_bytes(bytes[22..24].try_into().unwrap());
        assert_eq!(channels, MASTER_CHANNELS);
        assert_eq!(&bytes[36..40], b"data");
        let data_len = u32::from_le_bytes(bytes[40..44].try_into().unwrap());
        assert_eq!(data_len as usize, pcm.len() * 4);
    }

    #[test]
    #[cfg(feature = "audio-decode")]
    fn from_file_decodes_the_written_wav() {
        let path =
            std::env::temp_dir().join(format!("ranim-decode-test-{}.wav", std::process::id()));
        let pcm = vec![0.5f32; 480 * MASTER_CHANNELS as usize];
        write_wav(&path, &pcm, MASTER_SAMPLE_RATE, MASTER_CHANNELS).unwrap();
        let clip = AudioClip::from_file(&path).unwrap();
        std::fs::remove_file(&path).unwrap();
        assert_eq!(clip.sample_rate(), MASTER_SAMPLE_RATE);
        assert_eq!(clip.channels(), MASTER_CHANNELS);
        assert!((clip.duration_secs() - 480.0 / MASTER_SAMPLE_RATE as f64).abs() < 1e-9);
        assert!((clip.pcm()[0] - 0.5).abs() < 1e-6);
    }

    #[test]
    #[cfg(feature = "audio-decode")]
    fn decoding_resamples_to_master_rate() {
        // A 24 kHz stereo wav with a click at 0.5 s; decoding must keep the
        // click at 0.5 s on the 48 kHz grid (resampler delay trimmed) and
        // scale the duration.
        let rate = 24_000u32;
        let click = rate as usize / 2;
        let mut pcm = vec![0.0f32; rate as usize * 2];
        pcm[click * 2] = 1.0;
        pcm[click * 2 + 1] = 1.0;
        let path =
            std::env::temp_dir().join(format!("ranim-resample-test-{}.wav", std::process::id()));
        write_wav(&path, &pcm, rate, MASTER_CHANNELS).unwrap();
        let clip = AudioClip::from_file(&path).unwrap();
        std::fs::remove_file(&path).unwrap();
        assert_eq!(clip.sample_rate(), MASTER_SAMPLE_RATE);
        assert_eq!(clip.channels(), MASTER_CHANNELS);
        let samples = clip.pcm();
        let peak = samples
            .as_chunks::<2>()
            .0
            .iter()
            .enumerate()
            .max_by(|a, b| a.1[0].abs().total_cmp(&b.1[0].abs()))
            .unwrap()
            .0;
        assert!(
            (peak as i64 - 24_000).abs() < 1_000,
            "click at frame {peak}, expected ~24000"
        );
        assert!(
            (samples.len() as i64 / 2 - 48_000).abs() < 2_000,
            "duration {} frames, expected ~48000",
            samples.len() / 2
        );
    }

    #[test]
    #[cfg(feature = "audio-decode")]
    fn decoding_duplicates_mono_to_stereo() {
        let pcm = vec![0.5f32; MASTER_SAMPLE_RATE as usize];
        let path = std::env::temp_dir().join(format!("ranim-mono-test-{}.wav", std::process::id()));
        write_wav(&path, &pcm, MASTER_SAMPLE_RATE, 1).unwrap();
        let clip = AudioClip::from_file(&path).unwrap();
        std::fs::remove_file(&path).unwrap();
        assert_eq!(clip.sample_rate(), MASTER_SAMPLE_RATE);
        assert_eq!(clip.channels(), MASTER_CHANNELS);
        assert_eq!(clip.pcm().len(), MASTER_SAMPLE_RATE as usize * 2);
        assert!(clip.pcm().iter().all(|&s| (s - 0.5).abs() < 1e-6));
    }

    #[test]
    #[cfg(feature = "audio-decode")]
    fn decoding_garbage_is_an_error() {
        assert!(AudioClip::from_bytes(b"not an audio file").is_err());
    }
}
