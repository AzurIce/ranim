//! Pure animation evaluation and hierarchical type-erased composition.

use std::{any::type_name, fmt::Debug, ops::Range};

use crate::{
    core_item::{AnyExtractCoreItem, DynItem},
    utils::rate_functions::linear,
};

/// Evaluation protocols and author-facing adapters.
pub mod eval;

use eval::{Eval, EvalDyn, StaticDynItems};
use lagged::AnimLagged;
use sequence::AnimSequence;
use stack::AnimStack;

/// Dynamic lagged (staggered, end-filled) animation container.
pub mod lagged;
/// Dynamic sequential animation container.
pub mod sequence;
/// Dynamic overlay animation container.
pub mod stack;

/// Runtime animation content category used by preview tooling.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnimationInfoKind {
    /// A typed evaluator without animation children.
    Eval,
    /// A sequential animation container.
    Sequence,
    /// An overlay animation container.
    Stack,
    /// A lagged (staggered, end-held) animation container.
    Lagged,
    /// A captured, type-erased static output batch.
    Static,
}

/// Hierarchical runtime animation information used by preview tooling.
#[derive(Clone)]
pub struct AnimationInfo {
    /// Concrete evaluator or container type name.
    pub anim_name: String,
    /// Runtime content category.
    pub kind: AnimationInfoKind,
    /// Time range in the parent content's local coordinates.
    pub range: Range<f64>,
    /// Duration of this node's inner content before outer timing is applied.
    pub content_duration_secs: f64,
    /// Time remapping function applied by this node.
    pub rate_func: fn(f64) -> f64,
    /// Whether this node contributes values during evaluation.
    pub enabled: bool,
    /// Direct animation children in this node's content coordinates.
    pub children: Vec<AnimationInfo>,
}

/// A single runtime animation node in its parent's time coordinates.
pub struct AnimationCell {
    pub(in crate::animation) inner: Box<dyn EvalDyn>,
    pub(in crate::animation) rate_func: fn(f64) -> f64,
    pub(in crate::animation) time_range: Range<f64>,
    pub(in crate::animation) enabled: bool,
    pub(in crate::animation) anim_name: &'static str,
}

impl AnimationCell {
    /// Global or parent-relative time range, depending on its containing plan.
    pub fn time_range(&self) -> Range<f64> {
        self.time_range.clone()
    }

    /// Duration in seconds.
    pub fn duration_secs(&self) -> f64 {
        self.time_range.end - self.time_range.start
    }

    pub(in crate::animation) fn shift_by(&mut self, offset_sec: f64) {
        self.time_range.start += offset_sec;
        self.time_range.end += offset_sec;
    }

    /// Whether this clip contributes a value.
    pub fn enabled(&self) -> bool {
        self.enabled
    }

    /// Concrete evaluator type name captured before erasure.
    pub fn anim_name(&self) -> &str {
        self.anim_name
    }

    /// Whether the given scene time is inside this clip's inclusive range.
    pub fn active_at(&self, sec: f64) -> bool {
        sec >= self.time_range.start && sec <= self.time_range.end
    }

    /// Compute the rate-warped progress in this cell's local coordinates.
    ///
    /// The cell owns the time configuration (start, duration, rate function)
    /// and turns it into a reading; evaluators never see the configuration
    /// itself. A zero-duration cell reports `alpha == 1.0`.
    fn local_alpha(&self, sec: f64) -> f64 {
        let duration = self.duration_secs();
        let raw = if duration == 0.0 {
            1.0
        } else {
            (sec - self.time_range.start) / duration
        };
        (self.rate_func)(raw)
    }

    /// Evaluate this cell at a time point — the ONLY time-management entry.
    ///
    /// Remaps the scene time to this cell's local `alpha` (via its rate
    /// function), then evaluates the inner node at that progress (a pure query
    /// on `&self`). Direction management (forward vs backward reset+replay,
    /// how many `sim_step`s to integrate) is INTERNAL to each stateful node.
    pub(crate) fn eval_at(&self, sec: f64, output: &mut Vec<DynItem>) {
        if !self.enabled || !self.active_at(sec) {
            return;
        }
        let alpha = self.local_alpha(sec);
        self.inner.eval_dyn(alpha, output);
    }

    pub(in crate::animation) fn contains_sec(&self, sec: f64, parent_duration: f64) -> bool {
        self.time_range.contains(&sec) || (sec == parent_duration && sec == self.time_range.end)
    }

    pub(crate) fn animation_info(&self) -> AnimationInfo {
        AnimationInfo {
            anim_name: self.anim_name.to_string(),
            kind: self.inner.info_kind(),
            range: self.time_range.clone(),
            content_duration_secs: self.inner.content_duration_secs(),
            rate_func: self.rate_func,
            enabled: self.enabled,
            children: self.inner.child_infos(),
        }
    }
}

/// A statically typed animation definition that can be lowered into a runtime animation.
pub trait Animation: Sized {
    /// Lower this definition into its local runtime representation.
    fn build(self) -> AnimationCell;
}

/// Capability for animation definitions that have not been fixed in parent time coordinates.
///
/// This trait is used to constrain the anims to be inserted into [`AnimSequence`].
/// Only anims those are not placed can be inserted into it.
pub trait Placeable: Animation {
    /// Place this definition at an offset in its parent's local time coordinates.
    fn at(self, offset_sec: f64) -> At<Self> {
        At {
            inner: self,
            offset_sec,
        }
    }
}

/// Playback parameter builders for animations that have not been placed yet.
pub trait AnimationExt: Placeable {
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

impl<A: Placeable> AnimationExt for A {}

impl<E> Placeable for E
where
    E: Eval + 'static,
    E::Output: AnyExtractCoreItem,
{
}
impl<E> Animation for E
where
    E: Eval + 'static,
    E::Output: AnyExtractCoreItem,
{
    fn build(self) -> AnimationCell {
        AnimationCell {
            anim_name: type_name::<E>(),
            inner: Box::new(self),
            rate_func: linear,
            time_range: 0.0..1.0,
            enabled: true,
        }
    }
}

/// Playback parameters applied to an animation definition.
#[derive(Debug, Clone)]
pub(crate) struct AnimationParam {
    /// Time remapping function.
    pub rate_func: fn(f64) -> f64,
    /// Optional duration override in seconds.
    pub duration_secs: Option<f64>,
    /// Whether this animation contributes a value.
    pub enabled: bool,
}

impl Default for AnimationParam {
    fn default() -> Self {
        Self {
            rate_func: linear,
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
        self.param.rate_func = rate_func;
        self
    }

    /// Change the animation's duration.
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

impl<A: Placeable + 'static> Placeable for Paramed<A> {}
impl<A: Placeable + 'static> Animation for Paramed<A> {
    fn build(self) -> AnimationCell {
        let mut cell = self.inner.build();
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
/// This is a terminal placement entry: it implements [`Animation`] but not
/// [`Placeable`], so playback parameters must be configured before calling
/// [`Placeable::at`].
pub struct At<A> {
    inner: A,
    offset_sec: f64,
}

impl<A: Animation> Animation for At<A> {
    fn build(self) -> AnimationCell {
        let mut animation = self.inner.build();
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

/// Build a static cell replaying already-sampled items over `time_range`.
pub(in crate::animation) fn static_cell(
    state: Vec<DynItem>,
    time_range: Range<f64>,
) -> AnimationCell {
    AnimationCell {
        inner: Box::new(StaticDynItems(state)),
        rate_func: linear,
        time_range,
        enabled: true,
        anim_name: type_name::<StaticDynItems>(),
    }
}

/// Collect iterators of animations into containers.
pub trait AnimIterExt: Iterator + Sized {
    /// Collect the animations into an [`AnimStack`] (all at the same origin).
    fn into_stack(self) -> AnimStack
    where
        Self::Item: Animation + 'static,
    {
        self.collect()
    }

    /// Collect the animations into an [`AnimSequence`] (played in order).
    fn into_seq(self) -> AnimSequence
    where
        Self::Item: Placeable + 'static,
    {
        self.collect()
    }

    /// Collect the animations into an [`AnimLagged`] with the given stagger ratio.
    fn into_lagged(self, lag_ratio: f64) -> AnimLagged
    where
        Self::Item: Placeable + 'static,
    {
        let mut lagged = AnimLagged::new(lag_ratio);
        for animation in self {
            lagged.push(animation);
        }
        lagged
    }
}

impl<I: Iterator> AnimIterExt for I {}

/// Requirement for [`StaticAnim`].
pub trait StaticAnimRequirement: Clone + AnyExtractCoreItem {}

impl<T: Clone + AnyExtractCoreItem> StaticAnimRequirement for T {}

/// Convenience methods for zero-duration static animations.
pub trait StaticAnim: StaticAnimRequirement + Sized {
    /// Show this value.
    fn show(&self) -> Paramed<Static<Self>>;
    /// Hide this value.
    fn hide(&self) -> Paramed<Static<Self>>;
}

impl<T: StaticAnimRequirement + 'static> StaticAnim for T {
    fn show(&self) -> Paramed<Static<Self>> {
        Static(self.clone()).with_duration(0.0)
    }

    fn hide(&self) -> Paramed<Static<Self>> {
        Static(self.clone()).with_enabled(false).with_duration(0.0)
    }
}

/// A constant evaluator.
pub struct Static<T: Clone>(pub T);

impl<T: Clone> Eval for Static<T> {
    type Output = T;

    fn eval_alpha(&self, _alpha: f64) -> Self::Output {
        self.0.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::{
        eval::{Eval, EvalExt, pure::PureFunc},
        lagged::LaggedFill,
        sequence::AnimSequence,
        stack::AnimStack,
    };
    use crate::{Extract, core_item::CoreItem, core_item::vitem::VItem, lagged, seq, stack};

    /// A stateless test double: a `VItem` shifted to a fixed x.
    struct ShiftX(f32);

    impl Eval for ShiftX {
        type Output = VItem;

        fn eval_alpha(&self, _alpha: f64) -> Self::Output {
            let mut item = VItem::default();
            item.points[0].x = self.0;
            item
        }
    }

    /// A stateless test double: x = offset + alpha.
    struct ShiftAlpha(f32);

    impl Eval for ShiftAlpha {
        type Output = VItem;

        fn eval_alpha(&self, alpha: f64) -> Self::Output {
            let mut item = VItem::default();
            item.points[0].x = self.0 + alpha as f32;
            item
        }
    }

    fn leaf(x: f32, duration: f64) -> impl Placeable {
        ShiftX(x).with_duration(duration)
    }

    fn progress_leaf(offset: f32) -> impl Placeable {
        ShiftAlpha(offset)
    }

    fn evaluated_xs(items: Vec<DynItem>) -> Vec<f32> {
        items
            .into_iter()
            .flat_map(|item| item.extract())
            .filter_map(|item| match item {
                CoreItem::VItem(item) => Some(item.points[0].x),
                CoreItem::CameraFrame(_) | CoreItem::MeshItem(_) => None,
            })
            .collect()
    }

    fn sampled_xs(animation: &AnimationCell, sec: f64) -> Vec<f32> {
        let mut items = Vec::new();
        animation.eval_at(sec, &mut items);
        evaluated_xs(items)
    }

    #[test]
    fn at_offsets_the_built_animation() {
        let animation = leaf(1.0, 2.0).at(3.0).build();
        assert_eq!(animation.time_range(), 3.0..5.0);
    }

    #[test]
    fn eval_uses_linear_one_second_defaults() {
        let animation = Static(VItem::default()).build();
        assert_eq!(animation.time_range(), 0.0..1.0);
    }

    #[test]
    fn apply_alpha_to_writes_the_requested_progress_state() {
        let mut item = VItem::default();

        let animation = ShiftAlpha(0.0).apply_alpha_to(&mut item, 0.25);
        assert_eq!(item.points[0].x, 0.25);

        let animation = animation.apply_to(&mut item);
        assert_eq!(item.points[0].x, 1.0);
        assert_eq!(animation.eval_alpha(0.5).points[0].x, 0.5);
    }

    #[test]
    fn pure_func_wraps_a_closure_into_eval() {
        let mut item = VItem::default();
        PureFunc::new(|alpha: f64| {
            let mut item = VItem::default();
            item.points[0].x = alpha as f32;
            item
        })
        .apply_alpha_to(&mut item, 0.5);
        assert_eq!(item.points[0].x, 0.5);
    }

    #[test]
    fn parametrized_sequence_remaps_the_group_timeline() {
        use crate::utils::rate_functions::ease_in_quad;

        let animation = seq![progress_leaf(0.0), progress_leaf(10.0)]
            .with_duration(4.0)
            .with_rate_func(ease_in_quad);
        let animation = animation.build();

        assert_eq!(animation.time_range(), 0.0..4.0);
        assert_eq!(sampled_xs(&animation, 2.0), vec![0.5]);
        let info = animation.animation_info();
        assert_eq!(info.range, 0.0..4.0);
        assert_eq!(info.content_duration_secs, 2.0);
        assert_eq!(info.children.len(), 2);
        assert_eq!(info.children[0].range, 0.0..1.0);
        assert_eq!(info.children[1].range, 1.0..2.0);
    }

    #[test]
    fn seq_uses_child_durations() {
        let sequence = seq![leaf(1.0, 2.0), leaf(2.0, 3.0)];
        assert_eq!(sequence.built_animations()[0].time_range(), 0.0..2.0);
        assert_eq!(sequence.built_animations()[1].time_range(), 2.0..5.0);
        assert_eq!(sequence.at(5.0).build().time_range(), 5.0..10.0);
    }

    #[test]
    fn stack_accepts_plain_and_positioned_children() {
        let animation = stack![leaf(1.0, 2.0), leaf(2.0, 3.0).at(1.0)];
        assert_eq!(animation.duration_secs(), 4.0);
        assert_eq!(animation.built_animations()[0].time_range(), 0.0..2.0);
        assert_eq!(animation.built_animations()[1].time_range(), 1.0..4.0);
        assert_eq!(animation.at(10.0).build().time_range(), 10.0..14.0);
    }

    #[test]
    fn composition_macros_build_dynamic_containers_without_an_arity_limit() {
        let empty_sequence: AnimSequence = seq![];
        let empty_stack: AnimStack = stack![];
        assert_eq!(empty_sequence.duration_secs(), 0.0);
        assert_eq!(empty_stack.duration_secs(), 0.0);

        let sequence: AnimSequence = seq![
            leaf(1.0, 1.0),
            leaf(2.0, 1.0),
            leaf(3.0, 1.0),
            leaf(4.0, 1.0),
            leaf(5.0, 1.0),
            leaf(6.0, 1.0),
            leaf(7.0, 1.0),
            leaf(8.0, 1.0),
            leaf(9.0, 1.0),
        ];
        assert_eq!(sequence.duration_secs(), 9.0);
        assert_eq!(sequence.built_animations().len(), 9);

        let stack: AnimStack = stack![
            leaf(1.0, 1.0),
            leaf(2.0, 2.0),
            leaf(3.0, 3.0),
            leaf(4.0, 4.0),
            leaf(5.0, 5.0),
            leaf(6.0, 6.0),
            leaf(7.0, 7.0),
            leaf(8.0, 8.0),
            leaf(9.0, 9.0),
        ];
        assert_eq!(stack.duration_secs(), 9.0);
        assert_eq!(stack.built_animations().len(), 9);
    }

    #[test]
    fn sequence_can_be_repositioned_after_erasure() {
        let mut sequence = AnimSequence::new();
        sequence
            .push(leaf(1.0, 2.0))
            .forward(1.0)
            .push(leaf(2.0, 1.0));
        let animation = sequence.at(10.0).build();
        let info = animation.animation_info();
        assert_eq!(info.range, 10.0..14.0);
        assert_eq!(info.children[0].range, 0.0..2.0);
        assert_eq!(info.children[1].range, 3.0..4.0);
    }

    #[test]
    fn hold_samples_only_animations_active_before_the_cursor() {
        let mut sequence = AnimSequence::new();
        sequence
            .push(stack![leaf(1.0, 1.0), leaf(2.0, 2.0)])
            .hold(1.0);

        assert_eq!(sequence.built_animations().len(), 2);
        assert_eq!(sampled_xs(&sequence.built_animations()[1], 2.5), vec![2.0]);
    }

    #[test]
    fn repeated_hold_creates_adjacent_static_animations() {
        let mut sequence = AnimSequence::new();
        sequence.push(leaf(3.0, 1.0)).hold(1.0).hold(2.0);

        assert_eq!(sequence.cursor_sec(), 4.0);
        assert_eq!(sequence.built_animations().len(), 3);
        assert_eq!(sequence.built_animations()[1].time_range(), 1.0..2.0);
        assert_eq!(sequence.built_animations()[2].time_range(), 2.0..4.0);
        assert_eq!(sampled_xs(&sequence.built_animations()[2], 3.5), vec![3.0]);
    }

    #[test]
    fn repeated_hold_replays_dyn_items_without_nesting_the_output_batch() {
        let mut sequence = AnimSequence::new();
        sequence
            .push(stack![leaf(1.0, 1.0), leaf(2.0, 1.0)])
            .hold(1.0)
            .hold(1.0);

        let mut first_hold = Vec::new();
        sequence.built_animations()[1].eval_at(1.5, &mut first_hold);
        let mut second_hold = Vec::new();
        sequence.built_animations()[2].eval_at(2.5, &mut second_hold);

        assert_eq!(first_hold.len(), 2);
        assert_eq!(second_hold.len(), 2);
        assert_eq!(evaluated_xs(second_hold), vec![1.0, 2.0]);
    }

    #[test]
    fn forward_does_not_hold_the_previous_state() {
        let mut sequence = AnimSequence::new();
        sequence.push(leaf(4.0, 1.0)).forward(1.0).hold(1.0);

        assert_eq!(sequence.cursor_sec(), 3.0);
        assert_eq!(sequence.built_animations().len(), 1);
    }

    #[test]
    fn hold_uses_the_sequences_final_evaluation() {
        let mut shown = VItem::default();
        shown.points[0].x = 5.0;

        let mut hidden = AnimSequence::new();
        hidden.push(leaf(1.0, 1.0)).push(shown.hide()).hold(1.0);
        assert_eq!(hidden.built_animations().len(), 2);
        let mut hidden_items = Vec::new();
        hidden.built_animations()[1].eval_at(1.0, &mut hidden_items);
        assert!(hidden_items.is_empty());

        let mut restored = AnimSequence::new();
        restored.push(leaf(1.0, 1.0)).push(shown.show()).hold(1.0);
        assert_eq!(sampled_xs(&restored.built_animations()[2], 1.5), vec![5.0]);
    }

    #[test]
    fn nested_sequences_keep_their_own_final_evaluation() {
        let mut shown = VItem::default();
        shown.points[0].x = 7.0;

        let inner = seq![leaf(1.0, 1.0), shown.show()];
        let mut outer = AnimSequence::new();
        outer.push(inner).hold(1.0);

        assert_eq!(outer.built_animations().len(), 2);
        assert_eq!(sampled_xs(&outer.built_animations()[1], 1.5), vec![7.0]);

        let hidden_inner = seq![leaf(1.0, 1.0), shown.hide()];
        let mut hidden_outer = AnimSequence::new();
        hidden_outer.push(hidden_inner).hold(1.0);
        assert_eq!(hidden_outer.built_animations().len(), 1);
    }

    #[test]
    fn dynamic_stack_keeps_children_at_the_same_origin() {
        let mut stack = AnimStack::new();
        stack.push(leaf(1.0, 1.0)).push(leaf(2.0, 3.0));

        assert_eq!(stack.duration_secs(), 3.0);
        assert_eq!(stack.built_animations()[0].time_range(), 0.0..1.0);
        assert_eq!(stack.built_animations()[1].time_range(), 0.0..3.0);
    }

    #[test]
    fn sequence_extend_appends_direct_children_and_preserves_local_gaps() {
        let mut source = AnimSequence::new();
        source
            .push(leaf(2.0, 1.0))
            .forward(2.0)
            .push(leaf(3.0, 1.0));

        let mut sequence = AnimSequence::new();
        sequence.push(leaf(1.0, 2.0)).extend(source);

        assert_eq!(sequence.cursor_sec(), 6.0);
        assert_eq!(sequence.built_animations().len(), 3);
        assert_eq!(sequence.built_animations()[0].time_range(), 0.0..2.0);
        assert_eq!(sequence.built_animations()[1].time_range(), 2.0..3.0);
        assert_eq!(sequence.built_animations()[2].time_range(), 5.0..6.0);
    }

    #[test]
    fn stack_extend_appends_direct_children() {
        let source = stack![leaf(2.0, 1.0), leaf(3.0, 2.0).at(1.0)];
        let mut stack = stack![leaf(1.0, 4.0)];
        stack.extend(source);

        assert_eq!(stack.duration_secs(), 4.0);
        assert_eq!(stack.built_animations().len(), 3);
        assert_eq!(stack.built_animations()[1].time_range(), 0.0..1.0);
        assert_eq!(stack.built_animations()[2].time_range(), 1.0..3.0);
    }

    #[test]
    fn lagged_staggers_children_by_ratio_of_previous_durations() {
        let lagged = lagged![0.5; leaf(1.0, 1.0), leaf(2.0, 2.0), leaf(3.0, 1.0)];
        assert_eq!(lagged.built_animations()[0].time_range(), 0.0..1.0);
        assert_eq!(lagged.built_animations()[1].time_range(), 0.5..2.5);
        assert_eq!(lagged.built_animations()[2].time_range(), 1.5..2.5);
        assert_eq!(lagged.duration_secs(), 2.5);

        // ratio 1.0 is a sequence, ratio 0.0 is a stack.
        let as_sequence = lagged![1.0; leaf(1.0, 1.0), leaf(2.0, 1.0)];
        assert_eq!(as_sequence.built_animations()[1].time_range(), 1.0..2.0);
        let as_stack = lagged![0.0; leaf(1.0, 1.0), leaf(2.0, 2.0)];
        assert_eq!(as_stack.built_animations()[1].time_range(), 0.0..2.0);
        assert_eq!(as_stack.duration_secs(), 2.0);
    }

    #[test]
    fn lagged_fills_window_edges_with_static_cells() {
        let lagged = lagged![
            0.5;
            progress_leaf(0.0).with_duration(1.0),
            progress_leaf(10.0).with_duration(1.0),
        ];
        let animation = lagged.build();
        assert_eq!(animation.time_range(), 0.0..1.5);

        // Before the second child's start: it shows its initial state (Hold leading).
        assert_eq!(sampled_xs(&animation, 0.25), vec![0.25, 10.0]);
        // After the first child's end: it holds its final state (Hold trailing).
        assert_eq!(sampled_xs(&animation, 1.0), vec![1.0, 10.5]);
        // At the container's end: both hold their final states.
        assert_eq!(sampled_xs(&animation, 1.5), vec![1.0, 11.0]);
    }

    #[test]
    fn lagged_with_empty_leading_renders_nothing_before_start() {
        let animation = lagged![
            0.5;
            progress_leaf(0.0).with_duration(1.0),
            progress_leaf(10.0).with_duration(1.0),
        ]
        .with_leading(LaggedFill::Empty)
        .build();

        // Before the second child's start: only the first renders.
        assert_eq!(sampled_xs(&animation, 0.25), vec![0.25]);
        // Trailing still holds by default.
        assert_eq!(sampled_xs(&animation, 1.0), vec![1.0, 10.5]);
    }

    #[test]
    fn lagged_fill_structure_is_materialized_as_per_item_tracks() {
        let animation = lagged![
            0.5;
            progress_leaf(0.0).with_duration(1.0),
            progress_leaf(10.0).with_duration(1.0),
        ]
        .build();
        let info = animation.animation_info();
        // Each item becomes a sequence track spanning the whole extent:
        // [leading fill][anim][trailing fill] (empty fills skipped).
        assert_eq!(info.children.len(), 2);

        let first = &info.children[0];
        assert_eq!(first.kind, AnimationInfoKind::Sequence);
        assert_eq!(first.range, 0.0..1.5);
        assert_eq!(first.children.len(), 2);
        assert_eq!(first.children[0].kind, AnimationInfoKind::Eval);
        assert_eq!(first.children[0].range, 0.0..1.0);
        assert_eq!(first.children[1].kind, AnimationInfoKind::Static);
        assert_eq!(first.children[1].range, 1.0..1.5);

        let second = &info.children[1];
        assert_eq!(second.kind, AnimationInfoKind::Sequence);
        assert_eq!(second.range, 0.0..1.5);
        assert_eq!(second.children.len(), 2);
        assert_eq!(second.children[0].kind, AnimationInfoKind::Static);
        assert_eq!(second.children[0].range, 0.0..0.5);
        assert_eq!(second.children[1].kind, AnimationInfoKind::Eval);
        assert_eq!(second.children[1].range, 0.5..1.5);
    }

    #[test]
    fn lagged_child_ending_with_hide_stays_hidden_after_its_window() {
        let mut shown = VItem::default();
        shown.points[0].x = 2.0;

        let animation = lagged![0.5; seq![leaf(2.0, 1.0), shown.hide()]].build();
        assert_eq!(animation.time_range(), 0.0..1.0);

        assert_eq!(sampled_xs(&animation, 0.5), vec![2.0]);
        // The seq ends with a hide cell: after the window the item stays hidden.
        let mut items = Vec::new();
        animation.eval_at(1.0, &mut items);
        assert!(items.is_empty());
    }

    #[test]
    fn animations_collect_into_containers() {
        let stack: AnimStack = vec![leaf(1.0, 1.0), leaf(2.0, 2.0)].into_iter().collect();
        assert_eq!(stack.duration_secs(), 2.0);
        assert_eq!(stack.built_animations()[1].time_range(), 0.0..2.0);

        let sequence: AnimSequence = vec![leaf(1.0, 1.0), leaf(2.0, 2.0)].into_iter().collect();
        assert_eq!(sequence.duration_secs(), 3.0);
        assert_eq!(sequence.built_animations()[1].time_range(), 1.0..3.0);

        let lagged = vec![leaf(1.0, 1.0), leaf(2.0, 1.0)]
            .into_iter()
            .into_lagged(0.5);
        assert_eq!(lagged.built_animations()[1].time_range(), 0.5..1.5);
        assert_eq!(lagged.duration_secs(), 1.5);

        let stack = vec![leaf(1.0, 2.0)].into_iter().into_stack();
        assert_eq!(stack.duration_secs(), 2.0);
        let sequence = vec![leaf(1.0, 1.0), leaf(2.0, 1.0)].into_iter().into_seq();
        assert_eq!(sequence.duration_secs(), 2.0);
    }
}
