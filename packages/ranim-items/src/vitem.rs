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
/// Typst items
pub mod typst;
// pub mod line;

use color::{AlphaColor, Srgb, palette::css};
use glam::{DVec3, Vec4, vec4};
use ranim_core::anchor::Aabb;
use ranim_core::core_item::CoreItem;
use ranim_core::core_item::vitem::Basis2d;
use ranim_core::glam::DAffine3;
use ranim_core::traits::AffineTransform;
use ranim_core::{Extract, color, glam};

use ranim_core::{
    components::{PointVec, VecResizeTrait, rgba::Rgba, vpoint::VPointVec, width::Width},
    prelude::{Alignable, Empty, FillColor, Opacity, Partial, StrokeWidth},
    traits::{PointsFunc, Rotate, Scale, Shift, StrokeColor},
};

/// A vectorized item.
///
/// It is built from four components:
/// - [`VItem::vpoints`]: the vpoints of the item, see [`VPointVec`].
/// - [`VItem::stroke_widths`]: the stroke widths of the item, see [`Width`].
/// - [`VItem::stroke_rgbas`]: the stroke colors of the item, see [`Rgba`].
/// - [`VItem::fill_rgbas`]: the fill colors of the item, see [`Rgba`].
///
/// You can construct a [`VItem`] from a list of VPoints, see [`VPointVec`]:
///
/// ```rust
/// let vitem = VItem::from_vpoints(vec![
///     dvec3(0.0, 0.0, 0.0),
///     dvec3(1.0, 0.0, 0.0),
///     dvec3(0.5, 1.0, 0.0),
/// ]);
/// ```
#[derive(Debug, Clone, PartialEq, ranim_macros::Interpolatable)]
pub struct VItem {
    /// The 2d basis used for projecting the item during rendering.
    ///
    /// See [`Basis2d`]
    pub basis: Basis2d,
    /// vpoints data
    pub vpoints: VPointVec,
    /// stroke widths
    pub stroke_widths: PointVec<Width>,
    /// stroke rgbas
    pub stroke_rgbas: PointVec<Rgba>,
    /// fill rgbas
    pub fill_rgbas: PointVec<Rgba>,
}

impl PointsFunc for VItem {
    fn apply_points_func(&mut self, f: impl Fn(&mut [DVec3])) -> &mut Self {
        self.vpoints.apply_points_func(f);
        self
    }
}

impl Aabb for VItem {
    fn aabb(&self) -> [DVec3; 2] {
        self.vpoints.aabb()
    }
}

impl Shift for VItem {
    fn shift(&mut self, shift: DVec3) -> &mut Self {
        self.vpoints.shift(shift);
        self
    }
}

impl Rotate for VItem {
    fn rotate_at_point(&mut self, angle: f64, axis: DVec3, point: DVec3) -> &mut Self {
        self.vpoints.rotate_at_point(angle, axis, point);
        self.basis.rotate_axis(axis, angle);
        self
    }
}

impl Scale for VItem {
    fn scale_at_point(&mut self, scale: DVec3, point: DVec3) -> &mut Self {
        self.vpoints.scale_at_point(scale, point);
        self
    }
}

impl AffineTransform for VItem {
    fn affine_transform_at_point(&mut self, mat: DAffine3, origin: DVec3) -> &mut Self {
        self.vpoints.affine_transform_at_point(mat, origin);
        self
    }
}

/// Default stroke width
pub use ranim_core::core_item::vitem::DEFAULT_STROKE_WIDTH;

impl VItem {
    /// Close the VItem
    pub fn close(&mut self) -> &mut Self {
        if self.vpoints.last() != self.vpoints.first() && !self.vpoints.is_empty() {
            let start = self.vpoints[0];
            let end = self.vpoints[self.vpoints.len() - 1];
            self.extend_vpoints(&[(start + end) / 2.0, start]);
        }
        self
    }
    /// Shrink to center
    pub fn shrink(&mut self) -> &mut Self {
        let bb = self.aabb();
        self.vpoints.0 = vec![bb[1]; self.vpoints.len()];
        self
    }
    /// Set the vpoints of the VItem
    pub fn set_points(&mut self, vpoints: Vec<DVec3>) {
        self.vpoints.0 = vpoints;
    }
    /// Get anchor points
    pub fn get_anchor(&self, idx: usize) -> Option<&DVec3> {
        self.vpoints.get(idx * 2)
    }
    /// Set the projection of the VItem
    pub fn with_basis(mut self, basis: Basis2d) -> Self {
        self.basis = basis;
        self
    }
    /// Set the projection of the VItem
    pub fn set_proj(&mut self, basis: Basis2d) {
        self.basis = basis;
    }
    /// Construct a [`VItem`] form vpoints
    pub fn from_vpoints(vpoints: Vec<DVec3>) -> Self {
        let stroke_widths = vec![DEFAULT_STROKE_WIDTH.into(); vpoints.len().div_ceil(2)];
        let stroke_rgbas = vec![vec4(1.0, 1.0, 1.0, 1.0).into(); vpoints.len().div_ceil(2)];
        let fill_rgbas = vec![vec4(0.0, 0.0, 0.0, 0.0).into(); vpoints.len().div_ceil(2)];
        Self {
            basis: Basis2d::default(),
            vpoints: VPointVec(vpoints),
            stroke_rgbas: stroke_rgbas.into(),
            stroke_widths: stroke_widths.into(),
            fill_rgbas: fill_rgbas.into(),
        }
    }
    /// Extend vpoints of the VItem
    pub fn extend_vpoints(&mut self, vpoints: &[DVec3]) {
        self.vpoints.extend(vpoints.to_vec());

        let len = self.vpoints.len();
        self.fill_rgbas.resize_with_last(len.div_ceil(2));
        self.stroke_rgbas.resize_with_last(len.div_ceil(2));
        self.stroke_widths.resize_with_last(len.div_ceil(2));
    }

    pub(crate) fn get_render_points(&self) -> Vec<Vec4> {
        self.vpoints
            .iter()
            .zip(self.vpoints.get_closepath_flags())
            .map(|(p, f)| p.as_vec3().extend(f.into()))
            .collect()
    }
    /// Put start and end on
    pub fn put_start_and_end_on(&mut self, start: DVec3, end: DVec3) -> &mut Self {
        self.vpoints.put_start_and_end_on(start, end);
        self
    }
}

impl Extract for VItem {
    type Target = CoreItem;
    fn extract_into(&self, buf: &mut Vec<Self::Target>) {
        buf.push(CoreItem::VItem(ranim_core::core_item::vitem::VItem {
            origin: self.vpoints.first().unwrap().as_vec3(),
            basis: self.basis,
            points: self.get_render_points(),
            fill_rgbas: self.fill_rgbas.iter().cloned().collect(),
            stroke_rgbas: self.stroke_rgbas.iter().cloned().collect::<Vec<_>>(),
            stroke_widths: self.stroke_widths.iter().cloned().collect::<Vec<_>>(),
        }));
    }
}

// MARK: Anim traits impl
impl Alignable for VItem {
    fn is_aligned(&self, other: &Self) -> bool {
        self.vpoints.is_aligned(&other.vpoints)
            && self.stroke_widths.is_aligned(&other.stroke_widths)
            && self.stroke_rgbas.is_aligned(&other.stroke_rgbas)
            && self.fill_rgbas.is_aligned(&other.fill_rgbas)
    }
    fn align_with(&mut self, other: &mut Self) {
        self.vpoints.align_with(&mut other.vpoints);
        let len = self.vpoints.len().div_ceil(2);
        self.stroke_rgbas.resize_preserving_order(len);
        other.stroke_rgbas.resize_preserving_order(len);
        self.stroke_widths.resize_preserving_order(len);
        other.stroke_widths.resize_preserving_order(len);
        self.fill_rgbas.resize_preserving_order(len);
        other.fill_rgbas.resize_preserving_order(len);
    }
}

impl Opacity for VItem {
    fn set_opacity(&mut self, opacity: f32) -> &mut Self {
        self.stroke_rgbas.set_opacity(opacity);
        self.fill_rgbas.set_opacity(opacity);
        self
    }
}

impl Partial for VItem {
    fn get_partial(&self, range: std::ops::Range<f64>) -> Self {
        let vpoints = self.vpoints.get_partial(range.clone());
        let stroke_rgbas = self.stroke_rgbas.get_partial(range.clone());
        let stroke_widths = self.stroke_widths.get_partial(range.clone());
        let fill_rgbas = self.fill_rgbas.get_partial(range.clone());
        Self {
            basis: self.basis,
            vpoints,
            stroke_widths,
            stroke_rgbas,
            fill_rgbas,
        }
    }
    fn get_partial_closed(&self, range: std::ops::Range<f64>) -> Self {
        let mut partial = self.get_partial(range);
        partial.close();
        partial
    }
}

impl Empty for VItem {
    fn empty() -> Self {
        Self {
            basis: Basis2d::default(),
            vpoints: VPointVec(vec![DVec3::ZERO; 3]),
            stroke_widths: vec![0.0.into(); 2].into(),
            stroke_rgbas: vec![Vec4::ZERO.into(); 2].into(),
            fill_rgbas: vec![Vec4::ZERO.into(); 2].into(),
        }
    }
}

impl FillColor for VItem {
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

impl StrokeColor for VItem {
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

impl StrokeWidth for VItem {
    fn stroke_width(&self) -> f32 {
        self.stroke_widths[0].0
    }
    fn apply_stroke_func(&mut self, f: impl for<'a> Fn(&'a mut [Width])) -> &mut Self {
        f(self.stroke_widths.as_mut());
        self
    }
}
