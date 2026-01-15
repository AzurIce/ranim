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
use ranim_core::core_item::CoreItem;
use ranim_core::core_item::vitem_2d::VItem2d;
use ranim_core::traits::Anchor;
use ranim_core::{Extract, color, glam};

use ranim_core::{
    components::{ComponentVec, rgba::Rgba, vpoint::VPointComponentVec, width::Width},
    prelude::{Alignable, Empty, FillColor, Opacity, Partial, StrokeWidth},
    traits::{BoundingBox, PointsFunc, Rotate, Scale, Shift, StrokeColor},
};

/// The projection of a [`VItem`].
#[derive(Debug, Clone, Copy, PartialEq, ranim_macros::Interpolatable)]
pub struct Proj {
    /// The basis vector in the u direction.
    basis_u: DVec3,
    /// The basis vector in the v direction.
    basis_v: DVec3,
}

impl Default for Proj {
    fn default() -> Self {
        Self {
            basis_u: DVec3::X,
            basis_v: DVec3::Y,
        }
    }
}

impl Proj {
    /// The basis vector in the u direction.
    pub fn basis_u(&self) -> DVec3 {
        self.basis_u.normalize()
    }
    /// The basis vector in the v direction.
    pub fn basis_v(&self) -> DVec3 {
        self.basis_v.normalize()
    }
    /// The basis vectors
    pub fn basis(&self) -> (DVec3, DVec3) {
        (self.basis_u(), self.basis_v())
    }
    /// The corrected basis vector in the u direction.
    /// This is same as [`Proj::basis_u`].
    pub fn corrected_basis_u(&self) -> DVec3 {
        self.basis_u.normalize()
    }
    /// The corrected basis vector in the v direction.
    /// This is recalculated to ensure orthogonality.
    pub fn corrected_basis_v(&self) -> DVec3 {
        let normal = self.basis_u.cross(self.basis_v);
        normal.cross(self.basis_u).normalize()
    }
    /// Rotate the projection.
    pub fn rotate(&mut self, angle: f64, axis: DVec3) {
        self.basis_u = self.basis_u.rotate_axis(axis, angle).normalize();
        self.basis_v = self.basis_v.rotate_axis(axis, angle).normalize();
    }
    /// Get the normal vector of the projection target plane.
    #[inline]
    pub fn normal(&self) -> DVec3 {
        self.basis_u.cross(self.basis_v).normalize()
    }
}

/// A vectorized item.
///
/// It is built from four components:
/// - [`VItem::vpoints`]: the vpoints of the item, see [`VPointComponentVec`].
/// - [`VItem::stroke_widths`]: the stroke widths of the item, see [`Width`].
/// - [`VItem::stroke_rgbas`]: the stroke colors of the item, see [`Rgba`].
/// - [`VItem::fill_rgbas`]: the fill colors of the item, see [`Rgba`].
///
/// You can construct a [`VItem`] from a list of VPoints, see [`VPointComponentVec`]:
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
    /// The projection info.
    ///
    /// See [`Proj`]
    pub proj: Proj,
    /// vpoints data
    pub vpoints: VPointComponentVec,
    /// stroke widths
    pub stroke_widths: ComponentVec<Width>,
    /// stroke rgbas
    pub stroke_rgbas: ComponentVec<Rgba>,
    /// fill rgbas
    pub fill_rgbas: ComponentVec<Rgba>,
}

impl PointsFunc for VItem {
    fn apply_points_func(&mut self, f: impl Fn(&mut [DVec3])) -> &mut Self {
        self.vpoints.apply_points_func(f);
        self
    }
}

impl BoundingBox for VItem {
    fn get_bounding_box(&self) -> [DVec3; 3] {
        self.vpoints.get_bounding_box()
    }
}

impl Shift for VItem {
    fn shift(&mut self, shift: DVec3) -> &mut Self {
        self.vpoints.shift(shift);
        self
    }
}

impl Rotate for VItem {
    fn rotate_by_anchor(&mut self, angle: f64, axis: DVec3, anchor: Anchor) -> &mut Self {
        self.vpoints.rotate_by_anchor(angle, axis, anchor);
        self.proj.rotate(angle, axis);
        self
    }
}

impl Scale for VItem {
    fn scale_by_anchor(&mut self, scale: DVec3, anchor: Anchor) -> &mut Self {
        self.vpoints.scale_by_anchor(scale, anchor);
        self
    }
}

/// Default stroke width
pub use ranim_core::core_item::vitem_2d::DEFAULT_STROKE_WIDTH;

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
        let bb = self.get_bounding_box();
        self.vpoints.0 = vec![bb[1]; self.vpoints.len()].into();
        self
    }
    /// Set the vpoints of the VItem
    pub fn set_points(&mut self, vpoints: Vec<DVec3>) {
        self.vpoints.0 = vpoints.into();
    }
    /// Get anchor points
    pub fn get_anchor(&self, idx: usize) -> Option<&DVec3> {
        self.vpoints.get(idx * 2)
    }
    /// Set the projection of the VItem
    pub fn with_proj(mut self, proj: Proj) -> Self {
        self.proj = proj;
        self
    }
    /// Set the projection of the VItem
    pub fn set_proj(&mut self, proj: Proj) {
        self.proj = proj;
    }
    /// Construct a [`VItem`] form vpoints
    pub fn from_vpoints(vpoints: Vec<DVec3>) -> Self {
        let stroke_widths = vec![DEFAULT_STROKE_WIDTH; vpoints.len().div_ceil(2)];
        let stroke_rgbas = vec![vec4(1.0, 1.0, 1.0, 1.0); vpoints.len().div_ceil(2)];
        let fill_rgbas = vec![vec4(0.0, 0.0, 0.0, 0.0); vpoints.len().div_ceil(2)];
        Self {
            proj: Proj::default(),
            vpoints: VPointComponentVec(vpoints.into()),
            stroke_rgbas: stroke_rgbas.into(),
            stroke_widths: stroke_widths.into(),
            fill_rgbas: fill_rgbas.into(),
        }
    }
    /// Extend vpoints of the VItem
    pub fn extend_vpoints(&mut self, vpoints: &[DVec3]) {
        self.vpoints.extend_from_vec(vpoints.to_vec());

        let len = self.vpoints.len();
        self.fill_rgbas.resize_with_last(len.div_ceil(2));
        self.stroke_rgbas.resize_with_last(len.div_ceil(2));
        self.stroke_widths.resize_with_last(len.div_ceil(2));
    }

    pub(crate) fn get_render_points(&self) -> Vec<Vec4> {
        self.vpoints
            .iter()
            .zip(self.vpoints.get_closepath_flags().into_iter())
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
        buf.push(CoreItem::VItem2D(VItem2d {
            origin: self.vpoints.first().unwrap().as_vec3(),
            basis: (
                self.proj.corrected_basis_u().as_vec3(),
                self.proj.corrected_basis_v().as_vec3(),
            ),
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
            proj: self.proj,
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
            proj: Proj::default(),
            vpoints: VPointComponentVec(vec![DVec3::ZERO; 3].into()),
            stroke_widths: vec![0.0, 0.0].into(),
            stroke_rgbas: vec![Vec4::ZERO; 2].into(),
            fill_rgbas: vec![Vec4::ZERO; 2].into(),
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
        self.fill_rgbas.set_all(color);
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
        self.stroke_rgbas.set_all(color);
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
