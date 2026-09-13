//! Preview audio playback (native only).
//!
//! The scene's mixed audio buffer is played through rodio. The player keeps
//! the output device open across play/pause cycles; its position readout is a
//! wall-clock estimate over the playback rate, clamped to the buffer duration
//! — the audio device consumes in real time, so both advance together and
//! never accumulate drift.

use std::{sync::Arc, time::Duration, time::Instant};

use rodio::{
    Sink,
    source::{SeekError, Source},
};

use ranim_core::{
    SceneEvaluator,
    audio::{MASTER_CHANNELS, MASTER_SAMPLE_RATE},
};

/// The scene's mixed audio buffer, produced in one offline pass and shared
/// with the playback device.
#[derive(Clone)]
pub(crate) struct MixedAudio {
    pcm: Arc<[f32]>,
    total_secs: f64,
}

impl MixedAudio {
    /// The scene's baked audio, shared from the seal-time mix.
    pub fn new(evaluator: &SceneEvaluator, total_secs: f64) -> Self {
        Self {
            pcm: evaluator.audio().clone(),
            total_secs,
        }
    }

    fn is_empty(&self) -> bool {
        self.pcm.is_empty()
    }

    fn total_secs(&self) -> f64 {
        self.total_secs
    }
}

/// Interleaved stereo PCM source with sample-accurate absolute seeks.
struct PcmSource {
    pcm: Arc<[f32]>,
    next: usize,
}

impl Iterator for PcmSource {
    type Item = f32;

    fn next(&mut self) -> Option<f32> {
        let sample = self.pcm.get(self.next).copied();
        if sample.is_some() {
            self.next += 1;
        }
        sample
    }
}

impl Source for PcmSource {
    fn current_span_len(&self) -> Option<usize> {
        Some(self.pcm.len() - self.next)
    }

    fn channels(&self) -> u16 {
        MASTER_CHANNELS
    }

    fn sample_rate(&self) -> u32 {
        MASTER_SAMPLE_RATE
    }

    fn total_duration(&self) -> Option<Duration> {
        Some(Duration::from_secs_f64(
            self.pcm.len() as f64 / (MASTER_SAMPLE_RATE as f64 * MASTER_CHANNELS as f64),
        ))
    }

    fn try_seek(&mut self, pos: Duration) -> Result<(), SeekError> {
        let frame = (pos.as_secs_f64() * MASTER_SAMPLE_RATE as f64) as usize;
        self.next = (frame * MASTER_CHANNELS as usize).min(self.pcm.len());
        Ok(())
    }
}

enum PlayerState {
    Playing { started_at: Instant, base_sec: f64 },
    Paused { at_sec: f64 },
}

/// Scrub voices play at reduced volume and silence themselves once the
/// playhead stops moving.
const SCRUB_GAIN: f32 = 0.6;
const SCRUB_IDLE: Duration = Duration::from_millis(120);

/// Owns the output device and the playback sink for one scene's audio.
pub(crate) struct AudioPlayer {
    mixed: MixedAudio,
    sink: Sink,
    // The output stream must outlive the sink; dropped last.
    _stream: rodio::OutputStream,
    state: PlayerState,
    speed: f64,
    /// Last scrub move while a scrub voice is audible; `None` otherwise.
    scrub_at: Option<Instant>,
}

impl AudioPlayer {
    /// Open the default output device and prepare the scene buffer for
    /// playback. Returns `None` when the scene is silent or no device is
    /// available (the preview then runs on the wall clock).
    pub fn try_new(mixed: MixedAudio) -> Option<Self> {
        if mixed.is_empty() {
            return None;
        }
        let stream = rodio::OutputStreamBuilder::open_default_stream().ok()?;
        let sink = Sink::connect_new(stream.mixer());
        sink.pause();
        Some(Self {
            mixed,
            sink,
            _stream: stream,
            state: PlayerState::Paused { at_sec: 0.0 },
            speed: 1.0,
            scrub_at: None,
        })
    }

    /// Current media position in seconds; frozen while paused.
    pub fn pos_secs(&self) -> f64 {
        match self.state {
            PlayerState::Playing {
                started_at,
                base_sec,
            } => (base_sec + started_at.elapsed().as_secs_f64() * self.speed)
                .clamp(0.0, self.mixed.total_secs()),
            PlayerState::Paused { at_sec } => at_sec,
        }
    }

    /// Start playing from `sec` at `speed`.
    pub fn play_from(&mut self, sec: f64, speed: f64) {
        let sec = sec.clamp(0.0, self.mixed.total_secs());
        self.speed = speed;
        self.sink.set_volume(1.0);
        self.sink.set_speed(speed as f32);
        self.sink.clear();
        self.sink.append(PcmSource {
            pcm: self.mixed.pcm.clone(),
            next: (sec * MASTER_SAMPLE_RATE as f64) as usize * MASTER_CHANNELS as usize,
        });
        self.sink.play();
        self.scrub_at = None;
        self.state = PlayerState::Playing {
            started_at: Instant::now(),
            base_sec: sec,
        };
    }

    /// Stop playback and freeze the position.
    pub fn pause(&mut self) {
        let pos = self.pos_secs();
        self.scrub_at = None;
        self.sink.set_volume(1.0);
        self.sink.pause();
        self.state = PlayerState::Paused { at_sec: pos };
    }

    /// Seek to `sec`, staying paused or playing as-is.
    pub fn seek_to(&mut self, sec: f64) {
        let sec = sec.clamp(0.0, self.mixed.total_secs());
        match self.state {
            PlayerState::Playing { .. } => {
                self.ensure_source_at(sec);
                self.state = PlayerState::Playing {
                    started_at: Instant::now(),
                    base_sec: sec,
                };
            }
            PlayerState::Paused { .. } => {
                self.state = PlayerState::Paused { at_sec: sec };
            }
        }
    }

    /// Point the playback source at `sec`.
    ///
    /// Seeks in place — but if the source already ran to the end and left
    /// the queue empty (scrubbing to the scene end does this), a seek is a
    /// no-op and the source must be re-appended at the target instead.
    fn ensure_source_at(&mut self, sec: f64) {
        if self.sink.empty() {
            self.sink.clear();
            self.sink.append(PcmSource {
                pcm: self.mixed.pcm.clone(),
                next: (sec * MASTER_SAMPLE_RATE as f64) as usize * MASTER_CHANNELS as usize,
            });
        } else {
            let _ = self.sink.try_seek(Duration::from_secs_f64(sec));
        }
    }

    /// Audible scrubbing while paused: point the playback voice at `sec` at
    /// reduced volume. Between scrub moves it simply plays on, so a drag
    /// hears the audio under the cursor; [`Self::end_stale_scrub`] silences
    /// it once the playhead stops moving.
    pub fn scrub(&mut self, sec: f64) {
        let sec = sec.clamp(0.0, self.mixed.total_secs());
        self.sink.set_volume(SCRUB_GAIN);
        self.ensure_source_at(sec);
        self.sink.play();
        self.scrub_at = Some(Instant::now());
        self.state = PlayerState::Playing {
            started_at: Instant::now(),
            base_sec: sec,
        };
    }

    /// Silence the scrub voice after the playhead stopped moving.
    pub fn end_stale_scrub(&mut self) {
        let Some(last_move) = self.scrub_at else {
            return;
        };
        if last_move.elapsed() < SCRUB_IDLE {
            return;
        }
        self.pause();
    }

    /// Whether a scrub voice is still audible.
    pub fn scrub_voice_active(&self) -> bool {
        self.scrub_at.is_some()
    }

    /// Change the playback speed, rebasing the position clock.
    pub fn set_speed(&mut self, speed: f64) {
        let pos = self.pos_secs();
        self.speed = speed;
        self.sink.set_speed(speed as f32);
        if matches!(self.state, PlayerState::Playing { .. }) {
            self.state = PlayerState::Playing {
                started_at: Instant::now(),
                base_sec: pos,
            };
        }
    }
}
