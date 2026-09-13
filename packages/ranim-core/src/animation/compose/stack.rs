//! Dynamic overlay animation container.

use std::any::type_name;

use crate::animation::build::{IntoAnimNode, Unplaced};
use crate::animation::node::{AnimNode, NodeContent};

/// Dynamic overlay animation container.
///
/// Unlike [`AnimSequence`](super::sequence::AnimSequence), every pushed animation keeps its own local start
/// time and the stack duration is the maximum child extent.
#[derive(Default)]
pub struct AnimStack {
    animations: Vec<AnimNode>,
    duration_secs: f64,
}

impl AnimStack {
    /// Create an empty dynamic stack.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add an animation without advancing the other children.
    pub fn push<A: IntoAnimNode + 'static>(&mut self, animation: A) -> &mut Self {
        let animation = animation.into_anim_node();
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
    pub fn built_animations(&self) -> &[AnimNode] {
        &self.animations
    }

    /// Consume this stack into its direct child animations.
    pub fn into_built_animations(self) -> Vec<AnimNode> {
        self.animations
    }
}

impl Unplaced for AnimStack {}
impl IntoAnimNode for AnimStack {
    fn into_anim_node(self) -> AnimNode {
        let duration_secs = self.duration_secs;
        AnimNode {
            content: NodeContent::Stack(self.animations),
            internal_time_secs: duration_secs,
            rate_func: None,
            time_range: 0.0..duration_secs,
            enabled: true,
            anim_name: type_name::<Self>(),
        }
    }
}

/// Construct an [`AnimStack`] by pushing each animation at the same origin.
#[macro_export]
macro_rules! stack {
    ($($animation:expr),* $(,)?) => {
        {
            #[allow(unused_mut)]
            let mut stack = $crate::animation::compose::stack::AnimStack::new();
            $(stack.push($animation);)*
            stack
        }
    };
}

impl<A: IntoAnimNode + 'static> FromIterator<A> for AnimStack {
    fn from_iter<I: IntoIterator<Item = A>>(iter: I) -> Self {
        let mut stack = AnimStack::new();
        for animation in iter {
            stack.push(animation);
        }
        stack
    }
}
