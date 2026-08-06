//! Pure animation evaluation and hierarchical type-erased composition.

use std::{any::type_name, fmt::Debug, ops::Range};

use crate::{
    core_item::{AnyExtractCoreItem, DynItem},
    time::{DeltaTime, GlobalTime, Time},
    utils::rate_functions::linear,
};

/// The general animation segment protocol: what the runtime can do with a segment.
///
/// This is the only protocol the runtime drives. Author-facing specializations
/// live in `ranim-anims` (the `PureEval`/`IterativeEval` capability traits with
/// their `Pure`/`Iterative` adapter structs); implementing `Eval` directly is
/// the path for exotic segments. All three evaluation methods are required —
/// a stateful segment that forgets `reset` fails to compile instead of
/// silently breaking replay determinism.
///
/// How the protocol serves the outer layers:
///
/// - render's per-frame sampling and the pure query path both ride
///   [`sample`](Eval::sample) (a pure segment's closed form *is* its `sample`);
/// - [`SceneEvaluator::advance_to`](crate::SceneEvaluator::advance_to) drives
///   `step` tick by tick;
/// - [`SceneEvaluator::seek`](crate::SceneEvaluator::seek) (preview scrubbing)
///   resets everything, then replays `step` — seek is a session-level
///   composite, not a segment method.
pub trait Eval {
    /// Value produced by this evaluator.
    type Output;

    /// Sample the output at the current time point.
    ///
    /// `time.alpha` is the segment's rate-warped progress computed by the
    /// owning cell; a stateful segment typically ignores it and projects its
    /// current state. Sampling never advances state and carries no deltas.
    fn sample(&self, time: &Time) -> Self::Output;

    /// Reset to the segment's initial state (deterministic contract: no wall
    /// clock, no unseeded RNG). Stateless segments leave this empty.
    fn reset(&mut self);

    /// Advance one logic step. `delta_time.alpha` is the integration step in
    /// the segment's warped local progress (variable under a non-linear rate);
    /// `delta_time.global_secs` is the unwarped global step (`1 / logic_fps`)
    /// for real-time physics. Stateless segments leave this empty.
    fn step(&mut self, time: &Time, delta_time: &DeltaTime);

    /// Write the state at `alpha` into `item` and return this evaluator.
    ///
    /// A build-time convenience defined through `sample` (the constructed
    /// `Time` has `global_secs == 0.0`): pure segments apply their closed-form
    /// state; stateful segments apply their current projected state (the
    /// initial state if never stepped).
    fn apply_alpha_to(self, item: &mut Self::Output, alpha: f64) -> Self
    where
        Self: Sized,
    {
        *item = self.sample(&Time {
            alpha,
            global_secs: 0.0,
        });
        self
    }

    /// Write the end state (`alpha == 1.0`) into `item` and return this evaluator.
    fn apply_to(self, item: &mut Self::Output) -> Self
    where
        Self: Sized,
    {
        self.apply_alpha_to(item, 1.0)
    }
}

/// An auto implemented trait for erasing `Eval<Output = T>` where T: AnyExtractCoreItem
trait EvalDyn {
    /// Sample into erased items. The only erased read path: pure leaves
    /// evaluate their closed form, stateful leaves project current state,
    /// containers route `time.alpha` into their content coordinates.
    fn sample_dyn(&self, time: &Time, output: &mut Vec<DynItem>);

    /// Reset the internal state. No-op by default.
    fn reset_dyn(&mut self) {}

    /// Advance one logic step. No-op by default (`StaticDynItems` has nothing
    /// to advance); containers override to forward stepping to their children.
    fn step_dyn(&mut self, _time: &Time, _delta_time: &DeltaTime) {}

    /// Mark all descendant cells as not yet entered, so their next activation
    /// resets again (used by `SceneEvaluator::seek`). Containers override to
    /// recurse; leaves have no children.
    fn reset_entered_dyn(&mut self) {}

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
    fn sample_dyn(&self, _time: &Time, output: &mut Vec<DynItem>) {
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
    E: Eval,
    E::Output: AnyExtractCoreItem,
{
    fn sample_dyn(&self, time: &Time, output: &mut Vec<DynItem>) {
        output.push(DynItem(Box::new(self.sample(time))));
    }

    fn reset_dyn(&mut self) {
        Eval::reset(self);
    }

    fn step_dyn(&mut self, time: &Time, delta_time: &DeltaTime) {
        Eval::step(self, time, delta_time);
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

    /// Mark this clip and all its descendants as not yet entered (used by
    /// `SceneEvaluator::seek`).
    pub(crate) fn reset_entered(&mut self) {
        self.entered = false;
        self.inner.reset_entered_dyn();
    }

    /// Sample this clip at a scene time (pure; no tick advancement).
    ///
    /// `global_secs` is the true scene time, forwarded into the sampled
    /// [`Time`] unchanged.
    pub(crate) fn sample_at_sec(&self, sec: f64, global_secs: f64, output: &mut Vec<DynItem>) {
        if !self.active_at(sec) || !self.enabled {
            return;
        }
        let time = Time {
            alpha: self.local_alpha(sec),
            global_secs,
        };
        self.inner.sample_dyn(&time, output);
    }

    /// Advance this clip's inner state along the logic grid.
    ///
    /// Resets the inner state on first activation (deterministic replay), then
    /// steps it with the rate-warped local delta; stateless leaves step as
    /// no-ops. `global` is forwarded unchanged from the driving session, so
    /// [`Time::global_secs`]/[`DeltaTime::global_secs`] stay honest at any
    /// nesting depth.
    pub(crate) fn step_at_sec(&mut self, sec: f64, prev_sec: f64, global: &GlobalTime) {
        if !self.active_at(sec) || !self.enabled {
            return;
        }
        if !self.entered {
            self.inner.reset_dyn();
            self.entered = true;
        }
        let alpha = self.local_alpha(sec);
        let time = Time {
            alpha,
            global_secs: global.secs,
        };
        let delta_time = DeltaTime {
            alpha: alpha - self.local_alpha(prev_sec),
            global_secs: global.delta_secs,
        };
        self.inner.step_dyn(&time, &delta_time);
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

    fn sample_at_content_sec(&self, target_sec: f64, global_secs: f64, output: &mut Vec<DynItem>) {
        if let Some(animation) = self
            .animations
            .iter()
            .rev()
            .find(|animation| animation.contains_sec(target_sec, self.cursor_sec))
        {
            animation.sample_at_sec(target_sec, global_secs, output);
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
        self.sample_at_content_sec(self.cursor_sec, self.cursor_sec, &mut state);

        if !state.is_empty() {
            self.animations
                .push(static_cell(state, self.cursor_sec..self.cursor_sec + secs));
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
    fn sample_dyn(&self, time: &Time, output: &mut Vec<DynItem>) {
        self.sample_at_content_sec(self.cursor_sec * time.alpha, time.global_secs, output);
    }

    fn step_dyn(&mut self, time: &Time, delta_time: &DeltaTime) {
        // Map the wrapping cell's warped progress into this sequence's content
        // coordinates; the global channel is forwarded unchanged.
        let content_sec = self.cursor_sec * time.alpha;
        let prev_content_sec = self.cursor_sec * (time.alpha - delta_time.alpha);
        if let Some(child) = self
            .animations
            .iter_mut()
            .rev()
            .find(|child| child.contains_sec(content_sec, self.cursor_sec))
        {
            child.step_at_sec(content_sec, prev_content_sec, &GlobalTime::of(time, delta_time));
        }
    }

    fn reset_entered_dyn(&mut self) {
        for child in &mut self.animations {
            child.reset_entered();
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

    fn sample_at_content_sec(&self, target_sec: f64, global_secs: f64, output: &mut Vec<DynItem>) {
        for animation in &self.animations {
            if animation.contains_sec(target_sec, self.duration_secs) {
                animation.sample_at_sec(target_sec, global_secs, output);
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
    fn sample_dyn(&self, time: &Time, output: &mut Vec<DynItem>) {
        self.sample_at_content_sec(self.duration_secs * time.alpha, time.global_secs, output);
    }

    fn step_dyn(&mut self, time: &Time, delta_time: &DeltaTime) {
        let content_sec = self.duration_secs * time.alpha;
        let prev_content_sec = self.duration_secs * (time.alpha - delta_time.alpha);
        for child in &mut self.animations {
            if child.contains_sec(content_sec, self.duration_secs) {
                child.step_at_sec(content_sec, prev_content_sec, &GlobalTime::of(time, delta_time));
            }
        }
    }

    fn reset_entered_dyn(&mut self) {
        for child in &mut self.animations {
            child.reset_entered();
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

/// How an [`AnimLagged`] fills the time outside a child's window.
///
/// Fills are materialized as real static cells at `build` time (sampled from
/// the child's window edges), so the preview timeline shows exactly what is
/// rendered — there is no hidden clamping rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LaggedFill {
    /// Render nothing outside the window.
    Empty,
    /// Keep showing the window-edge state with a static animation
    /// (the initial state before, the final state after).
    Hold,
}

/// Dynamic lagged (staggered, end-filled) animation container.
///
/// Children are pushed un-placed ([`Placeable`]); the container computes the
/// placement itself: child `i` starts at `start_{i-1} + lag_ratio · d_{i-1}`.
/// `lag_ratio` interpolates between the other two containers:
///
/// - `0.0` — all children start together (like [`AnimStack`]);
/// - `1.0` — each child starts when the previous ends (like [`AnimSequence`]);
/// - in between — overlapping succession.
///
/// By default the time outside a child's window is **filled with real static
/// cells** ([`LaggedFill::Hold`] on both ends): each item is materialized at
/// build time as a per-item sequence track `[leading fill][anim][trailing
/// fill]` spanning the whole extent — before its start the item shows its
/// initial state, after its end it keeps showing its final state, and the
/// preview timeline shows exactly what is rendered. Configure with
/// [`with_leading`](Self::with_leading) /
/// [`with_trailing`](Self::with_trailing) — e.g. `Empty` leading makes items
/// appear only at their window; to hide an item after its window instead of
/// holding it, give it an animation that ends hidden, e.g.
/// `seq![item.fade_in(), item.hide()]`.
///
/// Fills are sampled at build time: empty fills are skipped, and a
/// zero-duration child gets no leading fill (it appears at its point, which is
/// what "show from that point on" means). Because fills are build-time
/// samples, children are expected to be pure (closed-form) animations — a
/// stateful child's trailing fill would be its initial state, not its true
/// final state.
pub struct AnimLagged {
    animations: Vec<AnimationCell>,
    lag_ratio: f64,
    /// Start offset for the next pushed child.
    cursor_sec: f64,
    duration_secs: f64,
    leading: LaggedFill,
    trailing: LaggedFill,
}

/// Build a static cell replaying already-sampled items over `time_range`.
fn static_cell(state: Vec<DynItem>, time_range: Range<f64>) -> AnimationCell {
    AnimationCell {
        inner: Box::new(StaticDynItems(state)),
        rate_func: linear,
        time_range,
        enabled: true,
        anim_name: type_name::<StaticDynItems>(),
        entered: false,
    }
}

impl AnimLagged {
    /// Create an empty lagged container with the given stagger ratio.
    pub fn new(lag_ratio: f64) -> Self {
        assert!(
            lag_ratio.is_finite() && lag_ratio >= 0.0,
            "lag ratio must be finite and non-negative"
        );
        Self {
            animations: Vec::new(),
            lag_ratio,
            cursor_sec: 0.0,
            duration_secs: 0.0,
            leading: LaggedFill::Hold,
            trailing: LaggedFill::Hold,
        }
    }

    /// The stagger ratio between successive children.
    pub fn lag_ratio(&self) -> f64 {
        self.lag_ratio
    }

    /// Configure the fill before each child's window (default
    /// [`LaggedFill::Hold`]).
    pub fn with_leading(mut self, behavior: LaggedFill) -> Self {
        self.leading = behavior;
        self
    }

    /// Configure the fill after each child's window (default
    /// [`LaggedFill::Hold`]).
    pub fn with_trailing(mut self, behavior: LaggedFill) -> Self {
        self.trailing = behavior;
        self
    }

    /// Add an animation, placed by the container's stagger rule.
    pub fn push<A: Placeable + 'static>(&mut self, animation: A) -> &mut Self {
        let mut animation = animation.build();
        let duration_secs = animation.duration_secs();
        animation.shift_by(self.cursor_sec);
        self.duration_secs = self.duration_secs.max(animation.time_range.end);
        self.animations.push(animation);
        self.cursor_sec += self.lag_ratio * duration_secs;
        self
    }

    /// Materialize each child as a per-item sequence track:
    /// `[leading fill][anim][trailing fill]` (empty fills skipped).
    ///
    /// The lagged container is thus a stack of per-item sequences — each
    /// item's track spans the whole extent, with its window-edge states held
    /// by real static cells.
    fn materialize_fills(&mut self) {
        let total = self.duration_secs;
        let children = std::mem::take(&mut self.animations);
        let mut animations = Vec::with_capacity(children.len());
        for child in children {
            let start = child.time_range.start;
            let end = child.time_range.end;
            let mut track = AnimSequence::new();
            if self.leading == LaggedFill::Hold && start > 0.0 && child.duration_secs() > 0.0 {
                let mut state = Vec::new();
                child.sample_at_sec(start, start, &mut state);
                if !state.is_empty() {
                    track.animations.push(static_cell(state, 0.0..start));
                }
            }
            track.animations.push(child);
            if self.trailing == LaggedFill::Hold && end < total {
                let child = track.animations.last().unwrap();
                let mut state = Vec::new();
                child.sample_at_sec(end, end, &mut state);
                if !state.is_empty() {
                    track.animations.push(static_cell(state, end..total));
                }
            }
            track.cursor_sec = total;
            animations.push(track.build());
        }
        self.animations = animations;
    }

    /// Current total extent (the last child's end).
    pub fn duration_secs(&self) -> f64 {
        self.duration_secs
    }

    /// Borrow the direct child animations in local container coordinates.
    pub fn built_animations(&self) -> &[AnimationCell] {
        &self.animations
    }

    /// Consume this container into its direct child animations.
    pub fn into_built_animations(self) -> Vec<AnimationCell> {
        self.animations
    }
}

impl Placeable for AnimLagged {}
impl Animation for AnimLagged {
    fn build(mut self) -> AnimationCell {
        self.materialize_fills();
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

impl EvalDyn for AnimLagged {
    fn sample_dyn(&self, time: &Time, output: &mut Vec<DynItem>) {
        let content_sec = self.duration_secs * time.alpha;
        for animation in &self.animations {
            if animation.contains_sec(content_sec, self.duration_secs) {
                animation.sample_at_sec(content_sec, time.global_secs, output);
            }
        }
    }

    fn step_dyn(&mut self, time: &Time, delta_time: &DeltaTime) {
        let content_sec = self.duration_secs * time.alpha;
        let prev_content_sec = self.duration_secs * (time.alpha - delta_time.alpha);
        for child in &mut self.animations {
            if child.contains_sec(content_sec, self.duration_secs) {
                child.step_at_sec(content_sec, prev_content_sec, &GlobalTime::of(time, delta_time));
            }
        }
    }

    fn reset_entered_dyn(&mut self) {
        for child in &mut self.animations {
            child.reset_entered();
        }
    }

    fn info_kind(&self) -> AnimationInfoKind {
        AnimationInfoKind::Lagged
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

/// Construct an [`AnimLagged`] with a stagger ratio, pushing each animation in order.
#[macro_export]
macro_rules! lagged {
    ($lag_ratio:expr; $($animation:expr),* $(,)?) => {
        {
            #[allow(unused_mut)]
            let mut lagged = $crate::animation::AnimLagged::new($lag_ratio);
            $(lagged.push($animation);)*
            lagged
        }
    };
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

impl<A: Animation + 'static> FromIterator<A> for AnimStack {
    fn from_iter<I: IntoIterator<Item = A>>(iter: I) -> Self {
        let mut stack = AnimStack::new();
        for animation in iter {
            stack.push(animation);
        }
        stack
    }
}

impl<A: Placeable + 'static> FromIterator<A> for AnimSequence {
    fn from_iter<I: IntoIterator<Item = A>>(iter: I) -> Self {
        let mut sequence = AnimSequence::new();
        for animation in iter {
            sequence.push(animation);
        }
        sequence
    }
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

    fn sample(&self, _time: &Time) -> Self::Output {
        self.0.clone()
    }

    fn reset(&mut self) {}

    fn step(&mut self, _time: &Time, _delta_time: &DeltaTime) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Extract, core_item::CoreItem, core_item::vitem::VItem};

    /// A stateless test double: a `VItem` shifted to a fixed x.
    struct ShiftX(f32);

    impl Eval for ShiftX {
        type Output = VItem;

        fn sample(&self, _time: &Time) -> Self::Output {
            let mut item = VItem::default();
            item.points[0].x = self.0;
            item
        }

        fn reset(&mut self) {}

        fn step(&mut self, _time: &Time, _delta_time: &DeltaTime) {}
    }

    /// A stateless test double: x = offset + alpha.
    struct ShiftAlpha(f32);

    impl Eval for ShiftAlpha {
        type Output = VItem;

        fn sample(&self, time: &Time) -> Self::Output {
            let mut item = VItem::default();
            item.points[0].x = self.0 + time.alpha as f32;
            item
        }

        fn reset(&mut self) {}

        fn step(&mut self, _time: &Time, _delta_time: &DeltaTime) {}
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
        animation.sample_at_sec(sec, sec, &mut items);
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
        sequence.built_animations()[1].sample_at_sec(1.5, 1.5, &mut first_hold);
        let mut second_hold = Vec::new();
        sequence.built_animations()[2].sample_at_sec(2.5, 2.5, &mut second_hold);

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
        hidden.built_animations()[1].sample_at_sec(1.0, 1.0, &mut hidden_items);
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
        animation.sample_at_sec(1.0, 1.0, &mut items);
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
        let sequence = vec![leaf(1.0, 1.0), leaf(2.0, 1.0)]
            .into_iter()
            .into_seq();
        assert_eq!(sequence.duration_secs(), 2.0);
    }
}
