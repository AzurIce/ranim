use color::{AlphaColor, Srgb};
use glam::{DVec3, Vec3, Vec4};

use crate::{
    Extract,
    components::{rgba::Rgba, width::Width},
    core_item::CoreItem,
    traits::FillColor,
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
    pub origin: Vec3,
    /// The basis vectors of the item's local coordinate system. Normalized.
    pub basis: Basis2d,
    /// The points of the item in the item's local coordinate system.
    /// (x, y, z, is_closed)
    pub points: Vec<Vec4>,
    /// Fill rgbas, see [`Rgba`].
    pub fill_rgbas: Vec<Rgba>,
    /// Stroke rgbs, see [`Rgba`].
    pub stroke_rgbas: Vec<Rgba>,
    /// Stroke widths, see [`Width`].
    pub stroke_widths: Vec<Width>,
}

impl Default for VItem {
    fn default() -> Self {
        Self {
            origin: Vec3::ZERO,
            basis: Basis2d::default(),
            points: vec![Vec4::ZERO; 3],
            stroke_widths: vec![Width::default(); 2],
            stroke_rgbas: vec![Rgba::default(); 2],
            fill_rgbas: vec![Rgba::default(); 2],
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
