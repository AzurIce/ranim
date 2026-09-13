//! User-facing composition helpers built on the core node language.
//!
//! `AnimSequence`, `AnimStack`, and `AnimLagged` are sugar/elaboration
//! constructors: they own their child definitions and lower to the closed
//! [`crate::animation::node::NodeContent`] vocabulary in
//! [`crate::animation::build::IntoAnimNode::into_anim_node`]. The primitives they
//! lower to stay in core so every interpreter (visual eval, audio bake, preview
//! info) can see the same structure.
//!
pub mod lagged;
pub mod sequence;
pub mod stack;

use crate::animation::build::{IntoAnimNode, Unplaced};
use crate::animation::compose::{lagged::AnimLagged, sequence::AnimSequence, stack::AnimStack};

/// Collect iterators of animations into containers.
pub trait AnimIterExt: Iterator + Sized {
    /// Collect the animations into an [`AnimStack`] (all at the same origin).
    fn into_stack(self) -> AnimStack
    where
        Self::Item: IntoAnimNode + 'static,
    {
        self.collect()
    }

    /// Collect the animations into an [`AnimSequence`] (played in order).
    fn into_seq(self) -> AnimSequence
    where
        Self::Item: Unplaced + 'static,
    {
        self.collect()
    }

    /// Collect the animations into an [`AnimLagged`] with the given stagger ratio.
    fn into_lagged(self, lag_ratio: f64) -> AnimLagged
    where
        Self::Item: Unplaced + 'static,
    {
        let mut lagged = AnimLagged::new(lag_ratio);
        for animation in self {
            lagged.push(animation);
        }
        lagged
    }
}

impl<I: Iterator> AnimIterExt for I {}
