//! Opacity animation parameters and prepared evaluators.

use ranim_core::{
    animation::{AnimRecipe, AnimWriteBack, eval::Eval, node::AnimNode},
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

impl<T: FadingRequirement + Send + Sync + 'static> AnimRecipe<T> for FadeIn {
    fn build(&self, input: &T) -> AnimNode {
        let mut src = input.clone();
        src.set_opacity(0.0);
        AnimNode::from_eval(
            FadeEval {
                src,
                dst: input.clone(),
            },
            1.0,
        )
    }
}
impl<T: FadingRequirement + Send + Sync + 'static> AnimWriteBack<T> for FadeIn {
    fn end_value(&self, input: &T) -> T {
        input.clone()
    }
}

impl<T: FadingRequirement + Send + Sync + 'static> AnimRecipe<T> for FadeOut {
    fn build(&self, input: &T) -> AnimNode {
        let mut dst = input.clone();
        dst.set_opacity(0.0);
        AnimNode::from_eval(
            FadeEval {
                src: input.clone(),
                dst,
            },
            1.0,
        )
    }
}
impl<T: FadingRequirement + Send + Sync + 'static> AnimWriteBack<T> for FadeOut {
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
