pub mod arrow;
pub mod line;

use color::{AlphaColor, Srgb, palette::css};
use glam::{DVec3, Vec4, dvec2, dvec3, vec4};
use itertools::Itertools;

use crate::{
    components::{ComponentVec, rgba::Rgba, vpoint::VPointComponentVec, width::Width},
    prelude::{Alignable, Empty, Fill, Interpolatable, Opacity, Partial, Stroke},
    render::primitives::{
        Extract,
        vitem::{VItemPrimitive, VItemPrimitiveData},
    },
    traits::{BoundingBox, PointsFunc, Position},
};

use super::Blueprint;

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
///
///
#[derive(Debug, Clone, PartialEq)]
pub struct VItem {
    pub vpoints: VPointComponentVec,
    pub stroke_widths: ComponentVec<Width>,
    pub stroke_rgbas: ComponentVec<Rgba>,
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

impl Position for VItem {
    fn shift(&mut self, shift: DVec3) -> &mut Self {
        self.vpoints.shift(shift);
        self
    }

    fn rotate_by_anchor(
        &mut self,
        angle: f64,
        axis: DVec3,
        anchor: crate::components::Anchor,
    ) -> &mut Self {
        self.vpoints.rotate_by_anchor(angle, axis, anchor);
        self
    }

    fn scale_by_anchor(&mut self, scale: DVec3, anchor: crate::components::Anchor) -> &mut Self {
        self.vpoints.scale_by_anchor(scale, anchor);
        self
    }
}

// impl HasTransform3dComponent for VItem {
//     type Component = VPoint;
//     fn transform_3d(&self) -> &ComponentVec<Self::Component> {
//         &self.vpoints
//     }

//     fn transform_3d_mut(&mut self) -> &mut ComponentVec<Self::Component> {
//         &mut self.vpoints
//     }
// }

// impl AsRef<ComponentVec<VPoint>> for VItem {
//     fn as_ref(&self) -> &ComponentVec<VPoint> {
//         &self.vpoints
//     }
// }

// impl AsMut<ComponentVec<VPoint>> for VItem {
//     fn as_mut(&mut self) -> &mut ComponentVec<VPoint> {
//         &mut self.vpoints
//     }
// }

impl VItem {
    // TODO: remove all constructor to blueprint impl
    pub fn from_vpoints(vpoints: Vec<DVec3>) -> Self {
        let stroke_widths = vec![1.0; vpoints.len().div_ceil(2)];
        let stroke_rgbas = vec![vec4(1.0, 0.0, 0.0, 1.0); vpoints.len().div_ceil(2)];
        let fill_rgbas = vec![vec4(0.0, 1.0, 0.0, 0.5); vpoints.len().div_ceil(2)];
        Self {
            vpoints: VPointComponentVec(vpoints.into()),
            stroke_rgbas: stroke_rgbas.into(),
            stroke_widths: stroke_widths.into(),
            fill_rgbas: fill_rgbas.into(),
        }
    }

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
            .zip(self.vpoints.get_closepath_flags().iter())
            .map(|(p, f)| {
                vec4(
                    p.x as f32,
                    p.y as f32,
                    p.z as f32,
                    if *f { 1.0 } else { 0.0 },
                )
            })
            .collect()
    }
}

// MARK: Extract
impl Extract for VItem {
    type Primitive = VItemPrimitive;
    fn extract(&self) -> <Self::Primitive as crate::render::primitives::Primitive>::Data {
        VItemPrimitiveData {
            points2d: self.get_render_points(),
            fill_rgbas: self.fill_rgbas.iter().cloned().collect(),
            stroke_rgbas: self.stroke_rgbas.iter().cloned().collect(),
            stroke_widths: self.stroke_widths.iter().cloned().collect(),
        }
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
        self.stroke_rgbas.align_with(&mut other.stroke_rgbas);
        self.stroke_widths.align_with(&mut other.stroke_widths);
        self.fill_rgbas.align_with(&mut other.fill_rgbas);
    }
}

impl Interpolatable for VItem {
    fn lerp(&self, target: &Self, t: f64) -> Self {
        let vpoints = self.vpoints.lerp(&target.vpoints, t);
        let stroke_rgbas = self.stroke_rgbas.lerp(&target.stroke_rgbas, t);
        let stroke_widths = self.stroke_widths.lerp(&target.stroke_widths, t);
        let fill_rgbas = self.fill_rgbas.lerp(&target.fill_rgbas, t);
        Self {
            vpoints,
            stroke_widths,
            stroke_rgbas,
            fill_rgbas,
        }
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
            vpoints,
            stroke_widths,
            stroke_rgbas,
            fill_rgbas,
        }
    }
}

impl Empty for VItem {
    fn empty() -> Self {
        Self {
            vpoints: VPointComponentVec(vec![DVec3::ZERO; 3].into()),
            stroke_widths: vec![0.0, 0.0].into(),
            stroke_rgbas: vec![Vec4::ZERO; 2].into(),
            fill_rgbas: vec![Vec4::ZERO; 2].into(),
        }
    }
}

impl Fill for VItem {
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

impl Stroke for VItem {
    fn set_stroke_color(&mut self, color: AlphaColor<Srgb>) -> &mut Self {
        self.stroke_rgbas.set_all(color);
        self
    }
    fn set_stroke_opacity(&mut self, opacity: f32) -> &mut Self {
        self.stroke_rgbas.set_opacity(opacity);
        self
    }
    fn set_stroke_width(&mut self, width: f32) -> &mut Self {
        self.stroke_widths.set_all(width);
        self
    }
}

// MARK: Blueprints

/// A polygon defined by a list of corner points (Counter Clock wise).
pub struct Polygon(pub Vec<DVec3>);

impl Blueprint<VItem> for Polygon {
    fn build(mut self) -> VItem {
        assert!(self.0.len() > 2);

        // Close the polygon
        self.0.push(self.0[0]);

        let anchors = self.0;
        let handles = anchors
            .iter()
            .tuple_windows()
            .map(|(&a, &b)| 0.5 * (a + b))
            .collect::<Vec<_>>();

        // Interleave anchors and handles
        let points = anchors.into_iter().interleave(handles).collect::<Vec<_>>();
        // trace!("points: {:?}", points);
        VItem::from_vpoints(points)
    }
}

pub struct Rectangle(pub f64, pub f64);

impl Blueprint<VItem> for Rectangle {
    fn build(self) -> VItem {
        let half_width = self.0 / 2.0;
        let half_height = self.1 / 2.0;
        Polygon(vec![
            dvec3(-half_width, -half_height, 0.0),
            dvec3(half_width, -half_height, 0.0),
            dvec3(half_width, half_height, 0.0),
            dvec3(-half_width, half_height, 0.0),
        ])
        .build()
    }
}

pub struct Line(pub DVec3, pub DVec3);

impl Blueprint<VItem> for Line {
    fn build(self) -> VItem {
        VItem::from_vpoints(vec![self.0, (self.0 + self.1) / 2.0, self.1])
    }
}

pub struct Square(pub f64);

impl Blueprint<VItem> for Square {
    fn build(self) -> VItem {
        Rectangle(self.0, self.0).build()
    }
}

pub struct Arc {
    pub angle: f64,
    pub radius: f64,
}

impl Blueprint<VItem> for Arc {
    fn build(self) -> VItem {
        const NUM_SEGMENTS: usize = 8;
        let len = 2 * NUM_SEGMENTS + 1;

        let mut points = (0..len)
            .map(|i| {
                let angle = self.angle * i as f64 / (len - 1) as f64;
                let (mut x, mut y) = (angle.cos(), angle.sin());
                if x.abs() < 1.8e-7 {
                    x = 0.0;
                }
                if y.abs() < 1.8e-7 {
                    y = 0.0;
                }
                dvec2(x, y).extend(0.0) * self.radius
            })
            .collect::<Vec<_>>();

        let theta = self.angle / NUM_SEGMENTS as f64;
        points.iter_mut().skip(1).step_by(2).for_each(|p| {
            *p /= (theta / 2.0).cos();
        });
        // trace!("start: {:?}, end: {:?}", points[0], points[len - 1]);
        VItem::from_vpoints(points)
    }
}

pub struct ArcBetweenPoints {
    pub start: DVec3,
    pub end: DVec3,
    pub angle: f64,
}

impl Blueprint<VItem> for ArcBetweenPoints {
    fn build(self) -> VItem {
        let radius = (self.start.distance(self.end) / 2.0) / self.angle.sin();
        let arc = Arc {
            angle: self.angle,
            radius,
        };
        let mut item = arc.build();
        item.vpoints.put_start_and_end_on(self.start, self.end);
        item
    }
}

/// A circle
pub struct Circle(pub f64);

impl Blueprint<VItem> for Circle {
    fn build(self) -> VItem {
        Arc {
            angle: std::f64::consts::TAU,
            radius: self.0,
        }
        .build()
    }
}

pub enum Dot {
    Small,
    Normal,
}

impl Blueprint<VItem> for Dot {
    fn build(self) -> VItem {
        Circle(match self {
            Dot::Small => 0.04,
            Dot::Normal => 0.08,
        })
        .build()
    }
}

// width, height
pub struct Ellipse(pub f64, pub f64);

impl Blueprint<VItem> for Ellipse {
    fn build(self) -> VItem {
        let mut mobject = Circle(1.0).build();
        mobject.vpoints.scale(dvec3(self.0, self.1, 1.0));
        mobject
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_arc() {
        let arc = Arc {
            angle: std::f64::consts::PI / 2.0,
            radius: 1.0,
        }
        .build();
        println!("{:?}", arc.vpoints);
    }
}
