//! User-facing composition helpers built on the core node language.
//!
//! `AnimSequence`, `AnimStack`, and `AnimLagged` are sugar/elaboration
//! constructors: they own their child definitions and lower to the closed
//! [`crate::animation::node::NodeContent`] vocabulary in
//! [`crate::animation::def_node::AnimDefNode::build`]. The primitives they
//! lower to stay in core so every interpreter (visual eval, audio bake, preview
//! info) can see the same structure.
//!
pub mod lagged;
pub mod sequence;
pub mod stack;

use crate::animation::compose::{lagged::AnimLagged, sequence::AnimSequence, stack::AnimStack};
use crate::animation::{build::Unplaced, def_node::AnimDefNode};

/// Collect iterators of animations into containers.
pub trait AnimIterExt: Iterator + Sized {
    /// Collect the animations into an [`AnimStack`] (all at the same origin).
    fn into_stack(self) -> AnimStack
    where
        Self::Item: Into<AnimDefNode>,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        animation::{
            AnimRecipe,
            def_node::{AnimDef, AnimDefNodeContent},
            eval::{OutputContext, Static},
            node::AnimNode,
        },
        frame::EvaluatedFrame,
    };

    #[test]
    fn holds_and_lagged_fills_keep_captured_snapshots_when_sources_are_edited() {
        struct Recipe;
        impl AnimRecipe<f64> for Recipe {
            fn build(&self, initial: &f64) -> AnimNode {
                AnimNode::from_eval(Static(*initial), 2.0)
            }
        }
        fn edit_sources(node: &mut AnimDefNode) {
            match &mut node.content {
                AnimDefNodeContent::Children(children) => {
                    children.iter_mut().for_each(edit_sources)
                }
                AnimDefNodeContent::Leaf(leaf) => {
                    if let Some(definition) = leaf.downcast_mut::<AnimDef<f64, Recipe>>() {
                        definition.initial += 10.0;
                    }
                }
            }
        }
        fn values(node: &AnimNode, sec: f64) -> Vec<f64> {
            let mut frame = EvaluatedFrame::new();
            node.eval_at(sec, &mut OutputContext::new(&mut frame, 0));
            frame.items::<f64>().iter().map(|item| item.value).collect()
        }

        let mut sequence = AnimSequence::new();
        sequence.push(AnimDef::new(1.0, Recipe)).hold(1.0);
        edit_sources(&mut sequence.animations[0]);
        let mut lagged = AnimLagged::new(0.5);
        lagged
            .push(AnimDef::new(2.0, Recipe))
            .push(AnimDef::new(3.0, Recipe));
        let mut lagged: AnimDefNode = lagged.into();
        edit_sources(&mut lagged);
        let definition: AnimDefNode = crate::stack![sequence, lagged.at(5.0)].into();

        for runtime in [definition.build(), definition.build()] {
            assert_eq!(values(&runtime, 1.0), vec![11.0]);
            assert_eq!(values(&runtime, 2.5), vec![1.0]);
            assert_eq!(values(&runtime, 5.5), vec![12.0, 3.0]);
            assert_eq!(values(&runtime, 7.5), vec![2.0, 13.0]);
        }
    }
}
