use std::{cmp::Ordering, ops::Range};

use color::{AlphaColor, ColorSpace, Srgb};
use glam::{
    DAffine2, DMat3, DMat4, DQuat, DVec2, DVec3, IVec3, USizeVec3, Vec3Swizzles, dvec3, ivec3,
};
use itertools::Itertools;
use num::complex::Complex64;
use tracing::warn;

use crate::{components::width::Width, utils::resize_preserving_order_with_repeated_indices};

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

pub trait SomeNamePoint<T> {
    fn get_point(&self, point: T) -> DVec3;
}

impl<T: Aabb + ?Sized> SomeNamePoint<AabbPoint> for T {
    fn get_point(&self, point: AabbPoint) -> DVec3 {
        let center = self.aabb_center();
        let half_size = self.aabb_size() / 2.0;
        center + point.0 * half_size
    }
}
// pub trait AnchorPoint<T> {}

// MARK: BoundingBox
/// A trait for items that have a bounding box
pub trait Aabb {
    /// Get the Axis-aligned bounding box represent in `[<min>, <max>]`.
    fn aabb(&self) -> [DVec3; 2];
    /// Get the size of the Aabb.
    fn aabb_size(&self) -> DVec3 {
        let [min, max] = self.aabb();
        max - min
    }
    /// Get the center of the Aabb.
    fn aabb_center(&self) -> DVec3 {
        let [min, max] = self.aabb();
        (max + min) / 2.0
    }
}

impl Aabb for DVec3 {
    fn aabb(&self) -> [DVec3; 2] {
        [*self; 2]
    }
}

impl<T: Aabb> Aabb for [T] {
    fn aabb(&self) -> [DVec3; 2] {
        let [min, max] = self
            .iter()
            .map(|x| x.aabb())
            .reduce(|[acc_min, acc_max], [min, max]| [acc_min.min(min), acc_max.max(max)])
            .unwrap_or([DVec3::ZERO, DVec3::ZERO]);
        if min == max {
            warn!("Empty bounding box, is the slice empty?")
        }
        [min, max]
    }
}

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

// MARK: Shift
/// A trait for shifting operations.
///
/// To implement this trait, you need to implement the [`BoundingBox`] trait first.
pub trait Shift: Aabb {
    /// Shift the item by a given vector.
    fn shift(&mut self, offset: DVec3) -> &mut Self;
    /// Put anchor at a given point.
    ///
    /// See [`Anchor`] for more details.
    fn move_anchor_to<T>(&mut self, anchor_point: T, point: DVec3) -> &mut Self
    where
        Self: SomeNamePoint<T>,
    {
        self.shift(point - self.get_point(anchor_point));
        self
    }
    /// Put center at a given point.
    fn move_to(&mut self, point: DVec3) -> &mut Self {
        self.move_anchor_to(AabbPoint::CENTER, point)
    }
    /// Put negative anchor of self on anchor of target
    fn move_next_to<T: Aabb + ?Sized>(&mut self, target: &T, anchor: AabbPoint) -> &mut Self {
        self.move_next_to_padded(target, anchor, 0.0)
    }
    /// Put negative anchor of self on anchor of target, with a distance of `padding`
    fn move_next_to_padded<T: Aabb + ?Sized>(
        &mut self,
        target: &T,
        anchor: AabbPoint,
        padding: f64,
    ) -> &mut Self {
        let neg_anchor = AabbPoint(-anchor.0);
        self.move_anchor_to(
            neg_anchor,
            anchor.get_pos(target) + anchor.0.normalize() * padding,
        )
    }
}

impl<T: Shift> Shift for [T] {
    fn shift(&mut self, shift: DVec3) -> &mut Self {
        self.iter_mut().for_each(|x| {
            x.shift(shift);
        });
        self
    }
}

// MARK: Rotate
/// A trait for rotating operations
pub trait Rotate {
    /// Rotate the item by a given angle about a given axis at anchor.
    ///
    /// See [`Anchor`]
    fn rotate_at(&mut self, angle: f64, axis: DVec3, anchor_point: impl AnchorPoint) -> &mut Self;
    /// Rotate the mobject by a given angle about a given axis at center.
    ///
    /// This is equivalent to [`Rotate::rotate_by_anchor`] with [`Anchor::CENTER`].
    fn rotate(&mut self, angle: f64, axis: DVec3) -> &mut Self {
        self.rotate_at(angle, axis, AabbPoint::CENTER)
    }
}

impl<T: Rotate + Aabb> Rotate for [T] {
    fn rotate_at(&mut self, angle: f64, axis: DVec3, anchor_point: impl AnchorPoint) -> &mut Self {
        let anchor = anchor_point.get_pos(self);
        self.iter_mut().for_each(|x| {
            x.rotate_at(angle, axis, anchor);
        });
        self
    }
}

// MARK: Anchor
/// A point based on [`Aabb`], the number in each axis means the fraction of the size of the [`Aabb`].
/// (0, 0, 0) is the center point.
/// ```text
///      +Y
///      |
///      |
///      +----- +X
///    /
/// +Z
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AabbPoint(pub DVec3);

impl AabbPoint {
    /// Center point, shorthand of `Anchor(DVec3::ZERO)`.
    pub const CENTER: Self = Self(DVec3::ZERO);
}

pub trait AnchorPoint {
    fn get_pos<T: Aabb + ?Sized>(&self, bbox: &T) -> DVec3;
}

impl AnchorPoint for DVec3 {
    fn get_pos<T: Aabb + ?Sized>(&self, _bbox: &T) -> DVec3 {
        *self
    }
}

impl AnchorPoint for AabbPoint {
    fn get_pos<T: Aabb + ?Sized>(&self, bbox: &T) -> DVec3 {
        let bbox = bbox.aabb();
        let pos = bbox[1] + self.0 * bbox.aabb_size() * 0.5;
        pos
    }
}

/// Apply the function by first transform the points to origin based on a point,
/// then apply the function, then transform the points back.
pub fn wrap_point_func_with_point(
    f: impl Fn(&mut DVec3) + Copy,
    point: DVec3,
) -> impl Fn(&mut DVec3) + Copy {
    move |points| {
        *points -= point;
        f(points);
        *points += point;
    }
}

// MARK: ScaleHint
/// A hint for scaling the mobject.
#[derive(Debug, Clone, Copy)]
pub enum ScaleHint {
    /// Scale the mobject's X axe
    X(f64),
    /// Scale the mobject's Y axe
    Y(f64),
    /// Scale the mobject's Z axe
    Z(f64),
    /// Scale the mobject's X axe, while other axes are scaled accordingly.
    PorportionalX(f64),
    /// Scale the mobject's Y axe, while other axes are scaled accordingly.
    PorportionalY(f64),
    /// Scale the mobject's Z axe, while other axes are scaled accordingly.
    PorportionalZ(f64),
}

// MARK: Scale
/// A trait for scaling operations
pub trait Scale: Aabb {
    /// Scale the item by a given scale at anchor.
    ///
    /// See [`Anchor`]
    fn scale_at(&mut self, scale: DVec3, anchor_point: impl AnchorPoint) -> &mut Self;
    /// Scale the item by a given scale at center.
    ///
    /// This is equivalent to [`Scale::scale_by_anchor`] with [`Anchor::CENTER`].
    fn scale(&mut self, scale: DVec3) -> &mut Self {
        self.scale_at(scale, AabbPoint::CENTER)
    }
    /// Calculate the scale ratio for a given hint.
    ///
    /// See [`ScaleHint`] for more details.
    fn calc_scale_ratio(&self, hint: ScaleHint) -> DVec3 {
        let aabb_size = self.aabb_size();
        match hint {
            ScaleHint::X(v) => dvec3(v / (aabb_size.x), 1.0, 1.0),
            ScaleHint::Y(v) => dvec3(1.0, v / (aabb_size.y), 1.0),
            ScaleHint::Z(v) => dvec3(1.0, 1.0, v / aabb_size.z),
            ScaleHint::PorportionalX(v) => DVec3::splat(v / aabb_size.x),
            ScaleHint::PorportionalY(v) => DVec3::splat(v / aabb_size.y),
            ScaleHint::PorportionalZ(v) => DVec3::splat(v / aabb_size.z),
        }
    }
    /// Scale the item to a given hint.
    ///
    /// See [`ScaleHint`] for more details.
    fn scale_to(&mut self, hint: ScaleHint) -> &mut Self {
        self.scale(self.calc_scale_ratio(hint));
        self
    }
    /// Scale the item to the minimum scale ratio of each axis from the given hints.
    ///
    /// See [`ScaleHint`] for more details.
    fn scale_to_min(&mut self, hints: &[ScaleHint]) -> &mut Self {
        let scale = hints
            .iter()
            .map(|hint| self.calc_scale_ratio(*hint))
            .reduce(|a, b| a.min(b))
            .unwrap_or(DVec3::ONE);
        self.scale(scale);
        self
    }
    /// Scale the item to the maximum scale ratio of each axis from the given hints.
    ///
    /// See [`ScaleHint`] for more details.
    fn scale_to_max(&mut self, hints: &[ScaleHint]) -> &mut Self {
        let scale = hints
            .iter()
            .map(|hint| self.calc_scale_ratio(*hint))
            .reduce(|a, b| a.max(b))
            .unwrap_or(DVec3::ONE);
        self.scale(scale);
        self
    }
}

impl<T: Scale> Scale for [T] {
    fn scale_at(&mut self, scale: DVec3, anchor_point: impl AnchorPoint) -> &mut Self {
        let p = anchor_point.get_pos(self);
        self.iter_mut().for_each(|x| {
            x.scale_at(scale, p);
        });
        self
    }
}

impl Shift for DVec3 {
    fn shift(&mut self, shift: DVec3) -> &mut Self {
        *self += shift;
        self
    }
}

impl Rotate for DVec3 {
    fn rotate_at(&mut self, angle: f64, axis: DVec3, anchor: impl AnchorPoint) -> &mut Self {
        let rotation = DMat3::from_axis_angle(axis, angle);
        let p = anchor.get_pos(self);
        wrap_point_func_with_point(|p| *p = rotation * *p, p)(self);
        if self.x.abs() < 1e-10 {
            self.x = 0.0;
        }
        if self.y.abs() < 1e-10 {
            self.y = 0.0;
        }
        if self.z.abs() < 1e-10 {
            self.z = 0.0;
        }
        self
    }
}

impl Scale for DVec3 {
    fn scale_at(&mut self, scale: DVec3, anchor: impl AnchorPoint) -> &mut Self {
        let p = anchor.get_pos(self);
        wrap_point_func_with_point(|p| *p *= scale, p)(self);
        self
    }
}

// MARK: Align
pub trait AlignSlice<T: Shift>: AsMut<[T]> {
    /// Align items' anchors in a given axis, based on the first item.
    fn align_anchor(&mut self, axis: DVec3, anchor: AabbPoint) -> &mut Self {
        let Some(dir) = axis.try_normalize() else {
            return self;
        };
        let Some(point) = self.as_mut().first().map(|x| anchor.get_pos(x)) else {
            return self;
        };

        self.as_mut().iter_mut().for_each(|x| {
            let p = anchor.get_pos(x);

            let v = p - point;
            let proj = dir * v.dot(dir);
            let closest = point + proj;
            let displacement = closest - p;
            x.shift(displacement);
        });
        self
    }
    /// Align items' centers in a given axis, based on the first item.
    fn align(&mut self, axis: DVec3) -> &mut Self {
        self.align_anchor(axis, AabbPoint::CENTER)
    }
    // fn align(&mut self, point: DVec3, axis: DVec3) -> &mut Self {
    //     self.align_anchor_at(Anchor::CENTER, point, axis)
    // }
}

// MARK: Arrange
/// A trait for arranging operations.
pub trait ArrangeSlice<T: Shift>: AsMut<[T]> {
    /// Arrange the items by a given function.
    ///
    /// The `pos_func` takes index as input and output the center position.
    fn arrange_with(&mut self, pos_func: impl Fn(usize) -> DVec3) {
        self.as_mut().iter_mut().enumerate().for_each(|(i, x)| {
            x.move_to(pos_func(i));
        });
    }
    fn arrange_in_y(&mut self, gap: f64) {
        let Some(mut bbox) = self.as_mut().first().map(|x| x.aabb()) else {
            return;
        };

        self.as_mut().iter_mut().for_each(|x| {
            x.move_next_to_padded(bbox.as_slice(), AabbPoint(DVec3::Y), gap);
            bbox = x.aabb();
        });
    }
    /// Arrange the items in a grid.
    fn arrange_in_grid(&mut self, cell_cnt: USizeVec3, cell_size: DVec3, gap: DVec3) -> &mut Self {
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
    fn arrange_in_cols_with(&mut self, ncols: usize, pos_func: impl Fn(usize, usize) -> DVec3) {
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
    fn arrange_in_rows_with(&mut self, nrows: usize, pos_func: impl Fn(usize, usize) -> DVec3) {
        let ncols = self.as_mut().len().div_ceil(nrows);
        self.arrange_in_cols_with(ncols, pos_func);
    }
}

impl<T: Shift, E: AsMut<[T]>> ArrangeSlice<T> for E {}

// MARK: ScaleStrokeExt
/// A trait for scaling operations with stroke width.
pub trait ScaleStrokeExt: Scale + StrokeWidth {
    /// Scale the item by a given scale at anchor with stroke width.
    fn scale_with_stroke_by_anchor(
        &mut self,
        scale: DVec3,
        anchor_point: impl AnchorPoint,
    ) -> &mut Self {
        self.scale_at(scale, anchor_point);

        let scales = [scale.x, scale.y, scale.z];
        let idx = scales
            .iter()
            .map(|x: &f64| if *x > 1.0 { *x } else { 1.0 / *x })
            .position_max_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal))
            .unwrap_or(0);
        let scale = scales[idx];
        self.apply_stroke_func(|widths| widths.iter_mut().for_each(|w| w.0 *= scale as f32));
        self
    }
    /// Scale the item by a given scale with stroke width.
    fn scale_with_stroke(&mut self, scale: DVec3) -> &mut Self {
        self.scale_with_stroke_by_anchor(scale, AabbPoint::CENTER)
    }
    /// Scale the item to a given hint.
    ///
    /// See [`ScaleHint`] for more details.
    fn scale_to_with_stroke(&mut self, hint: ScaleHint) -> &mut Self {
        let scale = self.calc_scale_ratio(hint);
        self.scale_with_stroke(scale)
    }
}

impl<T: Scale + StrokeWidth + ?Sized> ScaleStrokeExt for T {}
