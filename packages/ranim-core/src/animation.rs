//! Pure animation evaluation, static composition, and type-erased built clips.

use std::{any::type_name, fmt::Debug, marker::PhantomData, ops::Range};

use crate::{
    core_item::{AnyExtractCoreItem, DynItem},
    utils::rate_functions::linear,
};

/// A normalized, pure animation evaluator for values of type `T`.
pub trait Eval<T> {
    /// Evaluate the animation at a normalized progress in `[0, 1]`.
    fn eval_alpha(&self, alpha: f64) -> T;

    /// Wrap this evaluator in a typed animation with default timing.
    fn into_animation_cell(self) -> AnimationCell<T, Self>
    where
        Self: Sized + 'static,
    {
        AnimationCell {
            inner: self,
            param: EvalParam::default(),
            anim_name: type_name::<Self>(),
            _output: PhantomData,
        }
    }

    /// Erase both the evaluator and its output type for built storage.
    fn into_erased_boxed(self) -> Box<dyn EvalDyn>
    where
        T: AnyExtractCoreItem,
        Self: Sized + 'static,
    {
        Box::new(EvalDynAdapter::<T, Self> {
            inner: self,
            _output: PhantomData,
        })
    }
}

impl<T, F> Eval<T> for F
where
    F: Fn(f64) -> T,
{
    fn eval_alpha(&self, alpha: f64) -> T {
        (self)(alpha)
    }
}

/// Type-erased normalized evaluation used by [`BuiltAnimation`].
pub trait EvalDyn {
    /// Append type-erased scene items evaluated at `alpha` to `output`.
    fn eval_alpha_dyn_into(&self, alpha: f64, output: &mut Vec<DynItem>);

    /// Evaluate into a flat collection of type-erased scene items.
    fn eval_alpha_dyn(&self, alpha: f64) -> Vec<DynItem> {
        let mut output = Vec::new();
        self.eval_alpha_dyn_into(alpha, &mut output);
        output
    }
}

struct EvalDynAdapter<T, E> {
    inner: E,
    _output: PhantomData<fn() -> T>,
}

impl<T, E> EvalDyn for EvalDynAdapter<T, E>
where
    T: AnyExtractCoreItem,
    E: Eval<T>,
{
    fn eval_alpha_dyn_into(&self, alpha: f64, output: &mut Vec<DynItem>) {
        output.push(DynItem(Box::new(self.inner.eval_alpha(alpha))));
    }
}

/// Local timing applied to a normalized evaluator.
#[derive(Debug, Clone)]
pub struct EvalParam {
    /// Time remapping function.
    pub rate_func: fn(f64) -> f64,
    /// Duration in seconds.
    pub duration_secs: f64,
    /// Whether this animation contributes a value.
    pub enabled: bool,
}

impl Default for EvalParam {
    fn default() -> Self {
        Self {
            rate_func: linear,
            duration_secs: 1.0,
            enabled: true,
        }
    }
}

/// A concrete evaluator plus local timing, stored inline until build time.
pub struct AnimationCell<T, E: Eval<T>> {
    inner: E,
    param: EvalParam,
    anim_name: &'static str,
    _output: PhantomData<fn() -> T>,
}

impl<T, E: Eval<T>> AnimationCell<T, E> {
    /// Change the animation's rate function.
    pub fn with_rate_func(mut self, rate_func: fn(f64) -> f64) -> Self {
        self.param.rate_func = rate_func;
        self
    }

    /// Change the animation's duration.
    pub fn with_duration(mut self, duration_secs: f64) -> Self {
        assert!(
            duration_secs.is_finite() && duration_secs >= 0.0,
            "animation duration must be finite and non-negative"
        );
        self.param.duration_secs = duration_secs;
        self
    }

    /// Enable or disable this animation's output.
    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.param.enabled = enabled;
        self
    }

    /// Animation duration in seconds.
    pub fn duration_secs(&self) -> f64 {
        self.param.duration_secs
    }

    /// Apply the final state and return the animation.
    pub fn apply_to(self, item: &mut T) -> Self {
        self.apply_alpha_to(item, 1.0)
    }

    /// Apply a sampled state and return the animation.
    pub fn apply_alpha_to(self, item: &mut T, alpha: f64) -> Self {
        *item = self.eval_alpha(alpha);
        self
    }
}

impl<T, E: Eval<T>> Eval<T> for AnimationCell<T, E> {
    fn eval_alpha(&self, alpha: f64) -> T {
        self.inner.eval_alpha((self.param.rate_func)(alpha))
    }
}

/// A statically typed animation definition that can be flattened into built clips.
pub trait Animation: Sized {
    /// Range in this animation's local time coordinates.
    fn time_range(&self) -> Range<f64>;

    /// Build using `origin_sec` as the global origin of the local coordinates.
    fn build(self, origin_sec: f64, output: &mut Vec<BuiltAnimation>);

    /// Total local extent used when advancing a sequence cursor.
    fn duration_secs(&self) -> f64 {
        let range = self.time_range();
        range.end.max(0.0)
    }
}

/// Capability for animation definitions that may be explicitly positioned with [`Placeable::at`].
pub trait Placeable: Sized {
    /// Place this definition at an offset in its parent's local time coordinates.
    fn at(self, offset_sec: f64) -> At<Self> {
        At {
            inner: self,
            offset_sec,
        }
    }
}

/// A relative placement node used only before flattening.
pub struct At<A> {
    inner: A,
    offset_sec: f64,
}

impl<A: Animation> Animation for At<A> {
    fn time_range(&self) -> Range<f64> {
        let range = self.inner.time_range();
        self.offset_sec + range.start..self.offset_sec + range.end
    }

    fn build(self, origin_sec: f64, output: &mut Vec<BuiltAnimation>) {
        self.inner.build(origin_sec + self.offset_sec, output);
    }
}

impl<T, E> Animation for AnimationCell<T, E>
where
    T: AnyExtractCoreItem,
    E: Eval<T> + 'static,
{
    fn time_range(&self) -> Range<f64> {
        0.0..self.param.duration_secs
    }

    fn build(self, origin_sec: f64, output: &mut Vec<BuiltAnimation>) {
        let duration_secs = self.param.duration_secs;
        output.push(BuiltAnimation {
            inner: BuiltEval::Dynamic(self.inner.into_erased_boxed()),
            rate_func: self.param.rate_func,
            time_range: origin_sec..origin_sec + duration_secs,
            enabled: self.param.enabled,
            anim_name: self.anim_name,
        });
    }
}

impl<T, E> Placeable for AnimationCell<T, E>
where
    T: AnyExtractCoreItem,
    E: Eval<T> + 'static,
{
}

enum BuiltEval {
    Dynamic(Box<dyn EvalDyn>),
    Static(Vec<DynItem>),
}

/// A flattened animation with mutable timing outside its erased evaluator.
pub struct BuiltAnimation {
    inner: BuiltEval,
    rate_func: fn(f64) -> f64,
    time_range: Range<f64>,
    enabled: bool,
    anim_name: &'static str,
}

impl BuiltAnimation {
    /// Global or parent-relative time range, depending on its containing plan.
    pub fn time_range(&self) -> Range<f64> {
        self.time_range.clone()
    }

    /// Duration in seconds.
    pub fn duration_secs(&self) -> f64 {
        self.time_range.end - self.time_range.start
    }

    /// Whether this clip contributes a value.
    pub fn enabled(&self) -> bool {
        self.enabled
    }

    /// Concrete evaluator type name captured before erasure.
    pub fn anim_name(&self) -> &str {
        self.anim_name
    }

    /// Shift both endpoints by an offset.
    pub fn shift_by(&mut self, offset_sec: f64) {
        self.time_range.start += offset_sec;
        self.time_range.end += offset_sec;
    }

    /// Append normalized evaluation results after applying this clip's rate function.
    pub fn eval_alpha_dyn_into(&self, alpha: f64, output: &mut Vec<DynItem>) {
        match &self.inner {
            BuiltEval::Dynamic(inner) => {
                inner.eval_alpha_dyn_into((self.rate_func)(alpha), output);
            }
            BuiltEval::Static(items) => output.extend(items.iter().cloned()),
        }
    }

    /// Evaluate normalized progress into a flat collection of scene items.
    pub fn eval_alpha_dyn(&self, alpha: f64) -> Vec<DynItem> {
        let mut output = Vec::new();
        self.eval_alpha_dyn_into(alpha, &mut output);
        output
    }

    /// Evaluate at a time contained in this clip's inclusive range.
    pub fn eval_at_sec(&self, sec: f64) -> Option<Vec<DynItem>> {
        if sec < self.time_range.start || sec > self.time_range.end {
            return None;
        }
        let duration = self.duration_secs();
        let alpha = if duration == 0.0 {
            1.0
        } else {
            (sec - self.time_range.start) / duration
        };
        Some(self.eval_alpha_dyn(alpha))
    }

    fn eval_left_at_sec_into(&self, sec: f64, output: &mut Vec<DynItem>) {
        let is_zero_duration_at_sec = self.time_range.start == sec && self.time_range.end == sec;
        let is_active_before_sec = self.time_range.start < sec && sec <= self.time_range.end;
        if !self.enabled || (!is_zero_duration_at_sec && !is_active_before_sec) {
            return;
        }

        let duration = self.duration_secs();
        let alpha = if duration == 0.0 {
            1.0
        } else {
            (sec - self.time_range.start) / duration
        };
        self.eval_alpha_dyn_into(alpha, output);
    }

    fn static_items_mut(&mut self) -> Option<&mut Vec<DynItem>> {
        match &mut self.inner {
            BuiltEval::Static(items) => Some(items),
            BuiltEval::Dynamic(_) => None,
        }
    }
}

impl Animation for BuiltAnimation {
    fn time_range(&self) -> Range<f64> {
        self.time_range()
    }

    fn build(mut self, origin_sec: f64, output: &mut Vec<BuiltAnimation>) {
        self.shift_by(origin_sec);
        output.push(self);
    }
}

/// Statically typed sequential composition.
pub struct Chained<A>(pub A);

/// Statically typed overlay composition.
pub struct Stacked<A>(pub A);

impl Animation for Chained<()> {
    fn time_range(&self) -> Range<f64> {
        0.0..0.0
    }

    fn build(self, _origin_sec: f64, _output: &mut Vec<BuiltAnimation>) {}
}

impl Placeable for Chained<()> {}

impl Animation for Stacked<()> {
    fn time_range(&self) -> Range<f64> {
        0.0..0.0
    }

    fn build(self, _origin_sec: f64, _output: &mut Vec<BuiltAnimation>) {}
}

impl Placeable for Stacked<()> {}

macro_rules! impl_animation_tuples {
    ($(($ty:ident, $index:tt)),+ $(,)?) => {
        impl<$($ty),+> Animation for Chained<($($ty,)+)>
        where
            $($ty: Animation + Placeable,)+
        {
            fn time_range(&self) -> Range<f64> {
                let duration = 0.0 $(+ self.0.$index.duration_secs())+;
                0.0..duration
            }

            fn build(self, origin_sec: f64, output: &mut Vec<BuiltAnimation>) {
                let mut cursor = origin_sec;
                $(
                    let duration = self.0.$index.duration_secs();
                    self.0.$index.build(cursor, output);
                    cursor += duration;
                )+
                let _ = cursor;
            }
        }

        impl<$($ty),+> Placeable for Chained<($($ty,)+)>
        where
            Chained<($($ty,)+)>: Animation,
        {}

        impl<$($ty),+> Animation for Stacked<($($ty,)+)>
        where
            $($ty: Animation,)+
        {
            fn time_range(&self) -> Range<f64> {
                let mut end: f64 = 0.0;
                $(end = end.max(self.0.$index.time_range().end);)+
                0.0..end
            }

            fn build(self, origin_sec: f64, output: &mut Vec<BuiltAnimation>) {
                $(self.0.$index.build(origin_sec, output);)+
            }
        }

        impl<$($ty),+> Placeable for Stacked<($($ty,)+)>
        where
            Stacked<($($ty,)+)>: Animation,
        {}
    };
}

impl_animation_tuples!((A0, 0));
impl_animation_tuples!((A0, 0), (A1, 1));
impl_animation_tuples!((A0, 0), (A1, 1), (A2, 2));
impl_animation_tuples!((A0, 0), (A1, 1), (A2, 2), (A3, 3));
impl_animation_tuples!((A0, 0), (A1, 1), (A2, 2), (A3, 3), (A4, 4));
impl_animation_tuples!((A0, 0), (A1, 1), (A2, 2), (A3, 3), (A4, 4), (A5, 5));
impl_animation_tuples!(
    (A0, 0),
    (A1, 1),
    (A2, 2),
    (A3, 3),
    (A4, 4),
    (A5, 5),
    (A6, 6)
);
impl_animation_tuples!(
    (A0, 0),
    (A1, 1),
    (A2, 2),
    (A3, 3),
    (A4, 4),
    (A5, 5),
    (A6, 6),
    (A7, 7)
);

/// Dynamic sequential animation container.
///
/// `play` is this container's type-erasure boundary: the input may retain its
/// full static composition type, while the resulting leaves are stored as
/// relocatable [`BuiltAnimation`] values in this sequence's local coordinates.
#[derive(Default)]
pub struct AnimSequence {
    animations: Vec<BuiltAnimation>,
    cursor_sec: f64,
}

impl AnimSequence {
    /// Create an empty sequence.
    pub fn new() -> Self {
        Self::default()
    }

    /// Append an animation at the current cursor and advance by its local extent.
    pub fn play<A: Animation>(&mut self, animation: A) -> &mut Self {
        let duration = animation.duration_secs();
        animation.build(self.cursor_sec, &mut self.animations);
        self.cursor_sec += duration;
        self
    }

    /// Append another sequence at the current cursor.
    pub fn extend(&mut self, sequence: AnimSequence) -> &mut Self {
        self.play(sequence)
    }

    /// Advance the cursor without adding an animation.
    pub fn forward(&mut self, secs: f64) -> &mut Self {
        assert!(
            secs.is_finite() && secs >= 0.0,
            "forward duration must be finite and non-negative"
        );
        self.cursor_sec += secs;
        self
    }

    /// Advance the cursor to `target_sec` without adding an animation.
    pub fn forward_to(&mut self, target_sec: f64) -> &mut Self {
        assert!(
            target_sec.is_finite() && target_sec >= 0.0,
            "forward target must be finite and non-negative"
        );
        if target_sec > self.cursor_sec {
            self.forward(target_sec - self.cursor_sec);
        }
        self
    }

    /// Advance the cursor while holding the state immediately before it.
    pub fn hold(&mut self, secs: f64) -> &mut Self {
        assert!(
            secs.is_finite() && secs >= 0.0,
            "hold duration must be finite and non-negative"
        );
        if secs == 0.0 {
            return self;
        }

        if let Some(last) = self.animations.last_mut()
            && last.time_range.end == self.cursor_sec
            && last.static_items_mut().is_some()
        {
            last.time_range.end += secs;
            self.cursor_sec += secs;
            return self;
        }

        let has_state_event = self.animations.iter().any(|animation| {
            animation.time_range.start == self.cursor_sec
                && animation.time_range.end == self.cursor_sec
        });
        let mut state = Vec::new();
        for animation in &self.animations {
            if has_state_event {
                if animation.enabled
                    && animation.time_range.start == self.cursor_sec
                    && animation.time_range.end == self.cursor_sec
                {
                    animation.eval_alpha_dyn_into(1.0, &mut state);
                }
            } else {
                animation.eval_left_at_sec_into(self.cursor_sec, &mut state);
            }
        }

        if !state.is_empty() {
            self.animations.push(BuiltAnimation {
                inner: BuiltEval::Static(state),
                rate_func: linear,
                time_range: self.cursor_sec..self.cursor_sec + secs,
                enabled: true,
                anim_name: type_name::<Static<Vec<DynItem>>>(),
            });
        }
        self.cursor_sec += secs;
        self
    }

    /// Advance the cursor to `target_sec` while holding its current state.
    pub fn hold_to(&mut self, target_sec: f64) -> &mut Self {
        assert!(
            target_sec.is_finite() && target_sec >= 0.0,
            "hold target must be finite and non-negative"
        );
        if target_sec > self.cursor_sec {
            self.hold(target_sec - self.cursor_sec);
        }
        self
    }

    /// Current cursor position.
    pub fn cursor_sec(&self) -> f64 {
        self.cursor_sec
    }

    /// Borrow the flattened leaves in local sequence coordinates.
    pub fn built_animations(&self) -> &[BuiltAnimation] {
        &self.animations
    }

    /// Consume this sequence into its flattened leaves.
    pub fn into_built_animations(self) -> Vec<BuiltAnimation> {
        self.animations
    }
}

impl Animation for AnimSequence {
    fn time_range(&self) -> Range<f64> {
        0.0..self.cursor_sec
    }

    fn build(mut self, origin_sec: f64, output: &mut Vec<BuiltAnimation>) {
        for animation in &mut self.animations {
            animation.shift_by(origin_sec);
        }
        output.extend(self.animations);
    }
}

impl Placeable for AnimSequence {}

/// Dynamic overlay animation container.
///
/// Unlike [`AnimSequence`], every pushed animation keeps its own local start
/// time and the stack duration is the maximum child extent.
#[derive(Default)]
pub struct AnimStack {
    animations: Vec<BuiltAnimation>,
    duration_secs: f64,
}

impl AnimStack {
    /// Create an empty dynamic stack.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add an animation without advancing the other children.
    pub fn push<A: Animation>(&mut self, animation: A) -> &mut Self {
        self.duration_secs = self.duration_secs.max(animation.duration_secs());
        animation.build(0.0, &mut self.animations);
        self
    }

    /// Add all built leaves from another dynamic stack.
    pub fn extend(&mut self, stack: AnimStack) -> &mut Self {
        self.duration_secs = self.duration_secs.max(stack.duration_secs);
        self.animations.extend(stack.animations);
        self
    }

    /// Current maximum child extent.
    pub fn duration_secs(&self) -> f64 {
        self.duration_secs
    }

    /// Borrow the flattened leaves in local stack coordinates.
    pub fn built_animations(&self) -> &[BuiltAnimation] {
        &self.animations
    }
}

impl Animation for AnimStack {
    fn time_range(&self) -> Range<f64> {
        0.0..self.duration_secs
    }

    fn build(mut self, origin_sec: f64, output: &mut Vec<BuiltAnimation>) {
        for animation in &mut self.animations {
            animation.shift_by(origin_sec);
        }
        output.extend(self.animations);
    }
}

impl Placeable for AnimStack {}

/// Construct a statically typed sequential composition.
#[macro_export]
macro_rules! chain {
    ($($animation:expr),* $(,)?) => {
        $crate::animation::Chained(($($animation,)*))
    };
}

/// Construct a statically typed overlay composition.
#[macro_export]
macro_rules! stack {
    ($($animation:expr),* $(,)?) => {
        $crate::animation::Stacked(($($animation,)*))
    };
}

/// Requirement for [`StaticAnim`].
pub trait StaticAnimRequirement: Clone {}

impl<T: Clone> StaticAnimRequirement for T {}

/// Convenience methods for zero-duration static animations.
pub trait StaticAnim: StaticAnimRequirement + Sized {
    /// Show this value.
    fn show(&self) -> AnimationCell<Self, Static<Self>>;
    /// Hide this value.
    fn hide(&self) -> AnimationCell<Self, Static<Self>>;
}

impl<T: StaticAnimRequirement + 'static> StaticAnim for T {
    fn show(&self) -> AnimationCell<Self, Static<Self>> {
        Static(self.clone())
            .into_animation_cell()
            .with_duration(0.0)
    }

    fn hide(&self) -> AnimationCell<Self, Static<Self>> {
        Static(self.clone())
            .into_animation_cell()
            .with_enabled(false)
            .with_duration(0.0)
    }
}

/// A constant evaluator.
pub struct Static<T>(pub T);

impl<T: Clone> Eval<T> for Static<T> {
    fn eval_alpha(&self, _alpha: f64) -> T {
        self.0.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Extract, core_item::CoreItem, core_item::vitem::VItem};

    fn leaf(x: f32, duration: f64) -> AnimationCell<VItem, impl Eval<VItem>> {
        (move |_alpha| {
            let mut item = VItem::default();
            item.points[0].x = x;
            item
        })
        .into_animation_cell()
        .with_duration(duration)
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

    #[test]
    fn at_adds_to_parent_origin() {
        let mut output = Vec::new();
        leaf(1.0, 2.0).at(3.0).build(10.0, &mut output);
        assert_eq!(output[0].time_range(), 13.0..15.0);
    }

    #[test]
    fn chained_uses_child_durations() {
        let mut output = Vec::new();
        chain![leaf(1.0, 2.0), leaf(2.0, 3.0)].build(5.0, &mut output);
        assert_eq!(output[0].time_range(), 5.0..7.0);
        assert_eq!(output[1].time_range(), 7.0..10.0);
    }

    #[test]
    fn stacked_accepts_plain_and_positioned_children() {
        let mut output = Vec::new();
        let animation = stack![leaf(1.0, 2.0), leaf(2.0, 3.0).at(1.0)];
        assert_eq!(animation.time_range(), 0.0..4.0);
        animation.build(10.0, &mut output);
        assert_eq!(output[0].time_range(), 10.0..12.0);
        assert_eq!(output[1].time_range(), 11.0..14.0);
    }

    #[test]
    fn sequence_can_be_repositioned_after_erasure() {
        let mut sequence = AnimSequence::new();
        sequence
            .play(leaf(1.0, 2.0))
            .forward(1.0)
            .play(leaf(2.0, 1.0));
        let mut output = Vec::new();
        sequence.build(10.0, &mut output);
        assert_eq!(output[0].time_range(), 10.0..12.0);
        assert_eq!(output[1].time_range(), 13.0..14.0);
    }

    #[test]
    fn hold_samples_only_animations_active_before_the_cursor() {
        let mut sequence = AnimSequence::new();
        sequence
            .play(stack![leaf(1.0, 1.0), leaf(2.0, 2.0)])
            .hold(1.0);

        assert_eq!(sequence.built_animations().len(), 3);
        let held = sequence.built_animations()[2].eval_at_sec(2.5).unwrap();
        assert_eq!(evaluated_xs(held), vec![2.0]);
    }

    #[test]
    fn repeated_hold_extends_one_flat_static_animation() {
        let mut sequence = AnimSequence::new();
        sequence.play(leaf(3.0, 1.0)).hold(1.0).hold(2.0);

        assert_eq!(sequence.cursor_sec(), 4.0);
        assert_eq!(sequence.built_animations().len(), 2);
        assert_eq!(sequence.built_animations()[1].time_range(), 1.0..4.0);
        let held = sequence.built_animations()[1].eval_at_sec(3.5).unwrap();
        assert_eq!(evaluated_xs(held), vec![3.0]);
    }

    #[test]
    fn forward_does_not_hold_the_previous_state() {
        let mut sequence = AnimSequence::new();
        sequence.play(leaf(4.0, 1.0)).forward(1.0).hold(1.0);

        assert_eq!(sequence.cursor_sec(), 3.0);
        assert_eq!(sequence.built_animations().len(), 1);
    }

    #[test]
    fn zero_duration_state_events_override_the_left_state_for_hold() {
        let mut shown = VItem::default();
        shown.points[0].x = 5.0;

        let mut hidden = AnimSequence::new();
        hidden.play(leaf(1.0, 1.0)).play(shown.hide()).hold(1.0);
        assert_eq!(hidden.built_animations().len(), 2);

        let mut restored = AnimSequence::new();
        restored.play(leaf(1.0, 1.0)).play(shown.show()).hold(1.0);
        let held = restored.built_animations()[2].eval_at_sec(1.5).unwrap();
        assert_eq!(evaluated_xs(held), vec![5.0]);
    }

    #[test]
    fn dynamic_stack_keeps_children_at_the_same_origin() {
        let mut stack = AnimStack::new();
        stack.push(leaf(1.0, 1.0)).push(leaf(2.0, 3.0));

        assert_eq!(stack.duration_secs(), 3.0);
        assert_eq!(stack.built_animations()[0].time_range(), 0.0..1.0);
        assert_eq!(stack.built_animations()[1].time_range(), 0.0..3.0);
    }
}
