//! Quadratic Bezier Concatenated Item
//!
//! VItem itself is composed with 3d bezier path segments, but when *ranim* renders VItem,
//! it assumes that all points are in the same plane to calculate depth information.
//! Which means that ranim actually renders a **projection** of the VItem onto a plane.
//!
//! The projection target plane has the initial basis and normal defined as `(DVec3::X, DVec3::Y)` and `DVec3::Z` respectively, and it contains the first point of the VItem.
//!
//! So the normal way to use a [`VItem`] is to make sure that all points are in the same plane, at this time the **projection** is equivalent to the VItem itself. Or you may break this, and let ranim renders the **projection** of it.
// pub mod arrow;
/// Geometry items
pub mod geometry;
/// Svg item
pub mod svg;
/// Simple text items
pub mod text;
/// Typst items
pub mod typst;

use std::any::Any;

use color::{AlphaColor, Srgb, palette::css};
use glam::{DVec3, Vec4, vec4};
use ranim_core::anchor::Aabb;
use ranim_core::core_item::CoreItem;
use ranim_core::traits::{RotateTransform, ScaleTransform, ShiftTransform};
use ranim_core::{color, glam};

use ranim_core::{
    components::{PointVec, VecResizeTrait, rgba::Rgba, vpoint::VPointVec, width::Width},
    prelude::{Empty, FillColor, Opacity, Partial, StrokeWidth},
    traits::{Interpolatable, PointsFunc, StrokeColor},
};

/// Default stroke width
pub use ranim_core::core_item::vitem::DEFAULT_STROKE_WIDTH;

/// A trait for types that can build a [`VPointVec`] path.
pub trait VPath: Any + Into<VPointVec> + Clone {
    /// The normalized normal vector
    fn normal(&self) -> DVec3;
}

impl VPath for VPointVec {
    fn normal(&self) -> DVec3 {
        self.normal()
    }
}

/// A vectorized item.
///
/// The *shape* of a vectorized item is defined with a type implemented [`VPath`].
///
/// The *style* of a vectorized item is defined with the following fields:
/// - [`VItem::stroke_widths`]: the stroke widths of the item, see [`Width`].
/// - [`VItem::stroke_rgbas`]: the stroke colors of the item, see [`Rgba`].
/// - [`VItem::fill_rgbas`]: the fill colors of the item, see [`Rgba`].
#[derive(Clone, Debug)]
pub struct VItem<T = VPointVec> {
    /// The inner vpath
    pub inner: T,
    /// stroke widths
    pub stroke_widths: PointVec<Width>,
    /// stroke rgbas
    pub stroke_rgbas: PointVec<Rgba>,
    /// fill rgbas
    pub fill_rgbas: PointVec<Rgba>,
}

impl<T: VPath + Clone + 'static> VItem<T> {
    /// Create a new NeoVItem from a VPath.
    pub fn new_with(vpath: T) -> Self {
        Self {
            inner: vpath,
            stroke_widths: PointVec::default(),
            stroke_rgbas: PointVec::default(),
            fill_rgbas: PointVec::default(),
        }
    }
    /// Convert the inner VPath type to another type.
    pub fn convert<U: VPath>(self) -> VItem<U>
    where
        T: Into<U>,
    {
        VItem {
            inner: self.inner.into(),
            stroke_widths: self.stroke_widths,
            stroke_rgbas: self.stroke_rgbas,
            fill_rgbas: self.fill_rgbas,
        }
    }
    /// Operate on &inner
    pub fn with_inner<O, F: FnOnce(&T) -> O>(&self, f: F) -> O {
        f(&self.inner)
    }
    /// Operate on &mut inner
    pub fn with_inner_mut<O, F: FnOnce(&mut T) -> O>(&mut self, f: F) -> O {
        f(&mut self.inner)
    }
}

impl<T: VPath> Aabb for VItem<T> {
    fn aabb(&self) -> [DVec3; 2] {
        self.inner.clone().into().aabb()
    }
}

impl Partial for VItem<VPointVec> {
    fn get_partial(&self, range: std::ops::Range<f64>) -> Self {
        let inner = self.inner.get_partial(range.clone());
        let stroke_rgbas = self.stroke_rgbas.get_partial(range.clone());
        let stroke_widths = self.stroke_widths.get_partial(range.clone());
        let fill_rgbas = self.fill_rgbas.get_partial(range.clone());
        Self {
            inner,
            stroke_widths,
            stroke_rgbas,
            fill_rgbas,
        }
    }
    fn get_partial_closed(&self, range: std::ops::Range<f64>) -> Self {
        let mut partial = self.get_partial(range);
        partial.inner.close();
        partial
    }
}

impl PointsFunc for VItem<VPointVec> {
    fn apply_points_func(&mut self, f: impl Fn(&mut [DVec3])) -> &mut Self {
        self.inner.apply_points_func(f);
        self
    }
}

impl<T: VPath> From<VItem<T>> for ranim_core::core_item::vitem::VItem {
    fn from(value: VItem<T>) -> Self {
        Self {
            normal: value.inner.normal(),
            points: value.inner.into(),
            fill_rgbas: value.fill_rgbas,
            stroke_rgbas: value.stroke_rgbas,
            stroke_widths: value.stroke_widths,
        }
    }
}

impl<T: VPath> From<VItem<T>> for CoreItem {
    fn from(value: VItem<T>) -> Self {
        CoreItem::VItem(value.into())
    }
}

// MARK: Blanket impls
impl<T> Opacity for VItem<T> {
    fn set_opacity(&mut self, opacity: f32) -> &mut Self {
        self.stroke_rgbas.set_opacity(opacity);
        self.fill_rgbas.set_opacity(opacity);
        self
    }
}

impl<T> FillColor for VItem<T> {
    fn fill_color(&self) -> AlphaColor<Srgb> {
        self.fill_rgbas
            .first()
            .map(|&rgba| rgba.into())
            .unwrap_or(css::WHITE)
    }
    fn set_fill_color(&mut self, color: AlphaColor<Srgb>) -> &mut Self {
        self.fill_rgbas
            .iter_mut()
            .for_each(|rgba| *rgba = color.into());
        self
    }
    fn set_fill_opacity(&mut self, opacity: f32) -> &mut Self {
        self.fill_rgbas.set_opacity(opacity);
        self
    }
}

impl<T> StrokeColor for VItem<T> {
    fn stroke_color(&self) -> AlphaColor<Srgb> {
        self.stroke_rgbas
            .first()
            .map(|&rgba| rgba.into())
            .unwrap_or(css::WHITE)
    }
    fn set_stroke_color(&mut self, color: AlphaColor<Srgb>) -> &mut Self {
        self.stroke_rgbas
            .iter_mut()
            .for_each(|rgba| *rgba = color.into());
        self
    }
    fn set_stroke_opacity(&mut self, opacity: f32) -> &mut Self {
        self.stroke_rgbas.set_opacity(opacity);
        self
    }
}

impl<T> StrokeWidth for VItem<T> {
    fn stroke_width(&self) -> f32 {
        self.stroke_widths[0].0
    }
    fn apply_stroke_func(&mut self, f: impl for<'a> Fn(&'a mut [Width])) -> &mut Self {
        f(self.stroke_widths.as_mut());
        self
    }
}

impl<T: ShiftTransform> ShiftTransform for VItem<T> {
    fn shift(&mut self, shift: DVec3) -> &mut Self {
        self.inner.shift(shift);
        self
    }
}

impl<T: RotateTransform> RotateTransform for VItem<T> {
    fn rotate_on_axis(&mut self, axis: DVec3, angle: f64) -> &mut Self {
        self.inner.rotate_on_axis(axis, angle);
        self
    }
}

impl<T: ScaleTransform> ScaleTransform for VItem<T> {
    fn scale(&mut self, scale: DVec3) -> &mut Self {
        self.inner.scale(scale);
        self
    }
}

impl<T: Interpolatable> Interpolatable for VItem<T> {
    fn lerp(&self, other: &Self, t: f64) -> Self {
        Self {
            inner: Interpolatable::lerp(&self.inner, &other.inner, t),
            stroke_widths: Interpolatable::lerp(&self.stroke_widths, &other.stroke_widths, t),
            stroke_rgbas: Interpolatable::lerp(&self.stroke_rgbas, &other.stroke_rgbas, t),
            fill_rgbas: Interpolatable::lerp(&self.fill_rgbas, &other.fill_rgbas, t),
        }
    }
    fn is_aligned(&self, other: &Self) -> bool {
        self.inner.is_aligned(&other.inner)
            && self.stroke_widths.is_aligned(&other.stroke_widths)
            && self.stroke_rgbas.is_aligned(&other.stroke_rgbas)
            && self.fill_rgbas.is_aligned(&other.fill_rgbas)
    }
    fn align_with(&mut self, other: &mut Self) {
        self.inner.align_with(&mut other.inner);
        self.stroke_rgbas.align_with(&mut other.stroke_rgbas);
        self.stroke_widths.align_with(&mut other.stroke_widths);
        self.fill_rgbas.align_with(&mut other.fill_rgbas);
    }
    fn vec_align_with(a: &mut Vec<Self>, b: &mut Vec<Self>) {
        use ranim_core::utils::resize_preserving_order_with_repeated_indices;
        let len = a.len().max(b.len());
        let transparent_repeated = |items: &mut Vec<Self>, repeat_idxs: Vec<usize>| {
            for idx in repeat_idxs {
                items[idx].set_opacity(0.0);
            }
        };
        if a.len() != len {
            let (mut items, idxs) = resize_preserving_order_with_repeated_indices(a, len);
            transparent_repeated(&mut items, idxs);
            *a = items;
        }
        if b.len() != len {
            let (mut items, idxs) = resize_preserving_order_with_repeated_indices(b, len);
            transparent_repeated(&mut items, idxs);
            *b = items;
        }
        a.iter_mut().zip(b).for_each(|(x, y)| x.align_with(y));
    }
}

// MARK: VItem<VPointVec> specific impls

impl VItem<VPointVec> {
    /// Construct a [`VItem`] from vpoints
    pub fn from_vpoints(vpoints: Vec<DVec3>) -> Self {
        let stroke_widths = vec![DEFAULT_STROKE_WIDTH.into(); vpoints.len().div_ceil(2)];
        let stroke_rgbas = vec![vec4(1.0, 1.0, 1.0, 1.0).into(); vpoints.len().div_ceil(2)];
        let fill_rgbas = vec![vec4(0.0, 0.0, 0.0, 0.0).into(); vpoints.len().div_ceil(2)];
        Self {
            inner: VPointVec(vpoints),
            stroke_rgbas: stroke_rgbas.into(),
            stroke_widths: stroke_widths.into(),
            fill_rgbas: fill_rgbas.into(),
        }
    }
    /// Close the VItem
    pub fn close(&mut self) -> &mut Self {
        if self.inner.last() != self.inner.first() && !self.inner.is_empty() {
            let start = self.inner[0];
            let end = self.inner[self.inner.len() - 1];
            self.extend_vpoints(&[(start + end) / 2.0, start]);
        }
        self
    }
    /// Shrink to center
    pub fn shrink(&mut self) -> &mut Self {
        let bb = self.aabb();
        self.inner.0 = vec![bb[1]; self.inner.len()];
        self
    }
    /// Set the vpoints of the VItem
    pub fn set_points(&mut self, vpoints: Vec<DVec3>) {
        self.inner.0 = vpoints;
    }
    /// Get anchor points
    pub fn get_anchor(&self, idx: usize) -> Option<&DVec3> {
        self.inner.get(idx * 2)
    }
    /// Extend vpoints of the VItem
    pub fn extend_vpoints(&mut self, vpoints: &[DVec3]) {
        self.inner.extend(vpoints.to_vec());

        let len = self.inner.len();
        self.fill_rgbas.resize_with_last(len.div_ceil(2));
        self.stroke_rgbas.resize_with_last(len.div_ceil(2));
        self.stroke_widths.resize_with_last(len.div_ceil(2));
    }
    /// Put start and end on
    pub fn put_start_and_end_on(&mut self, start: DVec3, end: DVec3) -> &mut Self {
        self.inner.put_start_and_end_on(start, end);
        self
    }
}

impl Empty for VItem<VPointVec> {
    fn empty() -> Self {
        Self {
            inner: VPointVec(vec![DVec3::ZERO; 3]),
            stroke_widths: vec![0.0.into(); 2].into(),
            stroke_rgbas: vec![Vec4::ZERO.into(); 2].into(),
            fill_rgbas: vec![Vec4::ZERO.into(); 2].into(),
        }
    }
}
