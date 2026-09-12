//! Audio leaf animation: a sound placed and composed like any animation.
//!
//! [`Sound`](crate::animation::sound::Sound) makes the audio plane part of the animation tree: a sound sits
//! in `seq!`/`stack!`/`lagged!` beside visual animations, shares their
//! placement vocabulary ([`Unplaced::at`](crate::animation::build::Unplaced::at), duration overrides, enable), and
//! never enters the per-frame evaluation pipeline.
//!
//! The leaf contract mirrors `Eval`: content is a pure span, and all time
//! management (windows, placement, container remaps) lives on the cells. At
//! mix time the cells' remaps compose along the tree path — container rate
//! functions and duration overrides warp a sound's playback exactly like the
//! surrounding motion, pitch movement included.

use std::any::type_name;

use crate::audio::{AudioClip, AudioTrack};

use crate::animation::build::{IntoAnimNode, Unplaced};
use crate::animation::node::{AnimNode, NodeContent};

/// An audio leaf: plays a clip's span inside its placed window.
///
/// Authored with [`Sound::new`] plus the track shapers (`with_gain`,
/// `with_fade_in`, ...), then composed exactly like a visual animation:
///
/// ```ignore
/// seq![
///     square.fade_in(),
///     Sound::new(narration),          // plays while the next cells run
///     square.write(),
/// ]
/// ```
pub struct Sound {
    track: AudioTrack,
}

impl Sound {
    /// A sound playing the whole clip at unit gain.
    pub fn new(clip: AudioClip) -> Self {
        Self {
            track: AudioTrack::new(clip),
        }
    }

    /// A sound from a shaped track (gain, fades, speed, play length).
    pub fn from_track(track: AudioTrack) -> Self {
        Self { track }
    }

    /// Set the sound's linear gain.
    pub fn with_gain(mut self, gain: f64) -> Self {
        self.track = self.track.with_gain(gain);
        self
    }

    /// Fade in linearly over the first `secs` of the sound.
    pub fn with_fade_in(mut self, secs: f64) -> Self {
        self.track = self.track.with_fade_in(secs);
        self
    }

    /// Fade out linearly over the last `secs` of the sound.
    pub fn with_fade_out(mut self, secs: f64) -> Self {
        self.track = self.track.with_fade_out(secs);
        self
    }

    /// Resample the clip: `speed` 2.0 plays twice as fast (one octave up).
    pub fn with_speed(mut self, speed: f64) -> Self {
        self.track = self.track.with_speed(speed);
        self
    }

    /// The sound's natural window length: the clip's play length.
    pub fn duration_secs(&self) -> f64 {
        self.track.play_window_secs()
    }
}

impl Unplaced for Sound {}
impl IntoAnimNode for Sound {
    fn into_anim_node(self) -> AnimNode {
        let window = self.duration_secs();
        AnimNode {
            content: NodeContent::Audio(Box::new(self.track)),
            internal_time_secs: window,
            rate_func: None,
            time_range: 0.0..window,
            enabled: true,
            anim_name: type_name::<Self>(),
        }
    }
}
