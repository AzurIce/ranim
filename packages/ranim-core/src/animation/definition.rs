//! Parameter-based animation construction, separate from prepared evaluators.

use super::{
    build::{IntoAnimNode, Unplaced},
    eval::Eval,
    node::AnimNode,
};

/// An animation operation's editable parameters, independent of its input item.
pub trait AnimRecipe<T> {
    /// Prepared execution data. Rebuilt after the input or parameters change.
    type Animation: Eval<Output = T>;

    /// Prepare an evaluator without modifying the input or parameters.
    fn build(&self, input: &T) -> Self::Animation;

    /// Default parent-time duration for this operation, in seconds.
    ///
    /// Unlike `Eval`, implementing this trait explicitly opts into this policy.
    fn default_duration_secs(&self, _input: &T) -> f64 {
        1.0
    }
    /// Compute the terminal item, independent of outer playback remapping.
    ///
    /// Override to avoid preparing an evaluator when the terminal value can be
    /// constructed directly from the input and parameters.
    fn end_value(&self, input: &T) -> T {
        self.build(input).eval_alpha(1.0)
    }
}

/// An animation definition retaining its original input and editable parameters.
///
/// No prepared evaluator or simulation cache is stored here. `IntoAnimNode`
/// prepares execution data once when the definition enters the runtime tree.
#[derive(Clone)]
pub struct Anim<T, P> {
    /// Original input, not the terminal state written by `anim_mut`.
    pub input: T,
    /// Editable operation parameters.
    pub params: P,
}

impl<T, P: AnimRecipe<T>> Anim<T, P> {
    /// Prepare execution data from the current input and parameters.
    pub fn build(&self) -> P::Animation {
        self.params.build(&self.input)
    }
}

impl<T, P> Unplaced for Anim<T, P>
where
    T: Clone + Send + Sync + 'static,
    P: AnimRecipe<T> + 'static,
    P::Animation: 'static,
{
}

impl<T, P> IntoAnimNode for Anim<T, P>
where
    T: Clone + Send + Sync + 'static,
    P: AnimRecipe<T> + 'static,
    P::Animation: 'static,
{
    fn into_anim_node(self) -> AnimNode {
        let duration = self.params.default_duration_secs(&self.input);
        let mut node = AnimNode::from_eval(self.build(), duration);
        // Introspection identifies the author-facing operation, not its cache.
        node.anim_name = std::any::type_name::<P>();
        node
    }
}

/// Bind items to animation operations using owned, cloned, or write-back input.
pub trait ItemAnimExt: Sized {
    /// Retain this owned input and the operation parameters.
    fn anim<P: AnimRecipe<Self>>(self, params: P) -> Anim<Self, P> {
        Anim {
            input: self,
            params,
        }
    }

    /// Clone the input without changing it.
    fn anim_ref<P: AnimRecipe<Self>>(&self, params: P) -> Anim<Self, P>
    where
        Self: Clone,
    {
        Anim {
            input: Self::clone(self),
            params,
        }
    }

    /// Clone the original input, then write the operation's final state back.
    fn anim_mut<P: AnimRecipe<Self>>(&mut self, params: P) -> Anim<Self, P>
    where
        Self: Clone,
    {
        let animation = Anim {
            input: Self::clone(self),
            params,
        };
        *self = animation.params.end_value(&animation.input);
        animation
    }
}

impl<T> ItemAnimExt for T {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{RanimScene, animation::build::PlaybackExt, frame::EvaluatedFrame};
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    struct Add {
        delta: f64,
        builds: Arc<AtomicUsize>,
    }
    struct AddEval {
        input: f64,
        delta: f64,
    }
    impl Eval for AddEval {
        type Output = f64;
        fn eval_alpha(&self, alpha: f64) -> f64 {
            self.input + self.delta * alpha
        }
    }
    impl AnimRecipe<f64> for Add {
        type Animation = AddEval;
        fn build(&self, input: &f64) -> AddEval {
            self.builds.fetch_add(1, Ordering::Relaxed);
            AddEval {
                input: *input,
                delta: self.delta,
            }
        }
        fn default_duration_secs(&self, _input: &f64) -> f64 {
            3.0
        }
        fn end_value(&self, input: &f64) -> f64 {
            input + self.delta
        }
    }
    fn add(delta: f64, builds: &Arc<AtomicUsize>) -> Add {
        Add {
            delta,
            builds: Arc::clone(builds),
        }
    }

    #[test]
    fn owned_borrowed_and_write_back_inputs_have_distinct_semantics() {
        let builds = Arc::new(AtomicUsize::new(0));
        let mut input = 2.0;
        let owned = input.anim(add(4.0, &builds));
        let input_ref = &input;
        let borrowed = input_ref.anim_ref(add(4.0, &builds));
        let input_mut = &mut input;
        let write_back = input_mut.anim_mut(add(4.0, &builds));
        assert_eq!(input, 6.0);
        assert_eq!(owned.input, 2.0);
        assert_eq!(borrowed.input, 2.0);
        assert_eq!(write_back.input, 2.0);
        assert_eq!(builds.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn edits_rebuild_derived_state_and_lowering_prepares_only_once() {
        let builds = Arc::new(AtomicUsize::new(0));
        let mut definition = 2.0.anim(add(4.0, &builds)).with_duration(2.0);
        definition.inner_mut().params.delta = 8.0;
        assert_eq!(definition.inner().build().eval_alpha(0.5), 6.0);
        assert_eq!(builds.load(Ordering::Relaxed), 1);
        let mut scene = RanimScene::new();
        scene.play(definition);
        let scene = scene.seal();
        assert_eq!(builds.load(Ordering::Relaxed), 2);
        assert_eq!(scene.total_secs(), 2.0);
        assert!(scene.get_animation_infos()[0].anim_name.ends_with("::Add"));
        for t in [0.0, 1.0, 2.0] {
            let mut frame = EvaluatedFrame::new();
            scene.eval_at_sec(t, &mut frame);
            assert_eq!(frame.items::<f64>()[0].value, 2.0 + 4.0 * t);
        }
        assert_eq!(builds.load(Ordering::Relaxed), 2);
    }

    #[test]
    fn operation_defaults_apply_without_overrides() {
        let builds = Arc::new(AtomicUsize::new(0));
        let node = 2.0.anim(add(4.0, &builds)).into_anim_node();
        assert_eq!(node.duration_secs(), 3.0);
    }

    #[test]
    fn anim_recipe_alone_supports_write_back_using_default_terminal_evaluation() {
        struct DefaultEnd(Add);
        impl AnimRecipe<f64> for DefaultEnd {
            type Animation = AddEval;
            fn build(&self, input: &f64) -> AddEval {
                self.0.build(input)
            }
        }
        let builds = Arc::new(AtomicUsize::new(0));
        let mut input = 2.0;
        let animation = input.anim_mut(DefaultEnd(add(4.0, &builds)));
        assert_eq!(input, 6.0);
        assert_eq!(animation.input, 2.0);
        assert_eq!(builds.load(Ordering::Relaxed), 1);
        assert_eq!(animation.build().eval_alpha(0.5), 4.0);
    }
}
