use color::{AlphaColor, Srgb};
use glam::DVec3;

use crate::{
    Extract,
    components::{PointVec, VecResizeTrait, rgba::Rgba, vpoint::VPointVec, width::Width},
    core_item::CoreItem,
    traits::{FillColor, Interpolatable},
};

/// Default vitem stroke width
pub const DEFAULT_STROKE_WIDTH: f32 = 0.02;

/// The projection of a [`VItem`].
#[derive(Debug, Clone, Copy, PartialEq, ranim_macros::Interpolatable)]
pub struct Basis2d {
    /// The basis vector in the u direction.
    u: DVec3,
    /// The basis vector in the v direction.
    v: DVec3,
}

impl Default for Basis2d {
    fn default() -> Self {
        Self::XY
    }
}

impl Basis2d {
    /// XY
    pub const XY: Self = Self {
        u: DVec3::X,
        v: DVec3::Y,
    };
    /// XZ
    pub const XZ: Self = Self {
        u: DVec3::X,
        v: DVec3::Z,
    };
    /// YZ
    pub const YZ: Self = Self {
        u: DVec3::Y,
        v: DVec3::Z,
    };
    /// Create a new 2d basis from two 3d vectors.
    pub fn new(u: DVec3, v: DVec3) -> Self {
        Self {
            u: u.normalize(),
            v: v.normalize(),
        }
    }

    /// The basis vector in the u direction.
    pub fn u(&self) -> DVec3 {
        self.u.normalize()
    }
    /// The basis vector in the v direction.
    pub fn v(&self) -> DVec3 {
        self.v.normalize()
    }
    /// The basis vectors
    pub fn uv(&self) -> (DVec3, DVec3) {
        (self.u(), self.v())
    }
    /// The corrected basis vector in the u direction.
    /// This is same as [`Self::u`].
    pub fn corrected_u(&self) -> DVec3 {
        self.u.normalize()
    }
    /// The corrected basis vector in the v direction.
    /// This is recalculated to ensure orthogonality.
    pub fn corrected_v(&self) -> DVec3 {
        let normal = self.u.cross(self.v);
        normal.cross(self.u).normalize()
    }
    /// The corrected basis vectos.
    /// This is recalculated to ensure orthogonality.
    pub fn corrected_uv(&self) -> (DVec3, DVec3) {
        (self.corrected_u(), self.corrected_v())
    }
    /// Get the normal vector of the projection target plane.
    #[inline]
    pub fn normal(&self) -> DVec3 {
        self.u.cross(self.v).normalize()
    }
    /// Rotate the basis vectors around the given axis.
    pub fn rotate_on_axis(&mut self, axis: DVec3, angle: f64) {
        self.u = DVec3::rotate_axis(self.u, axis, angle).normalize();
        self.v = DVec3::rotate_axis(self.v, axis, angle).normalize();
    }
}

#[derive(Debug, Clone, PartialEq)]
/// A primitive for rendering a vitem.
pub struct VItem {
    /// The base point of the item, a.k.a. the origin of the item's local coordinate system.
    pub origin: DVec3,
    /// The basis vectors of the item's local coordinate system. Normalized.
    pub basis: Basis2d,
    /// The vpoints of the item in the item's local coordinate system.
    pub points: VPointVec,
    /// Fill rgbas, see [`Rgba`].
    pub fill_rgbas: PointVec<Rgba>,
    /// Stroke rgbs, see [`Rgba`].
    pub stroke_rgbas: PointVec<Rgba>,
    /// Stroke widths, see [`Width`].
    pub stroke_widths: PointVec<Width>,
}

impl Default for VItem {
    fn default() -> Self {
        Self {
            origin: DVec3::ZERO,
            basis: Basis2d::default(),
            points: VPointVec(vec![DVec3::ZERO; 3]),
            stroke_widths: vec![Width::default(); 2].into(),
            stroke_rgbas: vec![Rgba::default(); 2].into(),
            fill_rgbas: vec![Rgba::default(); 2].into(),
        }
    }
}

impl Extract for VItem {
    type Target = CoreItem;
    fn extract_into(&self, buf: &mut Vec<Self::Target>) {
        buf.push(CoreItem::VItem(self.clone()));
    }
}

impl FillColor for VItem {
    fn fill_color(&self) -> AlphaColor<Srgb> {
        let Rgba(rgba) = self.fill_rgbas[0];
        AlphaColor::new([rgba.x, rgba.y, rgba.z, rgba.w])
    }
    fn set_fill_color(&mut self, color: AlphaColor<Srgb>) -> &mut Self {
        self.fill_rgbas.fill(color.into());
        self
    }
    fn set_fill_opacity(&mut self, opacity: f32) -> &mut Self {
        self.fill_rgbas
            .iter_mut()
            .for_each(|rgba| rgba.0.w = opacity);
        self
    }
}

impl Interpolatable for VItem {
    fn lerp(&self, target: &Self, t: f64) -> Self {
        Self {
            origin: self.origin.lerp(target.origin, t),
            basis: self.basis.lerp(&target.basis, t),
            points: self.points.lerp(&target.points, t),
            fill_rgbas: self.fill_rgbas.lerp(&target.fill_rgbas, t),
            stroke_rgbas: self.stroke_rgbas.lerp(&target.stroke_rgbas, t),
            stroke_widths: self.stroke_widths.lerp(&target.stroke_widths, t),
        }
    }
    fn is_aligned(&self, other: &Self) -> bool {
        self.points.is_aligned(&other.points)
            && self.fill_rgbas.is_aligned(&other.fill_rgbas)
            && self.stroke_rgbas.is_aligned(&other.stroke_rgbas)
            && self.stroke_widths.is_aligned(&other.stroke_widths)
    }
    fn align_with(&mut self, other: &mut Self) {
        self.points.align_with(&mut other.points);
        let len = self.points.len().div_ceil(2);
        self.fill_rgbas.resize_preserving_order(len);
        other.fill_rgbas.resize_preserving_order(len);
        self.stroke_rgbas.resize_preserving_order(len);
        other.stroke_rgbas.resize_preserving_order(len);
        self.stroke_widths.resize_preserving_order(len);
        other.stroke_widths.resize_preserving_order(len);
    }
}

impl VItem {
    /// Get render points as Vec<Vec4> with is_closed flag in w component.
    /// This is used at the render boundary to produce GPU-ready data.
    pub fn get_render_points(&self) -> Vec<glam::Vec4> {
        self.points
            .iter()
            .zip(self.points.get_closepath_flags())
            .map(|(p, f)| p.as_vec3().extend(f.into()))
            .collect()
    }
}
