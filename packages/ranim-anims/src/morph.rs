//! Morph parameters and prepared geometry alignment.

use ranim_core::{
    animation::{AnimRecipe, AnimWriteBack, eval::Eval, node::AnimNode},
    traits::{Alignable, Interpolatable},
};

/// Item capabilities needed for aligned interpolation.
pub trait MorphRequirement: Alignable + Interpolatable + Clone {}
impl<T: Alignable + Interpolatable + Clone> MorphRequirement for T {}

/// Interpolate an input item to an explicit destination.
#[derive(Clone)]
pub struct Morph<T> {
    /// Destination before runtime alignment is performed.
    pub target: T,
}

impl<T> Morph<T> {
    /// Describe an explicit destination.
    pub fn to(target: T) -> Self {
        Self { target }
    }
}

impl<T: Clone> Morph<T> {
    /// Construct a destination by editing a clone of `input` once.
    ///
    /// The closure is not retained: the resulting explicit target can be
    /// inspected or rebuilt without re-executing authoring code.
    pub fn by(input: &T, edit: impl FnOnce(&mut T)) -> Self {
        let mut target = input.clone();
        edit(&mut target);
        Self { target }
    }
}

/// Prepared interpolation states and derived aligned geometry.
pub struct MorphEval<T> {
    src: T,
    dst: T,
    aligned_src: T,
    aligned_dst: T,
}

impl<T: MorphRequirement + Send + Sync + 'static> AnimRecipe<T> for Morph<T> {
    fn build(&self, input: &T) -> AnimNode {
        let mut aligned_src = input.clone();
        let mut aligned_dst = self.target.clone();
        if !aligned_src.is_aligned(&aligned_dst) {
            aligned_src.align_with(&mut aligned_dst);
        }
        AnimNode::from_eval(
            MorphEval {
                src: input.clone(),
                dst: self.target.clone(),
                aligned_src,
                aligned_dst,
            },
            1.0,
        )
    }
}
impl<T: MorphRequirement + Send + Sync + 'static> AnimWriteBack<T> for Morph<T> {
    fn end_value(&self, _input: &T) -> T {
        self.target.clone()
    }
}

impl<T: MorphRequirement> Eval for MorphEval<T> {
    type Output = T;
    fn eval_alpha(&self, alpha: f64) -> T {
        if alpha == 0.0 {
            self.src.clone()
        } else if (0.0..1.0).contains(&alpha) {
            self.aligned_src.lerp(&self.aligned_dst, alpha)
        } else if alpha == 1.0 {
            self.dst.clone()
        } else {
            unreachable!()
        }
    }
}
