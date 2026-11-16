//! Dynamic sequential animation container.

use std::any::type_name;

use crate::animation::{build::Unplaced, def_node::AnimDefNode, eval::HeldOutput};

/// Dynamic sequential animation container.
///
/// `push` erases each direct child's Rust type while retaining its definition
/// composition hierarchy in this sequence's local coordinates.
#[derive(Default)]
pub struct AnimSequence {
    pub(super) animations: Vec<AnimDefNode>,
    pub(super) cursor_sec: f64,
}

impl AnimSequence {
    /// Create an empty sequence.
    pub fn new() -> Self {
        Self::default()
    }

    /// Append an animation at the current cursor and advance by its local extent.
    pub fn push<A: Unplaced + 'static>(&mut self, animation: A) -> &mut Self {
        let mut animation = animation.into();
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

    /// Advance the cursor while holding a snapshot of its preceding state.
    pub fn hold(&mut self, secs: f64) -> &mut Self {
        assert!(
            secs.is_finite() && secs >= 0.0,
            "hold duration must be finite and non-negative"
        );
        if secs == 0.0 {
            return self;
        }

        if let Some(animation) = self
            .animations
            .iter()
            .rev()
            .find(|animation| animation.contains_sec(self.cursor_sec, self.cursor_sec))
        {
            let mut state = Vec::new();
            animation.build().capture_at(self.cursor_sec, &mut state);
            if !state.is_empty() {
                let mut held = AnimDefNode::leaf(HeldOutput::new(state));
                held.time_range = Some(self.cursor_sec..self.cursor_sec + secs);
                self.animations.push(held);
            }
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

    /// Borrow the direct child definitions in local sequence coordinates.
    pub fn definitions(&self) -> &[AnimDefNode] {
        &self.animations
    }

    /// Consume this sequence into its direct child definitions.
    pub fn into_definitions(self) -> Vec<AnimDefNode> {
        self.animations
    }
}

impl Unplaced for AnimSequence {}
impl From<AnimSequence> for AnimDefNode {
    /// Retain placements, discarding covered points and excluding earlier endpoints.
    fn from(sequence: AnimSequence) -> Self {
        let duration_secs = sequence.cursor_sec;
        // Holds have already captured their snapshots, so covered point clips
        // can be discarded when converting the sequence into a definition node.
        let last_at_end = sequence
            .animations
            .iter()
            .rposition(|animation| animation.contains_sec(duration_secs, duration_secs));
        let children = sequence
            .animations
            .into_iter()
            .enumerate()
            .filter_map(|(index, mut animation)| {
                if animation.duration_secs() == 0.0 && Some(index) != last_at_end {
                    return None;
                }
                if animation.time_range().end == duration_secs && Some(index) != last_at_end {
                    animation.end_inclusive = Some(false);
                }
                Some(animation)
            })
            .collect();
        let mut node = Self::children(children);
        node.internal_time_secs = Some(duration_secs);
        node.time_range = Some(0.0..duration_secs);
        node.anim_name = Some(type_name::<AnimSequence>());
        node
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
