//! The erased sound leaf node and its composition impls.

use std::{any::type_name, sync::Arc};

use crate::{
    audio::{Sound, SoundInfo, SoundSource},
    core_item::DynItem,
    utils::rate_functions::linear,
};

use super::{Animation, AnimationCell, AnimationInfoKind, Placeable, eval::EvalDyn};

/// The erased payload of a [`Sound`] leaf: an [`EvalDyn`] node that never
/// contributes visual items, and instead carries its source for the audio
/// plan flattener.
pub(crate) struct SoundNode {
    pub(crate) source: Arc<dyn SoundSource>,
    pub(crate) gain: f32,
    pub(crate) natural_secs: f64,
}

impl EvalDyn for SoundNode {
    // Audio never enters the per-frame evaluation output.
    fn eval_dyn(&self, _alpha: f64, _output: &mut Vec<DynItem>) {}

    fn info_kind(&self) -> AnimationInfoKind {
        AnimationInfoKind::Sound
    }

    fn content_duration_secs(&self) -> f64 {
        self.natural_secs
    }

    fn sound_info(&self) -> Option<SoundInfo> {
        Some(SoundInfo {
            source: Arc::clone(&self.source),
            gain: self.gain,
            natural_secs: self.natural_secs,
        })
    }
}

impl<S: SoundSource + 'static> Animation for Sound<S> {
    fn build(self) -> AnimationCell {
        let natural_secs = self.source.natural_secs().unwrap_or(1.0);
        AnimationCell {
            inner: Box::new(SoundNode {
                source: Arc::new(self.source),
                gain: self.gain,
                natural_secs,
            }),
            rate_func: linear,
            time_range: 0.0..natural_secs,
            enabled: true,
            anim_name: type_name::<Sound<S>>(),
        }
    }
}

impl<S: SoundSource + 'static> Placeable for Sound<S> {}
