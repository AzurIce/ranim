//! Creation and writing operation parameters.

use ranim_core::{
    animation::{definition::AnimRecipe, eval::Eval},
    traits::{Empty, FillColor, Interpolatable, Partial, StrokeColor, StrokeWidth},
};
use ranim_items::vitem::DEFAULT_STROKE_WIDTH;

/// Item capabilities needed for progressive creation.
pub trait CreationRequirement: Clone + Partial + Empty + Interpolatable {}
impl<T: Clone + Partial + Empty + Interpolatable> CreationRequirement for T {}

/// Item capabilities needed for outline-then-fill writing.
pub trait WritingRequirement: CreationRequirement + StrokeWidth + StrokeColor + FillColor {}
impl<T: CreationRequirement + StrokeWidth + StrokeColor + FillColor> WritingRequirement for T {}

/// Reveal an item progressively.
#[derive(Clone, Copy, Debug, Default, ranim_macros::Animation)]
pub struct Create;
/// Remove an item progressively.
#[derive(Clone, Copy, Debug, Default, ranim_macros::Animation)]
pub struct UnCreate;
/// Draw the outline, then restore the original fill.
#[derive(Clone, Copy, Debug, Default, ranim_macros::Animation)]
pub struct Write;
/// Remove the fill, then erase the outline.
#[derive(Clone, Copy, Debug, Default, ranim_macros::Animation)]
pub struct Unwrite;

/// Prepared progressive creation or removal.
pub struct CreateEval<T> {
    original: T,
    reverse: bool,
}

impl<T: CreationRequirement> AnimRecipe<T> for Create {
    type Animation = CreateEval<T>;
    fn build(&self, input: &T) -> Self::Animation {
        CreateEval {
            original: input.clone(),
            reverse: false,
        }
    }
    fn end_value(&self, input: &T) -> T {
        input.clone()
    }
}
impl<T: CreationRequirement> AnimRecipe<T> for UnCreate {
    type Animation = CreateEval<T>;
    fn build(&self, input: &T) -> Self::Animation {
        CreateEval {
            original: input.clone(),
            reverse: true,
        }
    }
    fn end_value(&self, _input: &T) -> T {
        T::empty()
    }
}
impl<T: CreationRequirement> Eval for CreateEval<T> {
    type Output = T;
    fn eval_alpha(&self, alpha: f64) -> T {
        let alpha = if self.reverse && !(0.0..=1.0).contains(&alpha) {
            tracing::warn!("the alpha is out of range: {alpha}, clamped to 0.0..=1.0");
            alpha.clamp(0.0, 1.0)
        } else {
            alpha
        };
        let progress = if self.reverse { 1.0 - alpha } else { alpha };
        if progress == 0.0 {
            T::empty()
        } else if (0.0..1.0).contains(&progress) {
            self.original.get_partial_closed(0.0..progress)
        } else if progress == 1.0 {
            self.original.clone()
        } else {
            unreachable!()
        }
    }
}

/// Prepared writing geometry, including the derived outline.
pub struct WriteEval<T> {
    original: T,
    outline: T,
    reverse: bool,
}

fn prepare_write<T: WritingRequirement>(input: &T, reverse: bool) -> WriteEval<T> {
    let mut outline = input.clone();
    outline.set_fill_opacity(0.0).set_stroke_opacity(1.0);
    if outline.stroke_width() == 0.0 {
        outline.set_stroke_width(DEFAULT_STROKE_WIDTH);
    }
    WriteEval {
        original: input.clone(),
        outline,
        reverse,
    }
}

impl<T: WritingRequirement> AnimRecipe<T> for Write {
    type Animation = WriteEval<T>;
    fn build(&self, input: &T) -> Self::Animation {
        prepare_write(input, false)
    }
    fn end_value(&self, input: &T) -> T {
        input.clone()
    }
}
impl<T: WritingRequirement> AnimRecipe<T> for Unwrite {
    type Animation = WriteEval<T>;
    fn build(&self, input: &T) -> Self::Animation {
        prepare_write(input, true)
    }
    fn end_value(&self, _input: &T) -> T {
        T::empty()
    }
}

impl<T: WritingRequirement> Eval for WriteEval<T> {
    type Output = T;
    fn eval_alpha(&self, alpha: f64) -> T {
        if self.reverse {
            let alpha = alpha * 2.0;
            if (0.0..1.0).contains(&alpha) {
                self.original.lerp(&self.outline, alpha)
            } else if alpha == 1.0 {
                self.outline.clone()
            } else if (1.0..2.0).contains(&alpha) {
                self.outline.get_partial(0.0..2.0 - alpha)
            } else if alpha == 2.0 {
                T::empty()
            } else {
                unreachable!()
            }
        } else {
            let alpha = alpha * 2.0;
            if (0.0..1.0).contains(&alpha) {
                self.outline.get_partial(0.0..alpha)
            } else if alpha == 1.0 {
                self.outline.clone()
            } else if (1.0..2.0).contains(&alpha) {
                self.outline.lerp(&self.original, alpha - 1.0)
            } else if alpha == 2.0 {
                self.original.clone()
            } else {
                unreachable!()
            }
        }
    }
}
