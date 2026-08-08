//! Dynamic lagged (staggered, end-filled) animation container.

use std::any::type_name;

use crate::{core_item::DynItem, utils::rate_functions::linear};

use super::{
    Animation, AnimationCell, AnimationInfo, AnimationInfoKind, Placeable, eval::EvalDyn,
    sequence::AnimSequence, static_cell,
};

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
/// - `0.0` — all children start together (like [`AnimStack`](super::stack::AnimStack));
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
                child.eval_at(start, &mut state);
                if !state.is_empty() {
                    track.animations.push(static_cell(state, 0.0..start));
                }
            }
            track.animations.push(child);
            if self.trailing == LaggedFill::Hold && end < total {
                let child = track.animations.last().unwrap();
                let mut state = Vec::new();
                child.eval_at(end, &mut state);
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
        }
    }
}

impl EvalDyn for AnimLagged {
    fn eval_dyn(&self, alpha: f64, output: &mut Vec<DynItem>) {
        let content_sec = self.duration_secs * alpha;
        for animation in &self.animations {
            if animation.contains_sec(content_sec, self.duration_secs) {
                animation.eval_at(content_sec, output);
            }
        }
    }

    fn materialize_dyn(&self, alpha: f64, ctx: &mut crate::logic::MaterializeCtx) {
        let content_sec = self.duration_secs * alpha;
        for animation in &self.animations {
            if animation.contains_sec(content_sec, self.duration_secs) {
                animation.materialize_at(content_sec, ctx);
            }
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
            let mut lagged = $crate::animation::lagged::AnimLagged::new($lag_ratio);
            $(lagged.push($animation);)*
            lagged
        }
    };
}
