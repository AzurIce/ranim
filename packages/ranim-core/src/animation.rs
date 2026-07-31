//! Pure animation evaluation and hierarchical type-erased composition.

use std::{any::type_name, fmt::Debug, ops::Range};

use crate::{
    core_item::{AnyExtractCoreItem, DynItem},
    utils::rate_functions::linear,
};

/// A normalized, pure animation evaluator.
pub trait Eval {
    /// Value produced by this evaluator.
    type Output;

    /// Evaluate the animation at a normalized progress in `[0, 1]`.
    fn eval_alpha(&self, alpha: f64) -> Self::Output;

    /// Apply the final state and return this evaluator.
    fn apply_to(self, item: &mut Self::Output) -> Self
    where
        Self: Sized,
    {
        self.apply_alpha_to(item, 1.0)
    }

    /// Apply a sampled state and return this evaluator.
    fn apply_alpha_to(self, item: &mut Self::Output, alpha: f64) -> Self
    where
        Self: Sized,
    {
        *item = self.eval_alpha(alpha);
        self
    }
}

impl<T, F> Eval for F
where
    F: Fn(f64) -> T,
{
    type Output = T;

    fn eval_alpha(&self, alpha: f64) -> Self::Output {
        (self)(alpha)
    }
}

/// A complete time context for one evaluation/step of an animation segment.
///
/// All time values are in seconds (`_secs`); `alpha` is normalized to `[0, 1]`.
/// Deltas are only meaningful during [`Evaluator::step`]; during sampling they are zero.
#[derive(Debug, Clone, Copy, Default)]
pub struct SegmentTime {
    /// Global scene time `t` at the current logic tick (seconds).
    pub global_secs: f64,
    /// Global logic step length, stable (`= 1 / logic_fps`).
    pub global_delta_secs: f64,
    /// Segment start `s` in its parent's coordinates (seconds).
    pub start_secs: f64,
    /// Segment duration `D` (seconds).
    pub duration_secs: f64,
    /// Rate-warped local time `u(t) = D · r((t−s)/D)` (seconds).
    pub local_secs: f64,
    /// Local increment `Δu` for this step, varies with the rate function (seconds).
    pub local_delta_secs: f64,
    /// Normalized progress `alpha = local_secs / D`.
    pub alpha: f64,
    /// Current render frame index (for frame-coupled segments).
    pub render_frame: u64,
    /// Whether this logic step completes a render frame (for frame-coupled segments).
    pub is_render_frame_boundary: bool,
}

/// The common evaluator surface driven by the world/runtime.
///
/// Iterative (stateful) segments implement [`Evaluator`] directly with
/// `sample`/`reset`/`step`. Functional segments implement [`Eval`] and get
/// [`Evaluator`] for free through the blanket impl below.
pub trait Evaluator {
    /// Value produced by this evaluator.
    type Output;

    /// Sample the current state at the given time context.
    fn sample(&self, time: &SegmentTime) -> Self::Output;

    /// Reset to the segment's initial state (deterministic contract: no wall
    /// clock, no unseeded RNG). No-op by default.
    fn reset(&mut self) {}

    /// Advance one logic step (or substep); `time.local_delta_secs` is the
    /// integration step. No-op by default (functional segments are stepped as
    /// no-ops for free; sampling is unaffected).
    fn step(&mut self, _time: &SegmentTime) {}
}

impl<E: Eval> Evaluator for E {
    type Output = E::Output;

    fn sample(&self, time: &SegmentTime) -> Self::Output {
        self.eval_alpha(time.alpha)
    }
}

/// An auto implemented trait for erasing Evaluator<Output = T> where T: AnyExtractCoreItem
trait EvalDyn {
    fn eval_alpha_dyn_into(&self, alpha: f64, output: &mut Vec<DynItem>);

    /// Sample at a full time context. Default rides the pure alpha path for
    /// containers and functional leaves; iterative leaves override it via the
    /// blanket impl below.
    fn sample_dyn(&self, time: &SegmentTime, output: &mut Vec<DynItem>) {
        self.eval_alpha_dyn_into(time.alpha, output);
    }

    /// Reset the internal state. No-op by default.
    fn reset_dyn(&mut self) {}

    /// Advance one logic step. No-op by default (functional segments and
    /// containers without iterative leaves step for free).
    fn step_dyn(&mut self, _time: &SegmentTime) {}

    fn info_kind(&self) -> AnimationInfoKind {
        AnimationInfoKind::Eval
    }

    fn content_duration_secs(&self) -> f64 {
        1.0
    }

    fn child_infos(&self) -> Vec<AnimationInfo> {
        Vec::new()
    }
}

struct StaticDynItems(Vec<DynItem>);

impl EvalDyn for StaticDynItems {
    fn eval_alpha_dyn_into(&self, _alpha: f64, output: &mut Vec<DynItem>) {
        output.extend(self.0.iter().cloned());
    }

    fn info_kind(&self) -> AnimationInfoKind {
        AnimationInfoKind::Static
    }

    fn content_duration_secs(&self) -> f64 {
        0.0
    }
}

impl<E> EvalDyn for E
where
    E: Evaluator,
    E::Output: AnyExtractCoreItem,
{
    fn eval_alpha_dyn_into(&self, alpha: f64, output: &mut Vec<DynItem>) {
        // Pure-path compatibility: build a time context where only alpha is
        // meaningful and sample through it. Functional = eval_alpha(alpha);
        // iterative = current state (the pure path never steps; use SceneEvaluator).
        let time = SegmentTime {
            alpha,
            ..Default::default()
        };
        self.sample_dyn(&time, output);
    }

    fn sample_dyn(&self, time: &SegmentTime, output: &mut Vec<DynItem>) {
        output.push(DynItem(Box::new(self.sample(time))));
    }

    fn reset_dyn(&mut self) {
        Evaluator::reset(self);
    }

    fn step_dyn(&mut self, time: &SegmentTime) {
        Evaluator::step(self, time);
    }
}

/// Runtime animation content category used by preview tooling.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnimationInfoKind {
    /// A typed evaluator without animation children.
    Eval,
    /// A sequential animation container.
    Sequence,
    /// An overlay animation container.
    Stack,
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
    inner: Box<dyn EvalDyn>,
    rate_func: fn(f64) -> f64,
    time_range: Range<f64>,
    enabled: bool,
    anim_name: &'static str,
    /// Whether this cell's inner state has been reset for the current run.
    entered: bool,
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

    fn shift_by(&mut self, offset_sec: f64) {
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

    /// Compute the time context in this cell's local coordinates.
    fn local_time(&self, sec: f64, prev_sec: f64) -> SegmentTime {
        let duration = self.duration_secs();
        let alpha = if duration == 0.0 {
            1.0
        } else {
            (sec - self.time_range.start) / duration
        };
        let prev_alpha = if duration == 0.0 {
            1.0
        } else {
            (prev_sec - self.time_range.start) / duration
        };
        let rate_alpha = (self.rate_func)(alpha);
        let rate_prev = (self.rate_func)(prev_alpha);
        SegmentTime {
            global_secs: sec,
            global_delta_secs: sec - prev_sec,
            start_secs: self.time_range.start,
            duration_secs: duration,
            local_secs: rate_alpha * duration,
            local_delta_secs: (rate_alpha - rate_prev) * duration,
            alpha: rate_alpha,
            render_frame: 0,
            is_render_frame_boundary: false,
        }
    }

    /// Mark this clip as not yet entered (used by `SceneEvaluator::seek`).
    pub(crate) fn reset_entered(&mut self) {
        self.entered = false;
    }

    /// Sample this clip at a scene time (pure; no tick advancement).
    pub(crate) fn sample_at_sec(&self, sec: f64, output: &mut Vec<DynItem>) {
        if !self.active_at(sec) || !self.enabled {
            return;
        }
        let time = self.local_time(sec, sec);
        self.inner.sample_dyn(&time, output);
    }

    /// Advance this clip's inner state along the logic grid.
    ///
    /// Resets the inner state on first activation (deterministic replay), then
    /// steps it with the rate-warped local delta. Functional leaves and
    /// containers without iterative content step as no-ops.
    pub(crate) fn step_at_sec(&mut self, sec: f64, prev_sec: f64) {
        if !self.active_at(sec) || !self.enabled {
            return;
        }
        if !self.entered {
            self.inner.reset_dyn();
            self.entered = true;
        }
        let time = self.local_time(sec, prev_sec);
        self.inner.step_dyn(&time);
    }

    /// Append normalized evaluation results after applying this node's rate function.
    pub fn eval_alpha_dyn_into(&self, alpha: f64, output: &mut Vec<DynItem>) {
        if self.enabled {
            self.inner
                .eval_alpha_dyn_into((self.rate_func)(alpha), output);
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

    fn eval_at_sec_into(&self, sec: f64, output: &mut Vec<DynItem>) {
        if sec < self.time_range.start || sec > self.time_range.end {
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

    fn contains_sec(&self, sec: f64, parent_duration: f64) -> bool {
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
    E: Evaluator + 'static,
    E::Output: AnyExtractCoreItem,
{
}
impl<E> Animation for E
where
    E: Evaluator + 'static,
    E::Output: AnyExtractCoreItem,
{
    fn build(self) -> AnimationCell {
        AnimationCell {
            anim_name: type_name::<E>(),
            inner: Box::new(self),
            rate_func: linear,
            time_range: 0.0..1.0,
            enabled: true,
            entered: false,
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

/// Map a cell-local time context into a container's content coordinates.
///
/// The cell wrapping the container applies its own rate function; the content
/// position is `content_duration · (local_secs / cell_duration)`, mirroring the
/// pure path's `content_secs = cursor · rate(alpha)` mapping.
fn map_content_time(time: &SegmentTime, content_duration: f64) -> (f64, f64) {
    let cell_duration = time.duration_secs;
    if cell_duration <= 0.0 {
        return (0.0, 0.0);
    }
    let content_sec = content_duration * (time.local_secs / cell_duration);
    let prev_content_sec =
        content_duration * ((time.local_secs - time.local_delta_secs) / cell_duration);
    (content_sec, prev_content_sec)
}

/// Dynamic sequential animation container.
///
/// `push` erases each direct child's Rust type while retaining its runtime
/// composition hierarchy in this sequence's local coordinates.
#[derive(Default)]
pub struct AnimSequence {
    animations: Vec<AnimationCell>,
    cursor_sec: f64,
}

impl AnimSequence {
    /// Create an empty sequence.
    pub fn new() -> Self {
        Self::default()
    }

    fn eval_at_sec_into(&self, target_sec: f64, output: &mut Vec<DynItem>) {
        if let Some(animation) = self
            .animations
            .iter()
            .rev()
            .find(|animation| animation.contains_sec(target_sec, self.cursor_sec))
        {
            animation.eval_at_sec_into(target_sec, output);
        }
    }

    /// Append an animation at the current cursor and advance by its local extent.
    pub fn push<A: Placeable + 'static>(&mut self, animation: A) -> &mut Self {
        let mut animation = animation.build();
        let duration_secs = animation.duration_secs();
        animation.shift_by(self.cursor_sec);
        self.animations.push(animation);
        self.cursor_sec += duration_secs;
        self
    }

    /// Append another sequence's direct children at the current cursor.
    pub fn extend(&mut self, mut sequence: AnimSequence) -> &mut Self {
        for animation in &mut sequence.animations {
            animation.shift_by(self.cursor_sec);
        }
        self.animations.extend(sequence.animations);
        self.cursor_sec += sequence.cursor_sec;
        self
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

        let mut state = Vec::new();
        self.eval_at_sec_into(self.cursor_sec, &mut state);

        if !state.is_empty() {
            self.animations.push(AnimationCell {
                inner: Box::new(StaticDynItems(state)),
                rate_func: linear,
                time_range: self.cursor_sec..self.cursor_sec + secs,
                enabled: true,
                anim_name: type_name::<StaticDynItems>(),
                entered: false,
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

    /// Current sequence duration.
    pub fn duration_secs(&self) -> f64 {
        self.cursor_sec
    }

    /// Borrow the direct child animations in local sequence coordinates.
    pub fn built_animations(&self) -> &[AnimationCell] {
        &self.animations
    }

    /// Consume this sequence into its direct child animations.
    pub fn into_built_animations(self) -> Vec<AnimationCell> {
        self.animations
    }
}

impl Placeable for AnimSequence {}
impl Animation for AnimSequence {
    fn build(self) -> AnimationCell {
        let duration_secs = self.cursor_sec;
        AnimationCell {
            inner: Box::new(self),
            rate_func: linear,
            time_range: 0.0..duration_secs,
            enabled: true,
            anim_name: type_name::<Self>(),
            entered: false,
        }
    }
}

impl EvalDyn for AnimSequence {
    fn eval_alpha_dyn_into(&self, alpha: f64, output: &mut Vec<DynItem>) {
        self.eval_at_sec_into(self.cursor_sec * alpha, output);
    }

    fn step_dyn(&mut self, time: &SegmentTime) {
        // Map the time from the wrapping cell's local coordinates into this
        // sequence's content coordinates.
        let (content_sec, prev_content_sec) = map_content_time(time, self.cursor_sec);
        if let Some(child) = self
            .animations
            .iter_mut()
            .rev()
            .find(|child| child.contains_sec(content_sec, self.cursor_sec))
        {
            child.step_at_sec(content_sec, prev_content_sec);
        }
    }

    fn info_kind(&self) -> AnimationInfoKind {
        AnimationInfoKind::Sequence
    }

    fn content_duration_secs(&self) -> f64 {
        self.cursor_sec
    }

    fn child_infos(&self) -> Vec<AnimationInfo> {
        self.animations
            .iter()
            .map(AnimationCell::animation_info)
            .collect()
    }
}

/// Construct an [`AnimSequence`] by playing each animation in order.
#[macro_export]
macro_rules! seq {
    ($($animation:expr),* $(,)?) => {
        {
            #[allow(unused_mut)]
            let mut sequence = $crate::animation::AnimSequence::new();
            $(sequence.push($animation);)*
            sequence
        }
    };
}

/// Dynamic overlay animation container.
///
/// Unlike [`AnimSequence`], every pushed animation keeps its own local start
/// time and the stack duration is the maximum child extent.
#[derive(Default)]
pub struct AnimStack {
    animations: Vec<AnimationCell>,
    duration_secs: f64,
}

impl AnimStack {
    /// Create an empty dynamic stack.
    pub fn new() -> Self {
        Self::default()
    }

    fn eval_at_sec_into(&self, target_sec: f64, output: &mut Vec<DynItem>) {
        for animation in &self.animations {
            if animation.contains_sec(target_sec, self.duration_secs) {
                animation.eval_at_sec_into(target_sec, output);
            }
        }
    }

    /// Add an animation without advancing the other children.
    pub fn push<A: Animation + 'static>(&mut self, animation: A) -> &mut Self {
        let animation = animation.build();
        self.duration_secs = self.duration_secs.max(animation.time_range.end);
        self.animations.push(animation);
        self
    }

    /// Add all direct child animations from another dynamic stack.
    pub fn extend(&mut self, stack: AnimStack) -> &mut Self {
        self.duration_secs = self.duration_secs.max(stack.duration_secs);
        self.animations.extend(stack.animations);
        self
    }

    /// Current maximum child extent.
    pub fn duration_secs(&self) -> f64 {
        self.duration_secs
    }

    /// Borrow the direct child animations in local stack coordinates.
    pub fn built_animations(&self) -> &[AnimationCell] {
        &self.animations
    }

    /// Consume this stack into its direct child animations.
    pub fn into_built_animations(self) -> Vec<AnimationCell> {
        self.animations
    }
}

impl Placeable for AnimStack {}
impl Animation for AnimStack {
    fn build(self) -> AnimationCell {
        let duration_secs = self.duration_secs;
        AnimationCell {
            inner: Box::new(self),
            rate_func: linear,
            time_range: 0.0..duration_secs,
            enabled: true,
            anim_name: type_name::<Self>(),
            entered: false,
        }
    }
}

impl EvalDyn for AnimStack {
    fn eval_alpha_dyn_into(&self, alpha: f64, output: &mut Vec<DynItem>) {
        self.eval_at_sec_into(self.duration_secs * alpha, output);
    }

    fn step_dyn(&mut self, time: &SegmentTime) {
        let (content_sec, prev_content_sec) = map_content_time(time, self.duration_secs);
        for child in &mut self.animations {
            if child.contains_sec(content_sec, self.duration_secs) {
                child.step_at_sec(content_sec, prev_content_sec);
            }
        }
    }

    fn info_kind(&self) -> AnimationInfoKind {
        AnimationInfoKind::Stack
    }

    fn content_duration_secs(&self) -> f64 {
        self.duration_secs
    }

    fn child_infos(&self) -> Vec<AnimationInfo> {
        self.animations
            .iter()
            .map(AnimationCell::animation_info)
            .collect()
    }
}

/// Construct an [`AnimStack`] by pushing each animation at the same origin.
#[macro_export]
macro_rules! stack {
    ($($animation:expr),* $(,)?) => {
        {
            #[allow(unused_mut)]
            let mut stack = $crate::animation::AnimStack::new();
            $(stack.push($animation);)*
            stack
        }
    };
}

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
    use crate::{Extract, core_item::CoreItem, core_item::vitem::VItem};

    fn leaf(x: f32, duration: f64) -> impl Placeable {
        (move |_alpha| {
            let mut item = VItem::default();
            item.points[0].x = x;
            item
        })
        .with_duration(duration)
    }

    fn progress_leaf(offset: f32) -> impl Placeable {
        move |alpha| {
            let mut item = VItem::default();
            item.points[0].x = offset + alpha as f32;
            item
        }
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
    fn parametrized_sequence_remaps_the_group_timeline() {
        use crate::utils::rate_functions::ease_in_quad;

        let animation = seq![progress_leaf(0.0), progress_leaf(10.0)]
            .with_duration(4.0)
            .with_rate_func(ease_in_quad);
        let animation = animation.build();

        assert_eq!(animation.time_range(), 0.0..4.0);
        let items = animation.eval_at_sec(2.0).unwrap();
        assert_eq!(evaluated_xs(items), vec![0.5]);
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
        let held = sequence.built_animations()[1].eval_at_sec(2.5).unwrap();
        assert_eq!(evaluated_xs(held), vec![2.0]);
    }

    #[test]
    fn repeated_hold_creates_adjacent_static_animations() {
        let mut sequence = AnimSequence::new();
        sequence.push(leaf(3.0, 1.0)).hold(1.0).hold(2.0);

        assert_eq!(sequence.cursor_sec(), 4.0);
        assert_eq!(sequence.built_animations().len(), 3);
        assert_eq!(sequence.built_animations()[1].time_range(), 1.0..2.0);
        assert_eq!(sequence.built_animations()[2].time_range(), 2.0..4.0);
        let held = sequence.built_animations()[2].eval_at_sec(3.5).unwrap();
        assert_eq!(evaluated_xs(held), vec![3.0]);
    }

    #[test]
    fn repeated_hold_replays_dyn_items_without_nesting_the_output_batch() {
        let mut sequence = AnimSequence::new();
        sequence
            .push(stack![leaf(1.0, 1.0), leaf(2.0, 1.0)])
            .hold(1.0)
            .hold(1.0);

        let first_hold = sequence.built_animations()[1].eval_at_sec(1.5).unwrap();
        let second_hold = sequence.built_animations()[2].eval_at_sec(2.5).unwrap();

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
        assert!(
            hidden.built_animations()[1]
                .eval_at_sec(1.0)
                .unwrap()
                .is_empty()
        );

        let mut restored = AnimSequence::new();
        restored.push(leaf(1.0, 1.0)).push(shown.show()).hold(1.0);
        let held = restored.built_animations()[2].eval_at_sec(1.5).unwrap();
        assert_eq!(evaluated_xs(held), vec![5.0]);
    }

    #[test]
    fn nested_sequences_keep_their_own_final_evaluation() {
        let mut shown = VItem::default();
        shown.points[0].x = 7.0;

        let inner = seq![leaf(1.0, 1.0), shown.show()];
        let mut outer = AnimSequence::new();
        outer.push(inner).hold(1.0);

        assert_eq!(outer.built_animations().len(), 2);
        let held = outer.built_animations()[1].eval_at_sec(1.5).unwrap();
        assert_eq!(evaluated_xs(held), vec![7.0]);

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
}
