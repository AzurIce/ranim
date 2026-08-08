//! Dynamic sequential animation container.

use std::any::type_name;

use crate::{core_item::DynItem, utils::rate_functions::linear};

use super::{
    Animation, AnimationCell, AnimationInfo, AnimationInfoKind, Placeable, eval::EvalDyn,
    static_cell,
};

/// Dynamic sequential animation container.
///
/// `push` erases each direct child's Rust type while retaining its runtime
/// composition hierarchy in this sequence's local coordinates.
#[derive(Default)]
pub struct AnimSequence {
    pub(super) animations: Vec<AnimationCell>,
    pub(super) cursor_sec: f64,
}

impl AnimSequence {
    /// Create an empty sequence.
    pub fn new() -> Self {
        Self::default()
    }

    fn eval_at_content_sec(&self, target_sec: f64, output: &mut Vec<DynItem>) {
        if let Some(animation) = self
            .animations
            .iter()
            .rev()
            .find(|animation| animation.contains_sec(target_sec, self.cursor_sec))
        {
            animation.eval_at(target_sec, output);
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
        self.eval_at_content_sec(self.cursor_sec, &mut state);

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
        }
    }
}

impl EvalDyn for AnimSequence {
    fn eval_dyn(&self, alpha: f64, output: &mut Vec<DynItem>) {
        let content_sec = self.cursor_sec * alpha;
        if let Some(child) = self
            .animations
            .iter()
            .rev()
            .find(|child| child.contains_sec(content_sec, self.cursor_sec))
        {
            child.eval_at(content_sec, output);
        }
    }

    fn materialize_dyn(&self, alpha: f64, ctx: &mut crate::logic::MaterializeCtx) {
        let content_sec = self.cursor_sec * alpha;
        if let Some(child) = self
            .animations
            .iter()
            .rev()
            .find(|child| child.contains_sec(content_sec, self.cursor_sec))
        {
            child.materialize_at(content_sec, ctx);
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
            let mut sequence = $crate::animation::sequence::AnimSequence::new();
            $(sequence.push($animation);)*
            sequence
        }
    };
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
