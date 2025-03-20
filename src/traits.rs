use std::ops::Range;

use color::{AlphaColor, ColorSpace, Srgb};
use glam::Mat4;

// MARK: Interpolatable
pub trait Interpolatable {
    fn lerp(&self, target: &Self, t: f32) -> Self;
}

impl Interpolatable for f32 {
    fn lerp(&self, target: &Self, t: f32) -> Self {
        self + (target - self) * t
    }
}

impl<CS: ColorSpace> Interpolatable for AlphaColor<CS> {
    fn lerp(&self, other: &Self, t: f32) -> Self {
        // TODO: figure out to use `lerp_rect` or `lerp`
        AlphaColor::lerp_rect(*self, *other, t)
    }
}

impl Interpolatable for Mat4 {
    fn lerp(&self, other: &Self, t: f32) -> Self {
        let mut result = Mat4::ZERO;
        for i in 0..4 {
            for j in 0..4 {
                result.col_mut(i)[j] = self.col(i)[j].lerp(&other.col(i)[j], t);
            }
        }
        result
    }
}

// MARK: Alignable
/// A trait for aligning two items
///
/// Alignment is actually the meaning of preparation for interpolation.
///
/// For example, if we want to interpolate two VItems, we need to
/// align all their inner components like `ComponentVec<VPoint>` to the same length.
pub trait Alignable {
    fn is_aligned(&self, other: &Self) -> bool;
    fn align_with(&mut self, other: &mut Self);
}

// MARK: Opacity
pub trait Opacity {
    fn set_opacity(&mut self, opacity: f32) -> &mut Self;
}

// MARK: Partial
pub trait Partial {
    fn get_partial(&self, range: Range<f32>) -> Self;
}

// MARK: Empty

pub trait Empty {
    fn empty() -> Self;
}

// MARK: Fill
pub trait Fill {
    fn set_fill_opacity(&mut self, opacity: f32) -> &mut Self;
    fn fill_color(&self) -> AlphaColor<Srgb>;
    fn set_fill_color(&mut self, color: AlphaColor<Srgb>) -> &mut Self;
}

// MARK: Stroke
pub trait Stroke {
    fn set_stroke_width(&mut self, width: f32) -> &mut Self;
    fn set_stroke_color(&mut self, color: AlphaColor<Srgb>) -> &mut Self;
    fn set_stroke_opacity(&mut self, opacity: f32) -> &mut Self;
}

// MARK: Color
pub trait Color: Fill + Stroke {
    fn set_color(&mut self, color: AlphaColor<Srgb>) -> &mut Self {
        self.set_fill_color(color);
        self.set_stroke_color(color);
        self
    }
}

impl<T: Fill + Stroke> Color for T {}
