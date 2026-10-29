//! Dynamic sequential animation container.

use std::any::type_name;

use crate::animation::eval::DynOutput;

use crate::animation::build::{IntoAnimNode, Unplaced};
use crate::animation::node::{AnimNode, NodeContent, constant_node};

/// Dynamic sequential animation container.
///
/// `push` erases each direct child's Rust type while retaining its runtime
/// composition hierarchy in this sequence's local coordinates.
#[derive(Default)]
pub struct AnimSequence {
    pub(super) animations: Vec<AnimNode>,
    pub(super) cursor_sec: f64,
}

impl AnimSequence {
    /// Create an empty sequence.
    pub fn new() -> Self {
        Self::default()
    }

    fn eval_at_content_sec(&self, target_sec: f64, output: &mut Vec<DynOutput>) {
        if let Some(animation) = self
            .animations
            .iter()
            .rev()
            .find(|animation| animation.contains_sec(target_sec, self.cursor_sec))
        {
            animation.capture_at(target_sec, output);
        }
    }

    /// Append an animation at the current cursor and advance by its local extent.
    pub fn push<A: Unplaced + 'static>(&mut self, animation: A) -> &mut Self {
        let mut animation = animation.into_anim_node();
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
            self.animations.push(constant_node(
                state,
                self.cursor_sec..self.cursor_sec + secs,
            ));
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
    pub fn built_animations(&self) -> &[AnimNode] {
        &self.animations
    }

    /// Consume this sequence into its direct child animations.
    pub fn into_built_animations(self) -> Vec<AnimNode> {
        self.animations
    }
}

impl Unplaced for AnimSequence {}
impl IntoAnimNode for AnimSequence {
    /// Lower placements to children, discarding covered point animations and
    /// excluding earlier endpoints when a final point supplies the final frame.
    fn into_anim_node(self) -> AnimNode {
        let duration_secs = self.cursor_sec;
        // Point clips only match at the final content boundary. Holds have
        // already sampled the authoring sequence, so earlier point clips can
        // now be removed, including points covered by a later final clip.
        let last_at_end = self
            .animations
            .iter()
            .rposition(|animation| animation.contains_sec(duration_secs, duration_secs));
        let children = self
            .animations
            .into_iter()
            .enumerate()
            .filter_map(|(index, mut animation)| {
                if animation.duration_secs() == 0.0 && Some(index) != last_at_end {
                    None
                } else {
                    if animation.time_range.end == duration_secs && Some(index) != last_at_end {
                        animation.end_inclusive = false;
                    }
                    Some(animation)
                }
            })
            .collect();
        AnimNode {
            content: NodeContent::Children(children),
            internal_time_secs: duration_secs,
            rate_func: None,
            time_range: 0.0..duration_secs,
            end_inclusive: true,
            enabled: true,
            anim_name: type_name::<Self>(),
        }
    }
}

/// Construct an [`AnimSequence`] by playing each animation in order.
#[macro_export]
macro_rules! seq {
    ($($animation:expr),* $(,)?) => {
        {
            #[allow(unused_mut)]
            let mut sequence = $crate::animation::compose::sequence::AnimSequence::new();
            $(sequence.push($animation);)*
            sequence
        }
    };
}

impl<A: Unplaced + 'static> FromIterator<A> for AnimSequence {
    fn from_iter<I: IntoIterator<Item = A>>(iter: I) -> Self {
        let mut sequence = AnimSequence::new();
        for animation in iter {
            sequence.push(animation);
        }
        sequence
    }
}
