use std::{cmp::Ordering, ops::Range};

use color::{AlphaColor, ColorSpace, Srgb};
use glam::{DAffine2, DMat3, DMat4, DVec3, IVec3, Vec3Swizzles, dvec3, ivec3};
use itertools::Itertools;
use log::warn;

use crate::{
    components::{Anchor, ScaleHint, vpoint::wrap_point_func_with_anchor, width::Width},
    items::Group,
};

// MARK: Interpolatable
/// A trait for interpolating to values
///
/// It uses the reference of two values and produce an owned interpolated value.
pub trait Interpolatable {
    /// Lerping between values
    fn lerp(&self, target: &Self, t: f64) -> Self;
}

impl<T: Interpolatable> Interpolatable for Group<T> {
    fn lerp(&self, target: &Self, t: f64) -> Self {
        self.into_iter()
            .zip(target)
            .map(|(a, b)| a.lerp(b, t))
            .collect()
    }
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

// MARK: Alignable
/// A trait for aligning two items
///
/// Alignment is actually the meaning of preparation for interpolation.
///
/// For example, if we want to interpolate two VItems, we need to
/// align all their inner components like `ComponentVec<VPoint>` to the same length.
pub trait Alignable {
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

// TODO: make this better
impl<T: Alignable> Alignable for Group<T> {
    fn is_aligned(&self, other: &Self) -> bool {
        self.len() == other.len() && self.iter().zip(other).all(|(a, b)| a.is_aligned(b))
    }
    fn align_with(&mut self, other: &mut Self) {
        self.iter_mut().zip(other).for_each(|(a, b)| {
            a.align_with(b);
        });
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
        self.into_iter().for_each(|x| {
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
    /// Applying stroke width function to an item
    fn apply_stroke_func(&mut self, f: impl for<'a> Fn(&'a mut [Width])) -> &mut Self;
    /// Setting stroke width of an item
    fn set_stroke_width(&mut self, width: f32) -> &mut Self {
        self.apply_stroke_func(|widths| widths.fill(width.into()))
    }
}

impl<T: StrokeWidth> StrokeWidth for [T] {
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

// MARK: BoundingBox
/// A trait for items that have a bounding box
pub trait BoundingBox {
    /// Get the bounding box of the mobject in [min, mid, max] order.
    fn get_bounding_box(&self) -> [DVec3; 3];
    /// Get the bounding box point of the mobject at an edge Anchor.
    ///
    /// See [`Anchor`].
    fn get_bounding_box_point(&self, edge: IVec3) -> DVec3 {
        let bb = self.get_bounding_box();
        let signum = (edge.signum() + IVec3::ONE).as_uvec3();

        dvec3(
            bb[signum.x as usize].x,
            bb[signum.y as usize].y,
            bb[signum.z as usize].z,
        )
    }
    /// Get the bounding box corners of the mobject.
    ///
    /// The order is the cartesian product of [-1, 1] on x, y, z axis.
    /// Which is `(-1, -1, -1)`, `(-1, -1, 1)`, `(-1, 1, -1)`, `(-1, 1, 1)`, ...
    fn get_bounding_box_corners(&self) -> [DVec3; 8] {
        [-1, 1]
            .into_iter()
            .cartesian_product([-1, 1])
            .cartesian_product([-1, 1])
            .map(|((x, y), z)| self.get_bounding_box_point(ivec3(x, y, z)))
            .collect::<Vec<_>>()
            .try_into()
            .unwrap()
    }
}

impl BoundingBox for DVec3 {
    fn get_bounding_box(&self) -> [DVec3; 3] {
        [*self, *self, *self]
    }
}

impl<T: BoundingBox> BoundingBox for [T] {
    fn get_bounding_box(&self) -> [DVec3; 3] {
        let [min, max] = self
            .iter()
            .map(|x| x.get_bounding_box())
            .map(|[min, _, max]| [min, max])
            .reduce(|[acc_min, acc_max], [min, max]| [acc_min.min(min), acc_max.max(max)])
            .unwrap_or([DVec3::ZERO, DVec3::ZERO]);
        if min == max {
            warn!("Empty bounding box, is the slice empty?")
        }
        [min, (min + max) / 2.0, max]
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
}

// MARK: Shift
/// A trait for shifting operations.
pub trait Shift: BoundingBox {
    /// Shift the item by a given vector.
    fn shift(&mut self, shift: DVec3) -> &mut Self;
    /// Put anchor at a given point.
    ///
    /// See [`Anchor`] for more details.
    fn put_anchor_on(&mut self, anchor: Anchor, point: DVec3) -> &mut Self {
        self.shift(point - anchor.get_pos(self));
        self
    }
    /// Put center at a given point.
    fn put_center_on(&mut self, point: DVec3) -> &mut Self {
        self.put_anchor_on(Anchor::CENTER, point)
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
    fn rotate_by_anchor(&mut self, angle: f64, axis: DVec3, anchor: Anchor) -> &mut Self;
    /// Rotate the mobject by a given angle about a given axis at center.
    ///
    /// This is equivalent to [`Rotate::rotate_by_anchor`] with [`Anchor::CENTER`].
    fn rotate(&mut self, angle: f64, axis: DVec3) -> &mut Self {
        self.rotate_by_anchor(angle, axis, Anchor::CENTER)
    }
}

impl<T: Rotate + BoundingBox> Rotate for [T] {
    fn rotate_by_anchor(&mut self, angle: f64, axis: DVec3, anchor: Anchor) -> &mut Self {
        let anchor = Anchor::Point(anchor.get_pos(self));
        self.iter_mut().for_each(|x| {
            x.rotate_by_anchor(angle, axis, anchor);
        });
        self
    }
}

// MARK: Scale
/// A trait for scaling operations
pub trait Scale: BoundingBox {
    /// Scale the item by a given scale at anchor.
    ///
    /// See [`Anchor`]
    fn scale_by_anchor(&mut self, scale: DVec3, anchor: Anchor) -> &mut Self;
    /// Scale the item by a given scale at center.
    ///
    /// This is equivalent to [`Scale::scale_by_anchor`] with [`Anchor::CENTER`].
    fn scale(&mut self, scale: DVec3) -> &mut Self {
        self.scale_by_anchor(scale, Anchor::CENTER)
    }
    /// Calculate the scale ratio for a given hint.
    ///
    /// See [`ScaleHint`] for more details.
    fn calc_scale_ratio(&self, hint: ScaleHint) -> DVec3 {
        let bb = self.get_bounding_box();
        match hint {
            ScaleHint::X(v) => dvec3(v / (bb[2].x - bb[0].x), 1.0, 1.0),
            ScaleHint::Y(v) => dvec3(1.0, v / (bb[2].y - bb[0].y), 1.0),
            ScaleHint::Z(v) => dvec3(1.0, 1.0, v / (bb[2].z - bb[0].z)),
            ScaleHint::PorportionalX(v) => DVec3::splat(v / (bb[2].x - bb[0].x)),
            ScaleHint::PorportionalY(v) => DVec3::splat(v / (bb[2].y - bb[0].y)),
            ScaleHint::PorportionalZ(v) => DVec3::splat(v / (bb[2].z - bb[0].z)),
        }
    }
    /// Scale the item to a given hint.
    ///
    /// See [`ScaleHint`] for more details.
    fn scale_to(&mut self, hint: ScaleHint) -> &mut Self {
        self.scale(self.calc_scale_ratio(hint));
        self
    }
}

impl<T: Scale> Scale for [T] {
    fn scale_by_anchor(&mut self, scale: DVec3, anchor: Anchor) -> &mut Self {
        let anchor = match anchor {
            Anchor::Point(p) => p,
            Anchor::Edge(e) => self.get_bounding_box_point(e),
        };
        self.iter_mut().for_each(|x| {
            x.scale_by_anchor(scale, Anchor::Point(anchor));
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
    fn rotate_by_anchor(&mut self, angle: f64, axis: DVec3, anchor: Anchor) -> &mut Self {
        let rotation = DMat3::from_axis_angle(axis, angle);
        let p = match anchor {
            Anchor::Point(point) => point,
            Anchor::Edge(edge) => self.get_bounding_box_point(edge),
        };
        wrap_point_func_with_anchor(|p| *p = rotation * *p, p)(self);
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
    fn scale_by_anchor(&mut self, scale: DVec3, anchor: Anchor) -> &mut Self {
        let p = match anchor {
            Anchor::Point(point) => point,
            Anchor::Edge(edge) => self.get_bounding_box_point(edge),
        };
        wrap_point_func_with_anchor(|p| *p *= scale, p)(self);
        self
    }
}

// MARK: Arrange
/// A trait for arranging operations.
pub trait Arrange: Shift {
    /// Arrange the items by a given function.
    ///
    /// The `pos_func` takes index as input and output the center position.
    fn arrange(&mut self, pos_func: impl Fn(usize) -> DVec3);
    /// Arrange the items in a grid with given number of columns.
    ///
    /// The `pos_func` takes row and column index as input and output the center position.
    fn arrange_cols(&mut self, ncols: usize, pos_func: impl Fn(usize, usize) -> DVec3);
    /// Arrange the items in a grid with given number of rows.
    ///
    /// The `pos_func` takes row and column index as input and output the center position.
    fn arrange_rows(&mut self, nrows: usize, pos_func: impl Fn(usize, usize) -> DVec3);
}

impl<T: Shift> Arrange for [T] {
    fn arrange(&mut self, pos_func: impl Fn(usize) -> DVec3) {
        self.iter_mut().enumerate().for_each(|(i, item)| {
            item.put_center_on(pos_func(i));
        });
    }
    fn arrange_cols(&mut self, ncols: usize, pos_func: impl Fn(usize, usize) -> DVec3) {
        let pos_func = |idx: usize| {
            let row = idx / ncols;
            let col = idx % ncols;
            pos_func(row, col)
        };
        self.arrange(pos_func);
    }
    fn arrange_rows(&mut self, nrows: usize, pos_func: impl Fn(usize, usize) -> DVec3) {
        let ncols = self.len().div_ceil(nrows);
        self.arrange_cols(ncols, pos_func);
    }
}

// MARK: ScaleStrokeExt
/// A trait for scaling operations with stroke width.
pub trait ScaleStrokeExt: Scale + StrokeWidth {
    /// Scale the item by a given scale at anchor with stroke width.
    fn scale_with_stroke_by_anchor(&mut self, scale: DVec3, anchor: Anchor) -> &mut Self {
        self.scale_by_anchor(scale, anchor);

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
        self.scale_with_stroke_by_anchor(scale, Anchor::CENTER)
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
