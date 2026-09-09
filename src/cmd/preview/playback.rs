//! The preview playback state machine: one clock, two strategies.
//!
//! While playing there is exactly one source of time. If the scene carries
//! audio and an output device is available, the audio player is the clock and
//! each repaint reads its position; otherwise the wall clock advances the
//! playhead. In both cases the visual is a pure function sampled at the clock
//! reading — nothing is evaluated between repaints, and errors never
//! accumulate because the playhead is always *read*, never integrated.
//!
//! Paused, there is no clock at all: the playhead is a plain position
//! variable only moved by scrubbing, stepping, or reading the clock one last
//! time when pausing.

use web_time::Instant;

use ranim_core::SceneEvaluator;

#[cfg(all(feature = "audio", not(target_family = "wasm")))]
use super::audio::{AudioPlayer, MixedAudio};

/// The active clock while playing.
enum PlaybackClock {
    Wall {
        started_at: Instant,
        base_sec: f64,
        speed: f64,
    },
    #[cfg(all(feature = "audio", not(target_family = "wasm")))]
    Audio { player: AudioPlayer },
}

impl PlaybackClock {
    fn pos_secs(&self) -> f64 {
        match self {
            PlaybackClock::Wall {
                started_at,
                base_sec,
                speed,
            } => base_sec + started_at.elapsed().as_secs_f64() * speed,
            #[cfg(all(feature = "audio", not(target_family = "wasm")))]
            PlaybackClock::Audio { player } => player.pos_secs(),
        }
    }
}

/// Playback engine for the preview: owns the persistent audio player (so the
/// output device survives pause cycles) and the active clock while playing.
pub(crate) struct PlaybackEngine {
    clock: Option<PlaybackClock>,
    #[cfg(all(feature = "audio", not(target_family = "wasm")))]
    player: Option<AudioPlayer>,
}

impl PlaybackEngine {
    /// Build an engine for a scene, preparing its mixed audio for playback.
    pub fn new(evaluator: &SceneEvaluator) -> Self {
        Self {
            clock: None,
            #[cfg(all(feature = "audio", not(target_family = "wasm")))]
            player: Self::build_player(evaluator),
        }
    }

    #[cfg(all(feature = "audio", not(target_family = "wasm")))]
    fn build_player(evaluator: &SceneEvaluator) -> Option<AudioPlayer> {
        if !evaluator.has_audio() {
            return None;
        }
        let total_secs = evaluator.total_secs();
        match AudioPlayer::try_new(MixedAudio::new(evaluator, total_secs)) {
            Some(player) => Some(player),
            None => {
                tracing::warn!("no audio output device available; previewing without sound");
                None
            }
        }
    }

    /// Whether playback is running.
    pub fn is_playing(&self) -> bool {
        self.clock.is_some()
    }

    /// Start playing from `from` at `speed`, wrapping to 0.0 at `total`.
    /// Returns the effective start position.
    pub fn play(&mut self, from: f64, total: f64, speed: f64) -> f64 {
        let start = if from >= total { 0.0 } else { from };
        self.clock = Some({
            #[cfg(all(feature = "audio", not(target_family = "wasm")))]
            match self.player.take() {
                Some(mut player) => {
                    player.play_from(start, speed);
                    PlaybackClock::Audio { player }
                }
                None => PlaybackClock::Wall {
                    started_at: Instant::now(),
                    base_sec: start,
                    speed,
                },
            }
            #[cfg(any(not(feature = "audio"), target_family = "wasm"))]
            PlaybackClock::Wall {
                started_at: Instant::now(),
                base_sec: start,
                speed,
            }
        });
        start
    }

    /// Stop playback and return the frozen position.
    pub fn pause(&mut self) -> f64 {
        match self.clock.take() {
            Some(PlaybackClock::Wall {
                started_at,
                base_sec,
                speed,
            }) => base_sec + started_at.elapsed().as_secs_f64() * speed,
            #[cfg(all(feature = "audio", not(target_family = "wasm")))]
            Some(PlaybackClock::Audio { mut player }) => {
                player.pause();
                let pos = player.pos_secs();
                self.player = Some(player);
                pos
            }
            None => 0.0,
        }
    }

    /// The live position while playing; `None` when paused.
    pub fn pos_secs(&self) -> Option<f64> {
        self.clock.as_ref().map(PlaybackClock::pos_secs)
    }

    /// Rebase the active clock at `sec` (scrubbing). While paused, drive a
    /// scrub voice instead so the drag stays audible.
    pub fn scrub_to(&mut self, sec: f64) {
        match self.clock.as_mut() {
            Some(PlaybackClock::Wall {
                started_at,
                base_sec,
                ..
            }) => {
                *base_sec = sec;
                *started_at = Instant::now();
            }
            #[cfg(all(feature = "audio", not(target_family = "wasm")))]
            Some(PlaybackClock::Audio { player }) => player.seek_to(sec),
            #[cfg(all(feature = "audio", not(target_family = "wasm")))]
            None => {
                if let Some(player) = &mut self.player {
                    player.scrub(sec);
                }
            }
            #[cfg(any(not(feature = "audio"), target_family = "wasm"))]
            None => {}
        }
    }

    /// Silence the scrub voice once the playhead stopped moving while
    /// paused. Returns whether a scrub voice is still audible (the caller
    /// should keep repainting so the staleness check can run).
    pub fn tick_scrub(&mut self) -> bool {
        #[cfg(all(feature = "audio", not(target_family = "wasm")))]
        if self.clock.is_none()
            && let Some(player) = &mut self.player
        {
            player.end_stale_scrub();
            return player.scrub_voice_active();
        }
        false
    }

    /// Change the playback speed, rebasing the active clock.
    pub fn set_speed(&mut self, speed: f64) {
        match self.clock.as_mut() {
            Some(PlaybackClock::Wall {
                started_at,
                base_sec,
                speed: old_speed,
            }) => {
                let pos = *base_sec + started_at.elapsed().as_secs_f64() * *old_speed;
                *base_sec = pos;
                *started_at = Instant::now();
                *old_speed = speed;
            }
            #[cfg(all(feature = "audio", not(target_family = "wasm")))]
            Some(PlaybackClock::Audio { player }) => player.set_speed(speed),
            None => {}
        }
    }

    /// Swap in a freshly built scene: stop playback and rebuild the audio
    /// player from the new scene's audio plane.
    pub fn reload_scene(&mut self, evaluator: &SceneEvaluator) {
        self.clock = None;
        #[cfg(all(feature = "audio", not(target_family = "wasm")))]
        {
            self.player = Self::build_player(evaluator);
        }
    }
}
