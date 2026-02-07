mod rotate;
mod scale;
mod shift;

pub use rotate::*;
pub use scale::*;
pub use shift::*;

use std::ops::Range;

use color::{AlphaColor, ColorSpace, Srgb};
use glam::{DAffine2, DMat4, DQuat, DVec2, DVec3, USizeVec3, Vec3Swizzles, dvec3};
use num::complex::Complex64;

use crate::{
    anchor::{Aabb, AabbPoint, Locate, Pivot},
    components::width::Width,
    utils::resize_preserving_order_with_repeated_indices,
};

// MARK: With
/// A trait for mutating a value in place.
///
/// This trait is automatically implemented for `T`.
///
/// # Example
/// ```
/// use ranim::prelude::*;
///
/// let mut a = 1;
/// a = a.with(|x| *x = 2);
/// assert_eq!(a, 2);
/// ```
pub trait With {
    /// Mutating a value inplace
    fn with(mut self, f: impl Fn(&mut Self)) -> Self
    where
        Self: Sized,
    {
        f(&mut self);
        self
    }
}

impl<T> With for T {}

/// A trait for discarding a value.
///
/// It is useful when you want a short closure:
/// ```
/// let x = Square::new(1.0).with(|x| {
///     x.set_color(manim::BLUE_C);
/// });
/// let x = Square::new(1.0).with(|x|
///     x.set_color(manim::BLUE_C).discard()
/// );
/// ```
pub trait Discard {
    /// Simply returns `()`
    fn discard(&self) -> () {}
}

impl<T> Discard for T {}

// MARK: Interpolatable
/// A trait for interpolating to values
///
/// It uses the reference of two values and produce an owned interpolated value.
pub trait Interpolatable {
    /// Lerping between values
    fn lerp(&self, target: &Self, t: f64) -> Self;
}

impl Interpolatable for f32 {
    fn lerp(&self, target: &Self, t: f64) -> Self {
        self + (target - self) * t as f32
    }
}

impl Interpolatable for f64 {
    fn lerp(&self, target: &Self, t: f64) -> Self {
        self + (target - self) * t
    }
}

impl Interpolatable for DVec3 {
    fn lerp(&self, target: &Self, t: f64) -> Self {
        self + (target - self) * t
    }
}

impl Interpolatable for DVec2 {
    fn lerp(&self, target: &Self, t: f64) -> Self {
        self + (target - self) * t
    }
}

impl Interpolatable for DQuat {
    fn lerp(&self, target: &Self, t: f64) -> Self {
        self.slerp(*target, t)
    }
}

impl<CS: ColorSpace> Interpolatable for AlphaColor<CS> {
    fn lerp(&self, other: &Self, t: f64) -> Self {
        // TODO: figure out to use `lerp_rect` or `lerp`
        AlphaColor::lerp_rect(*self, *other, t as f32)
    }
}

impl Interpolatable for DMat4 {
    fn lerp(&self, other: &Self, t: f64) -> Self {
        let mut result = DMat4::ZERO;
        for i in 0..4 {
            for j in 0..4 {
                result.col_mut(i)[j] = self.col(i)[j].lerp(&other.col(i)[j], t);
            }
        }
        result
    }
}

impl<T: Interpolatable> Interpolatable for Vec<T> {
    fn lerp(&self, target: &Self, t: f64) -> Self {
        self.iter().zip(target).map(|(a, b)| a.lerp(b, t)).collect()
    }
}

impl<T: Opacity + Alignable + Clone> Alignable for Vec<T> {
    fn is_aligned(&self, other: &Self) -> bool {
        self.len() == other.len() && self.iter().zip(other).all(|(a, b)| a.is_aligned(b))
    }
    fn align_with(&mut self, other: &mut Self) {
        let len = self.len().max(other.len());

        let transparent_repeated = |items: &mut Vec<T>, repeat_idxs: Vec<usize>| {
            for idx in repeat_idxs {
                items[idx].set_opacity(0.0);
            }
        };
        if self.len() != len {
            let (mut items, idxs) = resize_preserving_order_with_repeated_indices(self, len);
            transparent_repeated(&mut items, idxs);
            *self = items;
        }
        if other.len() != len {
            let (mut items, idxs) = resize_preserving_order_with_repeated_indices(other, len);
            transparent_repeated(&mut items, idxs);
            *other = items;
        }
        self.iter_mut()
            .zip(other)
            .for_each(|(a, b)| a.align_with(b));
    }
}

// MARK: Alignable
/// A trait for aligning two items
///
/// Alignment is actually the meaning of preparation for interpolation.
///
/// For example, if we want to interpolate two VItems, we need to
/// align all their inner components like `ComponentVec<VPoint>` to the same length.
pub trait Alignable: Clone {
    /// Checking if two items are aligned
    fn is_aligned(&self, other: &Self) -> bool;
    /// Aligning two items
    fn align_with(&mut self, other: &mut Self);
}

impl Alignable for DVec3 {
    fn align_with(&mut self, _other: &mut Self) {}
    fn is_aligned(&self, _other: &Self) -> bool {
        true
    }
}

// MARK: Opacity
/// A trait for items with opacity
pub trait Opacity {
    /// Setting opacity of an item
    fn set_opacity(&mut self, opacity: f32) -> &mut Self;
}

impl<T: Opacity, I> Opacity for I
where
    for<'a> &'a mut I: IntoIterator<Item = &'a mut T>,
{
    fn set_opacity(&mut self, opacity: f32) -> &mut Self {
        self.into_iter().for_each(|x: &mut T| {
            x.set_opacity(opacity);
        });
        self
    }
}

// MARK: Partial
/// A trait for items that can be displayed partially
pub trait Partial {
    /// Getting a partial item
    fn get_partial(&self, range: Range<f64>) -> Self;
    /// Getting a partial item closed
    fn get_partial_closed(&self, range: Range<f64>) -> Self;
}

// MARK: Empty
/// A trait for items that can be empty
pub trait Empty {
    /// Getting an empty item
    fn empty() -> Self;
}

// MARK: FillColor
/// A trait for items that have fill color
pub trait FillColor {
    /// Getting fill color of an item
    fn fill_color(&self) -> AlphaColor<Srgb>;
    /// Setting fill opacity of an item
    fn set_fill_opacity(&mut self, opacity: f32) -> &mut Self;
    /// Setting fill color(rgba) of an item
    fn set_fill_color(&mut self, color: AlphaColor<Srgb>) -> &mut Self;
}

impl<T: FillColor> FillColor for [T] {
    fn fill_color(&self) -> color::AlphaColor<color::Srgb> {
        self[0].fill_color()
    }
    fn set_fill_color(&mut self, color: color::AlphaColor<color::Srgb>) -> &mut Self {
        self.iter_mut().for_each(|x| {
            x.set_fill_color(color);
        });
        self
    }
    fn set_fill_opacity(&mut self, opacity: f32) -> &mut Self {
        self.iter_mut().for_each(|x| {
            x.set_fill_opacity(opacity);
        });
        self
    }
}

// MARK: StrokeColor
/// A trait for items that have stroke color
pub trait StrokeColor {
    /// Getting stroke color of an item
    fn stroke_color(&self) -> AlphaColor<Srgb>;
    /// Setting stroke opacity of an item
    fn set_stroke_opacity(&mut self, opacity: f32) -> &mut Self;
    /// Setting stroke color(rgba) of an item
    fn set_stroke_color(&mut self, color: AlphaColor<Srgb>) -> &mut Self;
}

impl<T: StrokeColor> StrokeColor for [T] {
    fn stroke_color(&self) -> AlphaColor<Srgb> {
        self[0].stroke_color()
    }
    fn set_stroke_color(&mut self, color: color::AlphaColor<color::Srgb>) -> &mut Self {
        self.iter_mut().for_each(|x| {
            x.set_stroke_color(color);
        });
        self
    }
    fn set_stroke_opacity(&mut self, opacity: f32) -> &mut Self {
        self.iter_mut().for_each(|x| {
            x.set_stroke_opacity(opacity);
        });
        self
    }
}

// MARK: StrokeWidth
/// A trait for items have stroke width
pub trait StrokeWidth {
    // TODO: Make this better
    /// Get the stroke width
    fn stroke_width(&self) -> f32;
    /// Applying stroke width function to an item
    fn apply_stroke_func(&mut self, f: impl for<'a> Fn(&'a mut [Width])) -> &mut Self;
    /// Setting stroke width of an item
    fn set_stroke_width(&mut self, width: f32) -> &mut Self {
        self.apply_stroke_func(|widths| widths.fill(width.into()))
    }
}

impl<T: StrokeWidth> StrokeWidth for [T] {
    fn stroke_width(&self) -> f32 {
        self[0].stroke_width()
    }
    fn apply_stroke_func(
        &mut self,
        f: impl for<'a> Fn(&'a mut [crate::components::width::Width]),
    ) -> &mut Self {
        self.iter_mut().for_each(|x| {
            x.apply_stroke_func(&f);
        });
        self
    }
}

// MARK: Color
/// A trait for items that have both fill color and stroke color
///
/// This trait is auto implemented for items that implement [`FillColor`] and [`StrokeColor`].
pub trait Color: FillColor + StrokeColor {
    /// Setting color(rgba) of an item
    fn set_color(&mut self, color: AlphaColor<Srgb>) -> &mut Self {
        self.set_fill_color(color);
        self.set_stroke_color(color);
        self
    }
}

impl<T: FillColor + StrokeColor + ?Sized> Color for T {}

// MARK: PointsFunc
/// A trait for items that can apply points function.
pub trait PointsFunc {
    /// Applying points function to an item
    fn apply_points_func(&mut self, f: impl for<'a> Fn(&'a mut [DVec3])) -> &mut Self;

    /// Applying affine transform to an item
    fn apply_affine(&mut self, affine: DAffine2) -> &mut Self {
        self.apply_points_func(|points| {
            points.iter_mut().for_each(|p| {
                let transformed = affine.transform_point2(p.xy());
                p.x = transformed.x;
                p.y = transformed.y;
            });
        });
        self
    }

    /// Applying point function to an item
    fn apply_point_func(&mut self, f: impl Fn(&mut DVec3)) -> &mut Self {
        self.apply_points_func(|points| {
            points.iter_mut().for_each(&f);
        });
        self
    }
    /// Applying point function to an item
    fn apply_point_map(&mut self, f: impl Fn(DVec3) -> DVec3) -> &mut Self {
        self.apply_points_func(|points| {
            points.iter_mut().for_each(|p| *p = f(*p));
        });
        self
    }

    /// Applying complex function to an item.
    ///
    /// The point's x and y coordinates will be used as real and imaginary parts of a complex number.
    fn apply_complex_func(&mut self, f: impl Fn(&mut Complex64)) -> &mut Self {
        self.apply_point_func(|p| {
            let mut c = Complex64::new(p.x, p.y);
            f(&mut c);
            p.x = c.re;
            p.y = c.im;
        });
        self
    }
    /// Applying complex function to an item.
    ///
    /// The point's x and y coordinates will be used as real and imaginary parts of a complex number.
    fn apply_complex_map(&mut self, f: impl Fn(Complex64) -> Complex64) -> &mut Self {
        self.apply_complex_func(|p| {
            *p = f(*p);
        });
        self
    }
}

impl PointsFunc for DVec3 {
    fn apply_points_func(&mut self, f: impl for<'a> Fn(&'a mut [DVec3])) -> &mut Self {
        f(std::slice::from_mut(self));
        self
    }
}

impl<T: PointsFunc> PointsFunc for [T] {
    fn apply_points_func(&mut self, f: impl for<'a> Fn(&'a mut [DVec3])) -> &mut Self {
        self.iter_mut()
            .for_each(|x| x.apply_points_func(&f).discard());
        self
    }
}

// MARK: Align
pub trait AlignSlice<T: Shift>: AsMut<[T]> {
    /// Align items' anchors in a given axis, based on the first item.
    fn align_anchor<A: Clone>(&mut self, axis: DVec3, anchor: A) -> &mut Self
    where
        T: Locate<A>,
    {
        let Some(dir) = axis.try_normalize() else {
            return self;
        };
        let Some(point) = self.as_mut().first().map(|x| x.locate(anchor.clone())) else {
            return self;
        };

        self.as_mut().iter_mut().for_each(|x| {
            let p = Locate::<A>::locate(x, anchor.clone());

            let v = p - point;
            let proj = dir * v.dot(dir);
            let closest = point + proj;
            let displacement = closest - p;
            x.shift(displacement);
        });
        self
    }
    /// Align items' centers in a given axis, based on the first item.
    fn align(&mut self, axis: DVec3) -> &mut Self
    where
        T: Locate<Pivot>,
    {
        self.align_anchor(axis, Pivot)
    }
}

// MARK: Arrange
/// A trait for arranging operations.
pub trait ArrangeSlice<T: Shift>: AsMut<[T]> {
    /// Arrange the items by a given function.
    ///
    /// The `pos_func` takes index as input and output the center position.
    fn arrange_with(&mut self, pos_func: impl Fn(usize) -> DVec3)
    where
        T: Locate<Pivot>,
    {
        self.as_mut().iter_mut().enumerate().for_each(|(i, x)| {
            x.move_to(pos_func(i));
        });
    }
    fn arrange_in_y(&mut self, gap: f64)
    where
        T: Aabb,
    {
        let Some(mut bbox) = self.as_mut().first().map(|x| x.aabb()) else {
            return;
        };

        self.as_mut().iter_mut().for_each(|x| {
            x.move_next_to_padded(bbox.as_slice(), AabbPoint(DVec3::Y), gap);
            bbox = x.aabb();
        });
    }
    /// Arrange the items in a grid.
    fn arrange_in_grid(&mut self, cell_cnt: USizeVec3, cell_size: DVec3, gap: DVec3) -> &mut Self
    where
        T: Locate<Pivot>,
    {
        // x -> y -> z
        let pos_func = |idx: usize| {
            let x = idx % cell_cnt.x;
            let temp = idx / cell_cnt.x;

            let y = temp % cell_cnt.y;
            let z = temp / cell_cnt.y;
            dvec3(x as f64, y as f64, z as f64) * cell_size
                + gap * dvec3(x as f64, y as f64, z as f64)
        };
        self.arrange_with(pos_func);
        self
    }
    /// Arrange the items in a grid with given number of columns.
    ///
    /// The `pos_func` takes row and column index as input and output the center position.
    fn arrange_in_cols_with(&mut self, ncols: usize, pos_func: impl Fn(usize, usize) -> DVec3)
    where
        T: Locate<Pivot>,
    {
        let pos_func = |idx: usize| {
            let row = idx / ncols;
            let col = idx % ncols;
            pos_func(row, col)
        };
        self.arrange_with(pos_func);
    }
    /// Arrange the items in a grid with given number of rows.
    ///
    /// The `pos_func` takes row and column index as input and output the center position.
    fn arrange_in_rows_with(&mut self, nrows: usize, pos_func: impl Fn(usize, usize) -> DVec3)
    where
        T: Locate<Pivot>,
    {
        let ncols = self.as_mut().len().div_ceil(nrows);
        self.arrange_in_cols_with(ncols, pos_func);
    }
}

impl<T: Shift, E: AsMut<[T]>> ArrangeSlice<T> for E {}
