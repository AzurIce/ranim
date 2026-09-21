//! Authoring protocol and playback wrappers for animation definitions.
//!
//! [`IntoAnimNode`](crate::animation::build::IntoAnimNode) is the lowering protocol: it turns an authoring definition
//! into an [`AnimNode`](crate::animation::node::AnimNode). It is intentionally
//! separate from [`Eval`](crate::animation::eval::Eval), the typed visual-leaf
//! protocol. Every `Eval` gets an [`IntoAnimNode`](crate::animation::build::IntoAnimNode) implementation by blanket
//! impl; the built-in containers implement it directly.
//!
//! [`Unplaced`](crate::animation::build::Unplaced) marks definitions that have not been fixed to a parent time
//! coordinate yet, so containers and [`Unplaced::at`](crate::animation::build::Unplaced::at) may schedule them.

use std::any::type_name;

use crate::core_item::AnyExtractCoreItem;

use super::{
    eval::Eval,
    node::{AnimNode, NodeContent},
};

/// A definition that can be lowered into the runtime animation tree.
pub trait IntoAnimNode: Sized {
    /// Lower this definition into its local runtime representation.
    fn into_anim_node(self) -> AnimNode;
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

impl<E> Unplaced for E
where
    E: Eval + 'static,
    E::Output: AnyExtractCoreItem,
{
}

impl<E> IntoAnimNode for E
where
    E: Eval + 'static,
    E::Output: AnyExtractCoreItem,
{
    fn into_anim_node(self) -> AnimNode {
        // Constant evaluators become static snapshot cells: replayed from a
        // seal-time capture instead of evaluated per frame (and the arena
        // walk replays the snapshot with zero allocations).
        if let Some(items) = Eval::capture_static(&self) {
            return crate::animation::node::static_cell(items, 0.0..1.0);
        }
        AnimNode {
            content: NodeContent::Leaf(Box::new(self)),
            internal_time_secs: 1.0,
            anim_name: type_name::<E>(),
            rate_func: None,
            time_range: 0.0..1.0,
            enabled: true,
        }
    }
}

/// Playback parameters applied to an animation definition.
#[derive(Debug, Clone)]
pub(crate) struct AnimationParam {
    /// Time remapping function; `None` is the identity (linear) rate, kept
    /// structural so the mixing descent can compose affine maps.
    pub rate_func: Option<fn(f64) -> f64>,
    /// Optional duration override in seconds.
    pub duration_secs: Option<f64>,
    /// Whether this animation contributes a value.
    pub enabled: bool,
}

impl Default for AnimationParam {
    fn default() -> Self {
        Self {
            rate_func: None,
            duration_secs: None,
            enabled: true,
        }
    }
}

/// An animation definition with overridden playback parameters.
pub struct Paramed<A> {
    inner: A,
    param: AnimationParam,
}

impl<A> Paramed<A> {
    /// Wrap an animation without overriding its duration.
    pub(crate) fn new(inner: A) -> Self {
        Self {
            inner,
            param: AnimationParam::default(),
        }
    }

    /// Change the animation's rate function.
    pub fn with_rate_func(mut self, rate_func: fn(f64) -> f64) -> Self {
        self.param.rate_func = Some(rate_func);
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
        self.param.enabled = enabled;
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
        cell.rate_func = self.param.rate_func;
        cell.enabled = self.param.enabled;
        cell.anim_name = type_name::<A>();
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
