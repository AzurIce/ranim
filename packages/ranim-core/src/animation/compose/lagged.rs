//! Lagged (staggered, end-filled) authoring combinator.
//!
//! [`AnimLagged`] staggers child definitions on the content axis. Conversion
//! into [`AnimDefNode`] samples window edges and creates nested per-item tracks
//! with constant fills, with no lagged runtime variant.

use std::any::type_name;

use crate::animation::compose::sequence::AnimSequence;
use crate::animation::{build::Unplaced, def_node::AnimDefNode, eval::HeldOutput};

/// How an [`AnimLagged`] fills the time outside a child's window.
///
/// Fills are sampled from the child's window edges during conversion into
/// [`AnimDefNode`], so the preview timeline shows exactly what is
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
/// Children are pushed un-placed ([`Unplaced`]); the container computes the
/// placement itself: child `i` starts at `start_{i-1} + lag_ratio · d_{i-1}`.
/// `lag_ratio` interpolates between the other two containers:
///
/// - `0.0` — all children start together (like [`AnimStack`](super::stack::AnimStack));
/// - `1.0` — each child starts when the previous ends (like [`AnimSequence`]);
/// - in between — overlapping succession.
///
/// By default the time outside a child's window is **filled with constant animation
/// leaves** ([`LaggedFill::Hold`] on both ends): conversion into [`AnimDefNode`]
/// creates a per-item sequence track `[leading fill][anim][trailing
/// fill]` spanning the whole extent — before its start the item shows its
/// initial state, after its end it keeps showing its final state, and the
/// preview timeline shows exactly what is rendered. Configure with
/// [`with_leading`](Self::with_leading) /
/// [`with_trailing`](Self::with_trailing) — e.g. `Empty` leading makes items
/// appear only at their window; to hide an item after its window instead of
/// holding it, give it an animation that ends hidden, e.g.
/// `seq![item.anim_ref(FadeIn), item.hide()]`.
///
/// Fills are sampled during conversion into [`AnimDefNode`]: empty fills are skipped,
/// and a zero-duration child gets no leading fill (it appears at its point,
/// which is what "show from that point on" means).
pub struct AnimLagged {
    animations: Vec<AnimDefNode>,
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
    pub fn push<A: Unplaced + 'static>(&mut self, animation: A) -> &mut Self {
        let mut animation = animation.into();
        let duration_secs = animation.duration_secs();
        animation.shift_by(self.cursor_sec);
        self.duration_secs = self.duration_secs.max(animation.time_range().end);
        self.animations.push(animation);
        self.cursor_sec += self.lag_ratio * duration_secs;
        self
    }

    /// Arrange each child as a per-item sequence definition:
    /// `[leading fill][anim][trailing fill]`, sampling the child's window edges.
    ///
    /// The lagged container is thus a stack of per-item sequences — each
    /// item's track spans the whole extent, with its window-edge states held
    /// by constant animation leaves.
    fn materialize_fills(&mut self) {
        let total = self.duration_secs;
        let children = std::mem::take(&mut self.animations);
        let mut animations = Vec::with_capacity(children.len());
        for child in children {
            let start = child.time_range().start;
            let end = child.time_range().end;
            let mut track = AnimSequence::new();
            if self.leading == LaggedFill::Hold && start > 0.0 && child.duration_secs() > 0.0 {
                let mut state = Vec::new();
                child.build().capture_at(start, &mut state);
                if !state.is_empty() {
                    let mut held = AnimDefNode::leaf(HeldOutput::new(state));
                    held.time_range = Some(0.0..start);
                    track.animations.push(held);
                }
            }
            track.animations.push(child);
            if self.trailing == LaggedFill::Hold && end < total {
                let mut state = Vec::new();
                track
                    .animations
                    .last()
                    .unwrap()
                    .build()
                    .capture_at(end, &mut state);
                if !state.is_empty() {
                    let mut held = AnimDefNode::leaf(HeldOutput::new(state));
                    held.time_range = Some(end..total);
                    track.animations.push(held);
                }
            }
            track.cursor_sec = total;
            animations.push(track.into());
        }
        self.animations = animations;
    }

    /// Current total extent (the maximum child end).
    pub fn duration_secs(&self) -> f64 {
        self.duration_secs
    }

    /// Borrow the children as placed by the stagger rule — the
    /// pre-materialization view, before conversion turns them into filled
    /// tracks.
    pub fn definitions(&self) -> &[AnimDefNode] {
        &self.animations
    }
}

impl Unplaced for AnimLagged {}
impl From<AnimLagged> for AnimDefNode {
    /// Sample edge fills and arrange full-extent definition tracks.
    /// Staggering is ordinary window placement, not a distinct node kind.
    fn from(mut lagged: AnimLagged) -> Self {
        lagged.materialize_fills();
        let duration = lagged.duration_secs;
        let mut node = Self::children(lagged.animations);
        node.internal_time_secs = Some(duration);
        node.time_range = Some(0.0..duration);
        node.anim_name = Some(type_name::<AnimLagged>());
        node
    }
}

/// Construct an [`AnimLagged`] with a stagger ratio, pushing each animation in order.
#[macro_export]
macro_rules! lagged {
    ($lag_ratio:expr; $($animation:expr),* $(,)?) => {
        {
            #[allow(unused_mut)]
            let mut lagged = $crate::animation::compose::lagged::AnimLagged::new($lag_ratio);
            $(lagged.push($animation);)*
            lagged
        }
    };
}
