use color::{AlphaColor, Srgb};
use glam::{Vec3, Vec4};

use crate::{
    Extract,
    components::{PointVec, VecResizeTrait, rgba::Rgba, width::Width},
    core_item::CoreItem,
    traits::{FillColor, Interpolatable},
};

/// Default vitem stroke width
pub const DEFAULT_STROKE_WIDTH: f32 = 0.02;

/// Compute the normal vector from the first three points of a VItem.
/// Falls back to Z axis if the first three points are collinear.
pub fn vitem_normal_from_points(points: &[Vec4]) -> Vec3 {
    if points.len() < 3 {
        return Vec3::Z;
    }
    let p0 = Vec3::new(points[0].x, points[0].y, points[0].z);
    let p1 = Vec3::new(points[1].x, points[1].y, points[1].z);
    let p2 = Vec3::new(points[2].x, points[2].y, points[2].z);
    let n = (p1 - p0).cross(p2 - p0);
    if n.length_squared() < 1e-6 {
        Vec3::Z
    } else {
        n.normalize()
    }
}

#[derive(Debug, Clone, PartialEq)]
/// A primitive for rendering a vitem.
pub struct VItem {
    /// The normal vector of the projection target plane.
    /// If `None`, the normal will be computed from the first three points at render time.
    pub normal: Option<Vec3>,
    /// The points of the item in world space.
    /// (x, y, z, is_closed)
    pub points: Vec<Vec4>,
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
            normal: None,
            points: vec![Vec4::ZERO; 3],
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
            normal: match (self.normal, target.normal) {
                (Some(a), Some(b)) => Some(a.lerp(b, t)),
                (Some(a), None) => Some(a),
                (None, Some(b)) => Some(b),
                (None, None) => None,
            },
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
        self.points.clone()
    }
}
