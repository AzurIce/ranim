//! Dynamic overlay animation container.

use std::any::type_name;

use crate::animation::{build::Unplaced, def_node::AnimDefNode};

/// Dynamic overlay animation container.
///
/// Unlike [`AnimSequence`](super::sequence::AnimSequence), every pushed animation keeps its own local start
/// time and the stack duration is the maximum child extent.
#[derive(Default, ranim_macros::Animation)]
pub struct AnimStack {
    animations: Vec<AnimDefNode>,
    duration_secs: f64,
}

impl AnimStack {
    /// Create an empty dynamic stack.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add an animation without advancing the other children.
    pub fn push<A: Into<AnimDefNode>>(&mut self, animation: A) -> &mut Self {
        let animation = animation.into();
        self.duration_secs = self.duration_secs.max(animation.time_range().end);
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

    /// Borrow the direct child definitions in local stack coordinates.
    pub fn definitions(&self) -> &[AnimDefNode] {
        &self.animations
    }

    /// Consume this stack into its direct child definitions.
    pub fn into_definitions(self) -> Vec<AnimDefNode> {
        self.animations
    }
}

impl Unplaced for AnimStack {}
impl From<AnimStack> for AnimDefNode {
    fn from(stack: AnimStack) -> Self {
        let duration = stack.duration_secs;
        let mut node = Self::children(stack.animations);
        node.internal_time_secs = Some(duration);
        node.time_range = Some(0.0..duration);
        node.anim_name = Some(type_name::<AnimStack>());
        node
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

impl<A: Into<AnimDefNode>> FromIterator<A> for AnimStack {
    fn from_iter<I: IntoIterator<Item = A>>(iter: I) -> Self {
        let mut stack = AnimStack::new();
        for animation in iter {
            stack.push(animation);
        }
        stack
    }
}
