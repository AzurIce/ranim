//! Retained definition trees, separate from prepared runtime animation nodes.
//!
//! [`AnimDef`] pairs an input snapshot with a recipe. [`AnimDefNode`] keeps
//! those pairs behind [`DynAnimDef`] at its leaves and owns child definitions
//! at its branches. Building borrows the entire tree, preparing a fresh
//! [`AnimNode`] without replacing any definition with execution data.
//!
//! ```
//! use ranim_core::animation::{
//!     AnimRecipe,
//!     def_node::{AnimDef, AnimDefNode, AnimDefNodeContent},
//!     eval::Static,
//!     node::AnimNode,
//! };
//!
//! struct HoldNumber { duration: f64 }
//! impl AnimRecipe<f64> for HoldNumber {
//!     fn build(&self, initial: &f64) -> AnimNode {
//!         AnimNode::from_eval(Static(*initial), self.duration)
//!     }
//! }
//! let mut definition = AnimDefNode::from_recipe(2.0, HoldNumber { duration: 3.0 });
//! let first = definition.build();
//! if let AnimDefNodeContent::Leaf(leaf) = &mut definition.content {
//!     leaf.downcast_mut::<AnimDef<f64, HoldNumber>>().unwrap().recipe.duration = 5.0;
//! }
//! let second = definition.build();
//! assert_eq!(first.duration_secs(), 3.0);
//! assert_eq!(second.duration_secs(), 5.0);
//! ```
//!
//! Playback builders are defined in [`super::build`]: `AnimDef` and `AnimDefNode`
//! implement [`Unplaced`](super::build::Unplaced), parameter builders produce
//! [`Paramed`](super::build::Paramed), and [`At`](super::build::At)
//! fixes placement. Converting any of these to `AnimDefNode` retains the
//! definition. Placement can temporarily build a node to resolve its natural window.

use std::{any::Any, ops::Range};

use super::{
    AnimRecipe, AnimWriteBack,
    node::{AnimNode, NodeContent},
};

/// An input snapshot paired with a recipe accepting that input.
///
/// Construction imposes no relationship between input and output types and
/// requires neither cloning nor runtime evaluation capabilities on the input.
#[derive(Clone)]
pub struct AnimDef<T, R> {
    /// The input state captured when this definition was created.
    pub initial: T,
    /// Parameters used to prepare execution state.
    pub recipe: R,
}

impl<T, R: AnimRecipe<T>> AnimDef<T, R> {
    /// Bind an owned input snapshot to a recipe.
    pub fn new(initial: T, recipe: R) -> Self {
        Self { initial, recipe }
    }

    /// Prepare a fresh runtime instance without consuming the definition.
    pub fn build(&self) -> AnimNode {
        self.recipe.build(&self.initial)
    }
}

/// Bind owned, cloned, or explicitly write-back inputs to retained recipes.
pub trait ItemAnimExt: Sized {
    /// Retain an owned input and its recipe.
    fn anim<R: AnimRecipe<Self>>(self, recipe: R) -> AnimDef<Self, R> {
        AnimDef::new(self, recipe)
    }

    /// Clone the input without changing it.
    fn anim_ref<R: AnimRecipe<Self>>(&self, recipe: R) -> AnimDef<Self, R>
    where
        Self: Clone,
    {
        AnimDef::new(self.clone(), recipe)
    }

    /// Retain the original input and write back the recipe's explicit terminal state.
    ///
    /// Recipes without write-back support still work with `anim` and `anim_ref`:
    ///
    /// ```compile_fail
    /// use ranim_core::{animation::{AnimRecipe, node::AnimNode}, prelude::*};
    /// struct Recipe;
    /// impl AnimRecipe<f64> for Recipe {
    ///     fn build(&self, input: &f64) -> AnimNode {
    ///         AnimNode::from_eval(Static(*input), 1.0)
    ///     }
    /// }
    /// let mut input = 1.0;
    /// let _ = input.anim_mut(Recipe);
    /// ```
    fn anim_mut<R: AnimWriteBack<Self>>(&mut self, recipe: R) -> AnimDef<Self, R>
    where
        Self: Clone,
    {
        let definition = AnimDef::new(self.clone(), recipe);
        *self = definition.recipe.end_value(&definition.initial);
        definition
    }
}

impl<T> ItemAnimExt for T {}

impl<T: 'static, R: AnimRecipe<T> + 'static> From<AnimDef<T, R>> for AnimDefNode {
    fn from(definition: AnimDef<T, R>) -> Self {
        Self::leaf(definition)
    }
}

/// Type-erased definition, retaining everything needed to rebuild runtime state.
///
/// This is an execution-construction interface, not a serialization or field
/// reflection guarantee. An erased [`AnimDef`] can be downcast for typed editing.
pub trait DynAnimDef: Any {
    /// Concrete author-facing recipe or definition type, before erasure.
    fn anim_name(&self) -> &'static str {
        std::any::type_name::<Self>()
    }
    /// Prepare a fresh runtime animation without consuming this definition.
    fn build(&self) -> AnimNode;
}

impl<T: 'static, R: AnimRecipe<T> + 'static> DynAnimDef for AnimDef<T, R> {
    fn anim_name(&self) -> &'static str {
        std::any::type_name::<R>()
    }
    fn build(&self) -> AnimNode {
        AnimDef::build(self)
    }
}

impl dyn DynAnimDef {
    /// Borrow the concrete definition, including its input and recipe.
    pub fn downcast_ref<D: Any>(&self) -> Option<&D> {
        (self as &dyn Any).downcast_ref()
    }

    /// Edit the concrete definition before constructing the next runtime instance.
    pub fn downcast_mut<D: Any>(&mut self) -> Option<&mut D> {
        (self as &mut dyn Any).downcast_mut()
    }
}

/// Definition-tree content, before any runtime expansion or preparation.
pub enum AnimDefNodeContent {
    /// Already placed child definitions in this node's local content coordinates.
    Children(Vec<AnimDefNode>),
    /// A retained input/recipe pair that may build any kind of runtime subtree.
    Leaf(Box<dyn DynAnimDef>),
}

/// An explicit definition-layer override of a recipe's progress mapping.
#[derive(Clone, Copy, Debug)]
pub enum AnimDefRate {
    /// Use the structural identity mapping, including when clearing a recipe's rate.
    Linear,
    /// Remap normalized parent progress before evaluating the content.
    Function(fn(f64) -> f64),
}

/// A retained authoring node that can repeatedly build a runtime [`AnimNode`].
///
/// Children carry explicit local placements, not sequence or stagger rules.
/// Editing one child does not move its siblings. Optional playback fields are
/// overrides: an unspecified field preserves the recipe's own default.
/// Containers retain their authored spans, including trailing gaps.
pub struct AnimDefNode {
    /// Retained child definitions or an erased complete leaf definition.
    pub content: AnimDefNodeContent,
    /// Optional parent-time window. Changing it remaps content, not child placement.
    pub time_range: Option<Range<f64>>,
    /// Optional rate override; `None` preserves the recipe's mapping.
    pub rate_func: Option<AnimDefRate>,
    /// Optional enable override; `None` preserves the recipe's value.
    pub enabled: Option<bool>,
    /// Explicit content span for a branch, including authored trailing gaps.
    pub internal_time_secs: Option<f64>,
    /// Optional right-endpoint inclusion override, resolved by sequence composition.
    pub end_inclusive: Option<bool>,
    /// Optional authoring name for containers and generated definitions.
    pub anim_name: Option<&'static str>,
}

impl AnimDefNode {
    /// Retain an erased definition without building execution state.
    pub fn leaf(definition: impl DynAnimDef) -> Self {
        Self {
            content: AnimDefNodeContent::Leaf(Box::new(definition)),
            time_range: None,
            rate_func: None,
            enabled: None,
            internal_time_secs: None,
            end_inclusive: None,
            anim_name: None,
        }
    }

    /// Retain an input snapshot and recipe as a leaf definition.
    pub fn from_recipe<T: 'static, R: AnimRecipe<T> + 'static>(initial: T, recipe: R) -> Self {
        Self::leaf(AnimDef::new(initial, recipe))
    }

    /// Retain children with explicit local placements, without preparing them.
    pub fn children(children: Vec<Self>) -> Self {
        Self {
            content: AnimDefNodeContent::Children(children),
            time_range: None,
            rate_func: None,
            enabled: None,
            internal_time_secs: None,
            end_inclusive: None,
            anim_name: None,
        }
    }

    /// Build a fresh runtime tree, retaining inputs, recipes, and child definitions.
    ///
    /// A branch's natural content span is `0..max(child.time_range.end)`, unless
    /// an authored content span preserves additional gaps.
    /// An explicit window remaps that span without changing its children's times.
    ///
    /// # Panics
    ///
    /// Panics if an effective window or content span is non-finite or has
    /// negative duration.
    pub fn build(&self) -> AnimNode {
        let mut node = match &self.content {
            AnimDefNodeContent::Leaf(definition) => definition.build(),
            AnimDefNodeContent::Children(children) => {
                let children: Vec<_> = children.iter().map(Self::build).collect();
                let duration = children
                    .iter()
                    .map(|node| node.time_range.end)
                    .fold(0.0, f64::max);
                AnimNode {
                    content: NodeContent::Children(children),
                    internal_time_secs: self.internal_time_secs.unwrap_or(duration),
                    rate_func: None,
                    time_range: 0.0..duration,
                    end_inclusive: true,
                    enabled: true,
                    anim_name: std::any::type_name::<Self>(),
                }
            }
        };
        if let Some(window) = &self.time_range {
            node.time_range = window.clone();
        }
        assert_valid_window(&node.time_range);
        if let Some(duration) = self.internal_time_secs {
            assert!(
                duration.is_finite() && duration >= 0.0,
                "content duration must be finite and non-negative"
            );
            node.internal_time_secs = duration;
        }
        if let Some(inclusive) = self.end_inclusive {
            node.end_inclusive = inclusive;
        }
        if let Some(name) = self.anim_name {
            node.anim_name = name;
        }
        if let Some(rate) = self.rate_func {
            node.rate_func = match rate {
                AnimDefRate::Linear => None,
                AnimDefRate::Function(rate) => Some(rate),
            };
        }
        if let Some(enabled) = self.enabled {
            node.enabled = enabled;
        }
        node
    }

    /// Resolve the authored window, temporarily building natural defaults if needed.
    pub fn time_range(&self) -> Range<f64> {
        let window = self
            .time_range
            .clone()
            .unwrap_or_else(|| self.build().time_range());
        assert_valid_window(&window);
        window
    }

    /// Authored duration, including any playback override.
    pub fn duration_secs(&self) -> f64 {
        let window = self.time_range();
        window.end - window.start
    }

    /// Direct child definitions, without expanding a leaf recipe's runtime output.
    pub fn definitions(&self) -> &[Self] {
        match &self.content {
            AnimDefNodeContent::Children(children) => children,
            AnimDefNodeContent::Leaf(_) => &[],
        }
    }

    /// Whether this definition explicitly contains children.
    pub fn is_container(&self) -> bool {
        matches!(self.content, AnimDefNodeContent::Children(_))
    }

    /// Whether this definition is a directly authored sound.
    pub fn is_audio(&self) -> bool {
        matches!(&self.content, AnimDefNodeContent::Leaf(leaf)
            if leaf.downcast_ref::<super::sound::Sound>().is_some())
    }

    /// Author-facing type name, independent of any runtime expansion.
    pub fn anim_name(&self) -> &'static str {
        self.anim_name.unwrap_or_else(|| match &self.content {
            AnimDefNodeContent::Children(_) => std::any::type_name::<Self>(),
            AnimDefNodeContent::Leaf(leaf) => leaf.anim_name(),
        })
    }

    /// Effective content span, resolving a leaf's runtime default when necessary.
    pub fn content_duration_secs(&self) -> f64 {
        self.internal_time_secs
            .unwrap_or_else(|| match &self.content {
                AnimDefNodeContent::Children(children) => children
                    .iter()
                    .map(|child| child.time_range().end)
                    .fold(0.0, f64::max),
                AnimDefNodeContent::Leaf(_) => self.build().internal_time_secs,
            })
    }

    /// Effective enable setting, including the recipe's default.
    pub fn enabled(&self) -> bool {
        self.enabled.unwrap_or_else(|| match &self.content {
            AnimDefNodeContent::Children(_) => true,
            AnimDefNodeContent::Leaf(_) => self.build().enabled(),
        })
    }

    /// Effective progress mapping, including the recipe's default.
    pub fn rate_func(&self) -> fn(f64) -> f64 {
        use crate::utils::rate_functions::linear;
        match self.rate_func {
            Some(AnimDefRate::Linear) => linear,
            Some(AnimDefRate::Function(rate)) => rate,
            None => match &self.content {
                AnimDefNodeContent::Children(_) => linear,
                AnimDefNodeContent::Leaf(_) => self.build().rate_func.unwrap_or(linear),
            },
        }
    }

    /// A leaf's declared simulation step; branches do not have one.
    pub fn sim_step(&self) -> Option<f64> {
        match &self.content {
            AnimDefNodeContent::Children(_) => None,
            AnimDefNodeContent::Leaf(_) => self.build().sim_step(),
        }
    }

    pub(super) fn contains_sec(&self, sec: f64, extent: f64) -> bool {
        let window = self.time_range();
        window.contains(&sec)
            || (self.end_inclusive != Some(false) && sec == extent && sec == window.end)
    }

    /// Resolve and shift the authored window without consuming the definition.
    pub(super) fn shift_by(&mut self, offset_sec: f64) {
        assert!(offset_sec.is_finite(), "definition offset must be finite");
        let window = self.time_range();
        let shifted = window.start + offset_sec..window.end + offset_sec;
        assert_valid_window(&shifted);
        self.time_range = Some(shifted);
    }
}

fn assert_valid_window(window: &Range<f64>) {
    assert!(
        window.start.is_finite()
            && window.end.is_finite()
            && (window.end - window.start).is_finite()
            && window.start <= window.end,
        "definition time range must have finite bounds and a finite non-negative duration"
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        animation::eval::{Eval, OutputContext, Static},
        frame::EvaluatedFrame,
        utils::rate_functions::smooth,
    };
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    struct Progress {
        delta: f64,
        duration: f64,
        builds: Arc<AtomicUsize>,
    }
    struct ProgressEval {
        initial: f64,
        delta: f64,
    }
    impl Eval for ProgressEval {
        type Output = f64;
        fn eval_alpha(&self, alpha: f64) -> f64 {
            self.initial + self.delta * alpha
        }
    }
    impl AnimRecipe<f64> for Progress {
        fn build(&self, input: &f64) -> AnimNode {
            self.builds.fetch_add(1, Ordering::Relaxed);
            AnimNode::from_eval(
                ProgressEval {
                    initial: *input,
                    delta: self.delta,
                },
                self.duration,
            )
        }
    }
    impl AnimWriteBack<f64> for Progress {
        fn end_value(&self, input: &f64) -> f64 {
            input + self.delta
        }
    }
    fn progress(builds: &Arc<AtomicUsize>) -> AnimDefNode {
        AnimDefNode::from_recipe(
            2.0,
            Progress {
                delta: 4.0,
                duration: 2.0,
                builds: Arc::clone(builds),
            },
        )
    }
    fn values(node: &AnimNode, sec: f64) -> Vec<f64> {
        let mut frame = EvaluatedFrame::new();
        node.eval_at(sec, &mut OutputContext::new(&mut frame, 0));
        frame
            .items::<f64>()
            .iter()
            .map(|value| value.value)
            .collect()
    }

    #[test]
    fn definitions_are_retained_and_rebuild_edited_runtime_state() {
        let builds = Arc::new(AtomicUsize::new(0));
        let mut definition = progress(&builds);
        let AnimDefNodeContent::Leaf(leaf) = &definition.content else {
            panic!()
        };
        let typed = leaf.downcast_ref::<AnimDef<f64, Progress>>().unwrap();
        assert_eq!((typed.initial, typed.recipe.delta), (2.0, 4.0));
        assert_eq!(builds.load(Ordering::Relaxed), 0);

        let first = definition.build();
        let AnimDefNodeContent::Leaf(leaf) = &mut definition.content else {
            panic!()
        };
        let pair = leaf.downcast_mut::<AnimDef<f64, Progress>>().unwrap();
        pair.initial = 10.0;
        pair.recipe.delta = 8.0;
        pair.recipe.duration = 4.0;
        let second = definition.build();

        assert_eq!(first.duration_secs(), 2.0);
        assert_eq!(second.duration_secs(), 4.0);
        assert_eq!(values(&first, 1.0), vec![4.0]);
        assert_eq!(values(&second, 2.0), vec![14.0]);
        assert_eq!(builds.load(Ordering::Relaxed), 2);
    }

    #[test]
    fn owned_borrowed_and_write_back_inputs_remain_snapshots_after_sealing() {
        use crate::{RanimScene, animation::build::PlaybackExt};
        let builds = Arc::new(AtomicUsize::new(0));
        let recipe = || Progress {
            delta: 4.0,
            duration: 2.0,
            builds: Arc::clone(&builds),
        };
        let mut input = 2.0;
        let owned = input.anim(recipe());
        let borrowed = input.anim_ref(recipe());
        let write_back = input.anim_mut(recipe());
        assert_eq!(input, 6.0);
        assert_eq!(
            (owned.initial, borrowed.initial, write_back.initial),
            (2.0, 2.0, 2.0)
        );
        assert_eq!(builds.load(Ordering::Relaxed), 0);
        let mut scene = RanimScene::new();
        scene.play(write_back.with_duration(2.0));
        let scene = scene.seal();
        let AnimDefNodeContent::Leaf(leaf) = &scene.definition().definitions()[0].content else {
            panic!()
        };
        let retained = leaf.downcast_ref::<AnimDef<f64, Progress>>().unwrap();
        assert_eq!((retained.initial, retained.recipe.delta), (2.0, 4.0));
        let mut frame = EvaluatedFrame::new();
        scene.eval_at_sec(1.0, &mut frame);
        assert_eq!(frame.items::<f64>()[0].value, 4.0);
        assert_eq!(builds.load(Ordering::Relaxed), 1);
    }

    struct Configured;
    impl AnimRecipe<()> for Configured {
        fn build(&self, _: &()) -> AnimNode {
            let mut node = AnimNode::from_eval(Static(1.0), 7.0);
            node.time_range = 2.0..9.0;
            node.rate_func = Some(smooth);
            node.enabled = false;
            node.end_inclusive = false;
            node.anim_name = "configured";
            node
        }
    }

    #[test]
    fn unset_overrides_keep_recipe_defaults_and_linear_rate_clears_them() {
        let mut definition = AnimDefNode::from_recipe((), Configured);
        let runtime = definition.build();
        assert_eq!(runtime.time_range(), 2.0..9.0);
        assert!(!runtime.enabled());
        assert_eq!(runtime.rate_func.unwrap()(0.25), smooth(0.25));
        assert_eq!(runtime.anim_name(), "configured");

        definition.time_range = Some(10.0..24.0);
        definition.rate_func = Some(AnimDefRate::Linear);
        definition.enabled = Some(true);
        let runtime = definition.build();
        assert_eq!(runtime.time_range(), 10.0..24.0);
        assert!(runtime.rate_func.is_none());
        assert!(runtime.enabled());
        assert!(!runtime.end_inclusive);
    }

    #[test]
    fn branch_windows_remap_content_without_moving_child_placement() {
        let builds = Arc::new(AtomicUsize::new(0));
        let mut first = progress(&builds);
        first.time_range = Some(1.0..3.0);
        let mut second = progress(&builds);
        second.time_range = Some(4.0..6.0);
        let mut branch = AnimDefNode::children(vec![first, second]);
        branch.time_range = Some(10.0..22.0);

        let runtime = branch.build();
        assert_eq!(runtime.internal_time_secs, 6.0);
        assert_eq!(runtime.children()[0].time_range(), 1.0..3.0);
        assert_eq!(runtime.children()[1].time_range(), 4.0..6.0);
        assert_eq!(values(&runtime, 14.0), vec![4.0]); // remapped to content time 2
    }

    #[test]
    fn recipe_output_does_not_replace_its_definition() {
        struct Expand;
        impl AnimRecipe<()> for Expand {
            fn build(&self, _: &()) -> AnimNode {
                AnimDefNode::children(vec![AnimDefNode::from_recipe((), Configured)]).build()
            }
        }
        let definition = AnimDefNode::from_recipe((), Expand);
        assert_eq!(definition.build().children().len(), 1);
        assert!(matches!(definition.content, AnimDefNodeContent::Leaf(_)));
    }

    #[test]
    fn invalid_windows_are_rejected_and_negative_windows_are_allowed() {
        for window in [
            1.0..0.0,
            f64::NAN..1.0,
            0.0..f64::INFINITY,
            -f64::MAX..f64::MAX,
        ] {
            let mut definition = AnimDefNode::from_recipe((), Configured);
            definition.time_range = Some(window);
            assert!(
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| definition.build()))
                    .is_err()
            );
        }
        let mut definition = AnimDefNode::from_recipe((), Configured);
        definition.time_range = Some(-2.0..0.0);
        assert_eq!(definition.build().time_range(), -2.0..0.0);
    }
}
