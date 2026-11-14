//! Authoring protocol and playback wrappers for animation definitions.
//!
//! [`IntoAnimNode`] is the lowering protocol: it turns an authoring definition
//! into an [`AnimNode`]. It is intentionally
//! separate from [`Eval`](super::eval::Eval), the typed visual-leaf
//! protocol. Operations bind their parameters to inputs using `Anim`;
//! containers and evaluator adapters explicitly implement this protocol.
//!
//! [`Unplaced`] marks definitions that have not been fixed to a parent time
//! coordinate yet, so containers and [`Unplaced::at`] may schedule them.

use super::node::AnimNode;

/// A definition that can be lowered into the runtime animation tree.
pub trait IntoAnimNode: Sized {
    /// Lower this definition into its local runtime representation.
    fn into_anim_node(self) -> AnimNode;
}

impl IntoAnimNode for AnimNode {
    fn into_anim_node(self) -> AnimNode {
        self
    }
}

/// Capability for animation definitions that have not been fixed in parent
/// time coordinates.
///
/// This trait is used to constrain the anims that can be inserted into
/// [`AnimSequence`](crate::animation::compose::sequence::AnimSequence). Only
/// anims that are not placed can be inserted into it.
pub trait Unplaced: IntoAnimNode {
    /// Place this definition at an offset in its parent's local time coordinates.
    fn at(self, offset_sec: f64) -> At<Self> {
        At {
            inner: self,
            offset_sec,
        }
    }
}

/// Playback parameter builders for animations that have not been placed yet.
pub trait PlaybackExt: Unplaced {
    /// Change the animation's rate function.
    fn with_rate_func(self, rate_func: fn(f64) -> f64) -> Paramed<Self> {
        Paramed::new(self).with_rate_func(rate_func)
    }

    /// Override an inner rate with the structural identity mapping.
    fn with_linear_rate(self) -> Paramed<Self> {
        Paramed::new(self).with_linear_rate()
    }

    /// Change the animation's duration.
    fn with_duration(self, duration_secs: f64) -> Paramed<Self> {
        Paramed::new(self).with_duration(duration_secs)
    }

    /// Enable or disable this animation's output.
    fn with_enabled(self, enabled: bool) -> Paramed<Self> {
        Paramed::new(self).with_enabled(enabled)
    }
}

impl<A: Unplaced> PlaybackExt for A {}

/// Playback parameters applied to an animation definition.
#[derive(Debug, Clone, Default)]
pub(crate) struct AnimationParam {
    /// `None` preserves the inner rate; a present override selects a function
    /// or the structural identity mapping used by affine audio mixing.
    pub rate_func: Option<RateOverride>,
    /// Optional duration override in seconds.
    pub duration_secs: Option<f64>,
    /// Whether this animation contributes a value.
    pub enabled: Option<bool>,
}

#[derive(Debug, Clone)]
pub(crate) enum RateOverride {
    Linear,
    Function(fn(f64) -> f64),
}

/// An animation definition with overridden playback parameters.
pub struct Paramed<A> {
    inner: A,
    param: AnimationParam,
}

impl<A> Paramed<A> {
    /// Borrow the original definition, including its editable input and parameters.
    pub fn inner(&self) -> &A {
        &self.inner
    }

    /// Edit the original definition before preparing runtime execution data.
    pub fn inner_mut(&mut self) -> &mut A {
        &mut self.inner
    }

    /// Wrap an animation without overriding its duration.
    pub(crate) fn new(inner: A) -> Self {
        Self {
            inner,
            param: AnimationParam::default(),
        }
    }

    /// Change the animation's rate function.
    pub fn with_rate_func(mut self, rate_func: fn(f64) -> f64) -> Self {
        self.param.rate_func = Some(RateOverride::Function(rate_func));
        self
    }

    /// Override an inner rate with the structural identity mapping.
    pub fn with_linear_rate(mut self) -> Self {
        self.param.rate_func = Some(RateOverride::Linear);
        self
    }

    /// Change the animation's duration.
    ///
    /// For a [`Sound`](crate::animation::sound::Sound) this resamples the
    /// audio linearly: playing a clip faster also shifts its pitch up, since
    /// the whole content span is warped onto the new window.
    pub fn with_duration(mut self, duration_secs: f64) -> Self {
        assert_valid_duration(duration_secs);
        self.param.duration_secs = Some(duration_secs);
        self
    }

    /// Enable or disable this animation's output.
    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.param.enabled = Some(enabled);
        self
    }
}

impl<A: Unplaced + 'static> Unplaced for Paramed<A> {}
impl<A: Unplaced + 'static> IntoAnimNode for Paramed<A> {
    fn into_anim_node(self) -> AnimNode {
        let mut cell = self.inner.into_anim_node();
        if let Some(duration_secs) = self.param.duration_secs {
            cell.time_range = 0.0..duration_secs;
        }
        if let Some(rate_func) = self.param.rate_func {
            cell.rate_func = match rate_func {
                RateOverride::Linear => None,
                RateOverride::Function(function) => Some(function),
            };
        }
        if let Some(enabled) = self.param.enabled {
            cell.enabled = enabled;
        }
        cell
    }
}

/// An animation fixed at an offset in its parent's time coordinates.
///
/// This is a terminal placement entry: it implements
/// [`IntoAnimNode`] but not [`Unplaced`], so playback parameters must be
/// configured before calling [`Unplaced::at`].
pub struct At<A> {
    inner: A,
    offset_sec: f64,
}

impl<A: IntoAnimNode> IntoAnimNode for At<A> {
    fn into_anim_node(self) -> AnimNode {
        let mut animation = self.inner.into_anim_node();
        animation.shift_by(self.offset_sec);
        animation
    }
}

fn assert_valid_duration(duration_secs: f64) {
    assert!(
        duration_secs.is_finite() && duration_secs >= 0.0,
        "animation duration must be finite and non-negative"
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{animation::eval::Static, utils::rate_functions::smooth};

    struct Configured;
    impl Unplaced for Configured {}
    impl IntoAnimNode for Configured {
        fn into_anim_node(self) -> AnimNode {
            let mut node = AnimNode::from_eval(Static(0_u8), 3.0);
            node.enabled = false;
            node.rate_func = Some(smooth);
            node.anim_name = "configured";
            node
        }
    }

    #[test]
    fn overrides_preserve_unspecified_defaults_and_inner_identity() {
        let node = Configured.with_duration(5.0).into_anim_node();
        assert_eq!(node.duration_secs(), 5.0);
        assert!(!node.enabled());
        assert_eq!(node.anim_name(), "configured");
        assert_eq!(node.rate_func.unwrap()(0.25), smooth(0.25));
        let node = Configured.with_enabled(true).into_anim_node();
        assert_eq!(node.duration_secs(), 3.0);
        assert!(node.enabled());
        assert!(node.rate_func.is_some());
    }

    #[test]
    fn nested_overrides_only_replace_requested_fields_and_can_clear_rates() {
        let inner = Configured.with_enabled(true).with_rate_func(smooth);
        let node = PlaybackExt::with_duration(inner, 6.0).into_anim_node();
        assert!(node.enabled());
        assert_eq!(node.duration_secs(), 6.0);
        assert!(node.rate_func.is_some());
        let node = Configured.with_linear_rate().into_anim_node();
        assert!(node.rate_func.is_none());
        assert!(!node.enabled());
    }
}
