use color::{AlphaColor, Srgb, palette::css};
use glam::{DVec3, Vec3, Vec4, vec4};

use crate::{
    CoreItem, Extract,
    anchor::Aabb,
    components::{PointVec, rgba::Rgba, vpoint::VPointVec, width::Width},
    traits::{
        Alignable, Empty, FillColor, Interpolatable, Opacity, Partial, PointsFunc, RotateTransform,
        ScaleTransform, ShiftTransform, StrokeColor, StrokeWidth,
    },
};

/// Default vitem stroke width.
pub const DEFAULT_STROKE_WIDTH: f32 = 0.02;

/// Compute the normal vector from rendered VItem points.
///
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
/// use ranim_core::{VItem, glam::dvec3};
///
/// let vitem = VItem::from_vpoints(vec![
///     dvec3(0.0, 0.0, 0.0),
///     dvec3(1.0, 0.0, 0.0),
///     dvec3(0.5, 1.0, 0.0),
/// ]);
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct VItem {
    /// The normal vector of the projection target plane.
    /// If `None`, the normal will be computed from the first three points at render time.
    pub normal: Option<DVec3>,
    /// Vpoints data.
    pub vpoints: VPointVec,
    /// Stroke widths.
    pub stroke_widths: PointVec<Width>,
    /// Stroke rgbas.
    pub stroke_rgbas: PointVec<Rgba>,
    /// Fill rgbas.
    pub fill_rgbas: PointVec<Rgba>,
}

impl VItem {
    /// Construct a [`VItem`] from vpoints.
    pub fn from_vpoints(vpoints: Vec<DVec3>) -> Self {
        Self {
            normal: None,
            vpoints: VPointVec(vpoints),
            stroke_widths: vec![DEFAULT_STROKE_WIDTH.into()].into(),
            stroke_rgbas: vec![vec4(1.0, 1.0, 1.0, 1.0).into()].into(),
            fill_rgbas: vec![vec4(0.0, 0.0, 0.0, 0.0).into()].into(),
        }
    }

    /// Close the VItem.
    pub fn close(&mut self) -> &mut Self {
        if self.vpoints.last() != self.vpoints.first() && !self.vpoints.is_empty() {
            let start = self.vpoints[0];
            let end = self.vpoints[self.vpoints.len() - 1];
            self.extend_vpoints(&[(start + end) / 2.0, start]);
        }
        self
    }

    /// Shrink to center.
    pub fn shrink(&mut self) -> &mut Self {
        let bb = self.aabb();
        self.vpoints.0 = vec![bb[1]; self.vpoints.len()];
        self
    }

    /// Set the vpoints of the VItem.
    pub fn set_points(&mut self, vpoints: Vec<DVec3>) {
        self.vpoints.0 = vpoints;
    }

    /// Get anchor point.
    pub fn get_anchor(&self, idx: usize) -> Option<&DVec3> {
        self.vpoints.get(idx * 2)
    }

    /// Set the normal of the VItem's projection plane.
    pub fn with_normal(mut self, normal: DVec3) -> Self {
        self.normal = Some(normal);
        self
    }

    /// Set the normal of the VItem's projection plane.
    pub fn set_normal(&mut self, normal: DVec3) {
        self.normal = Some(normal);
    }

    /// Extend vpoints of the VItem.
    pub fn extend_vpoints(&mut self, vpoints: &[DVec3]) {
        self.vpoints.extend(vpoints.to_vec());

        // let attr_len = self.attr_len();
        // self.fill_rgbas.resize_with_last(attr_len);
        // self.stroke_rgbas.resize_with_last(attr_len);
        // self.stroke_widths.resize_with_last(attr_len);
    }

    /// Get render points as `Vec4`, with close-path flags in the w component.
    pub fn get_render_points(&self) -> Vec<Vec4> {
        self.vpoints
            .iter()
            .zip(self.vpoints.get_closepath_flags())
            .map(|(p, f)| p.as_vec3().extend(f.into()))
            .collect()
    }

    /// Put start and end on.
    pub fn put_start_and_end_on(&mut self, start: DVec3, end: DVec3) -> &mut Self {
        self.vpoints.put_start_and_end_on(start, end);
        self
    }
}

impl Default for VItem {
    fn default() -> Self {
        Self {
            normal: None,
            vpoints: VPointVec(vec![DVec3::ZERO; 3]),
            stroke_widths: vec![Width::default(); 2].into(),
            stroke_rgbas: vec![Rgba::default(); 2].into(),
            fill_rgbas: vec![Rgba::default(); 2].into(),
        }
    }
}

impl From<VItem> for CoreItem {
    fn from(value: VItem) -> Self {
        CoreItem::VItem(value)
    }
}

impl Extract for VItem {
    type Target = CoreItem;

    fn extract_into(&self, buf: &mut Vec<Self::Target>) {
        buf.push(CoreItem::VItem(self.clone()));
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
            vpoints: self.vpoints.lerp(&target.vpoints, t),
            stroke_widths: self.stroke_widths.lerp(&target.stroke_widths, t),
            stroke_rgbas: self.stroke_rgbas.lerp(&target.stroke_rgbas, t),
            fill_rgbas: self.fill_rgbas.lerp(&target.fill_rgbas, t),
        }
    }
}

impl Alignable for VItem {
    fn is_aligned(&self, other: &Self) -> bool {
        self.vpoints.is_aligned(&other.vpoints)
            && self.stroke_widths.is_aligned(&other.stroke_widths)
            && self.stroke_rgbas.is_aligned(&other.stroke_rgbas)
            && self.fill_rgbas.is_aligned(&other.fill_rgbas)
    }

    fn align_with(&mut self, other: &mut Self) {
        self.vpoints.align_with(&mut other.vpoints);
        let n = self.stroke_rgbas.len().max(other.stroke_rgbas.len());
        self.stroke_rgbas = self.stroke_rgbas.resize_by_sample(n).into();
        other.stroke_rgbas = other.stroke_rgbas.resize_by_sample(n).into();
        let n = self.stroke_widths.len().max(other.stroke_widths.len());
        self.stroke_widths = self.stroke_widths.resize_by_sample(n).into();
        other.stroke_widths = other.stroke_widths.resize_by_sample(n).into();
        let n = self.fill_rgbas.len().max(other.fill_rgbas.len());
        self.fill_rgbas = self.fill_rgbas.resize_by_sample(n).into();
        other.fill_rgbas = other.fill_rgbas.resize_by_sample(n).into();
    }
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

impl ShiftTransform for VItem {
    fn shift(&mut self, shift: DVec3) -> &mut Self {
        self.vpoints.shift(shift);
        self
    }
}

impl RotateTransform for VItem {
    fn rotate_on_axis(&mut self, axis: DVec3, angle: f64) -> &mut Self {
        self.vpoints.rotate_on_axis(axis, angle);
        if let Some(ref mut n) = self.normal {
            *n = DVec3::rotate_axis(*n, axis, angle);
        }
        self
    }
}

impl ScaleTransform for VItem {
    fn scale(&mut self, scale: DVec3) -> &mut Self {
        self.vpoints.scale(scale);
        self
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
        Self {
            normal: self.normal,
            vpoints: self.vpoints.get_partial(range.clone()),
            stroke_widths: self.stroke_widths.get_partial(range.clone()),
            stroke_rgbas: self.stroke_rgbas.get_partial(range.clone()),
            fill_rgbas: self.fill_rgbas.get_partial(range),
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
            normal: Some(DVec3::Z),
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
        self.stroke_widths
            .first()
            .map(|width| width.0)
            .unwrap_or_default()
    }

    fn apply_stroke_func(&mut self, f: impl for<'a> Fn(&'a mut [Width])) -> &mut Self {
        f(self.stroke_widths.as_mut());
        self
    }
}

#[cfg(test)]
mod tests {
    use glam::DVec3;

    use super::*;

    #[test]
    fn render_points_include_close_path_flags() {
        let vitem = VItem::from_vpoints(vec![DVec3::ZERO, DVec3::X, DVec3::ZERO]);

        assert_eq!(vitem.get_render_points()[2].w, 1.0);
    }
}
