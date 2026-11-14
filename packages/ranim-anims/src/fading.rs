//! Opacity animation parameters and prepared evaluators.

use ranim_core::{
    animation::{definition::AnimRecipe, eval::Eval},
    traits::{Interpolatable, Opacity},
};

/// Item capabilities needed for opacity interpolation.
pub trait FadingRequirement: Opacity + Interpolatable + Clone {}
impl<T: Opacity + Interpolatable + Clone> FadingRequirement for T {}

/// Fade from transparent to the input item's original opacity.
#[derive(Clone, Copy, Debug, Default, ranim_macros::Animation)]
pub struct FadeIn;

/// Fade from the input item's opacity to transparent.
#[derive(Clone, Copy, Debug, Default, ranim_macros::Animation)]
pub struct FadeOut;

/// Prepared opacity interpolation; derived states are not construction parameters.
pub struct FadeEval<T> {
    src: T,
    dst: T,
}

impl<T: FadingRequirement> AnimRecipe<T> for FadeIn {
    type Animation = FadeEval<T>;
    fn build(&self, input: &T) -> Self::Animation {
        let mut src = input.clone();
        src.set_opacity(0.0);
        FadeEval {
            src,
            dst: input.clone(),
        }
    }
    fn end_value(&self, input: &T) -> T {
        input.clone()
    }
}

impl<T: FadingRequirement> AnimRecipe<T> for FadeOut {
    type Animation = FadeEval<T>;
    fn build(&self, input: &T) -> Self::Animation {
        let mut dst = input.clone();
        dst.set_opacity(0.0);
        FadeEval {
            src: input.clone(),
            dst,
        }
    }
    fn end_value(&self, input: &T) -> T {
        let mut result = input.clone();
        result.set_opacity(0.0);
        result
    }
}

impl<T: FadingRequirement> Eval for FadeEval<T> {
    type Output = T;
    fn eval_alpha(&self, alpha: f64) -> T {
        self.src.lerp(&self.dst, alpha)
    }
}
