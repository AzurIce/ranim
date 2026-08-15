//! Dynamic overlay animation container.

use std::any::type_name;

use crate::{core_item::DynItem, utils::rate_functions::linear};

use super::{Animation, AnimationCell, AnimationInfo, AnimationInfoKind, Placeable, eval::EvalDyn};

/// Dynamic overlay animation container.
///
/// Unlike [`AnimSequence`](super::sequence::AnimSequence), every pushed animation keeps its own local start
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
        }
    }
}

impl EvalDyn for AnimStack {
    fn eval_dyn(&self, alpha: f64, output: &mut Vec<DynItem>) {
        let content_sec = self.duration_secs * alpha;
        for child in &self.animations {
            if child.contains_sec(content_sec, self.duration_secs) {
                child.eval_at(content_sec, output);
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
            let mut stack = $crate::animation::stack::AnimStack::new();
            $(stack.push($animation);)*
            stack
        }
    };
}

impl<A: Animation + 'static> FromIterator<A> for AnimStack {
    fn from_iter<I: IntoIterator<Item = A>>(iter: I) -> Self {
        let mut stack = AnimStack::new();
        for animation in iter {
            stack.push(animation);
        }
        stack
    }
}
