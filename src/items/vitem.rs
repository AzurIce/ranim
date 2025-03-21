use color::{AlphaColor, Srgb, palette::css};
use glam::{Vec3, Vec4, vec2, vec3, vec4};
use itertools::Itertools;

use crate::{
    components::{rgba::Rgba, vpoint::{VPoint, VPointSliceMethods}, width::Width, ComponentVec, Transformable},
    context::WgpuContext,
    prelude::{Alignable, Empty, Fill, Interpolatable, Opacity, Partial, Stroke},
    render::primitives::{vitem::VItemPrimitive, ExtractFrom, RenderInstance, RenderInstances},
};

use super::{Blueprint, Entity};

/// A vectorized item.
///
/// It is built from four components:
/// - [`VItem::vpoints`]: the vpoints of the item, see [`VPoint`].
/// - [`VItem::stroke_widths`]: the stroke widths of the item, see [`Width`].
/// - [`VItem::stroke_rgbas`]: the stroke colors of the item, see [`Rgba`].
/// - [`VItem::fill_rgbas`]: the fill colors of the item, see [`Rgba`].
///
/// You can construct a [`VItem`] from a list of [`VPoint`]s:
///
/// ```rust
/// let vitem = VItem::from_vpoints(vec![
///     vec3(0.0, 0.0, 0.0),
///     vec3(1.0, 0.0, 0.0),
///     vec3(0.5, 1.0, 0.0),
/// ]);
/// ```
///
///
#[derive(Debug, Clone, PartialEq)]
pub struct VItem {
    pub vpoints: ComponentVec<VPoint>,
    pub stroke_widths: ComponentVec<Width>,
    pub stroke_rgbas: ComponentVec<Rgba>,
    pub fill_rgbas: ComponentVec<Rgba>,
}

impl AsRef<ComponentVec<VPoint>> for VItem {
    fn as_ref(&self) -> &ComponentVec<VPoint> {
        &self.vpoints
    }
}

impl AsMut<ComponentVec<VPoint>> for VItem {
    fn as_mut(&mut self) -> &mut ComponentVec<VPoint> {
        &mut self.vpoints
    }
}

impl VItem {
    // TODO: remove all constructor to blueprint impl
    pub fn from_vpoints(vpoints: Vec<Vec3>) -> Self {
        let stroke_widths = vec![1.0; (vpoints.len() + 1) / 2];
        let stroke_rgbas = vec![vec4(1.0, 0.0, 0.0, 1.0); (vpoints.len() + 1) / 2];
        let fill_rgbas = vec![vec4(0.0, 1.0, 0.0, 0.5); (vpoints.len() + 1) / 2];
        Self {
            vpoints: vpoints.into(),
            stroke_rgbas: stroke_rgbas.into(),
            stroke_widths: stroke_widths.into(),
            fill_rgbas: fill_rgbas.into(),
        }
    }

    pub fn extend_vpoints(&mut self, vpoints: &[Vec3]) {
        self.vpoints
            .extend_from_vec(vpoints.iter().cloned().map(Into::into).collect());

        let len = self.vpoints.len();
        self.fill_rgbas.resize_with_last((len + 1) / 2);
        self.stroke_rgbas.resize_with_last((len + 1) / 2);
        self.stroke_widths.resize_with_last((len + 1) / 2);
    }

    pub(crate) fn get_render_points(&self) -> Vec<Vec4> {
        self.vpoints
            .iter()
            .zip(self.vpoints.get_closepath_flags().iter())
            .map(|(p, f)| vec4(p.x, p.y, p.z, if *f { 1.0 } else { 0.0 }))
            .collect()
    }
}

// MARK: Entity impl
impl Entity for VItem {
    fn get_render_instance_for_entity<'a>(
        &self,
        render_instances: &'a RenderInstances,
        entity_id: usize,
    ) -> Option<&'a dyn RenderInstance> {
        render_instances
            .get_dynamic::<VItemPrimitive>(entity_id)
            .map(|x| x as &dyn RenderInstance)
    }
    fn prepare_render_instance_for_entity(
        &self,
        ctx: &WgpuContext,
        render_instances: &mut RenderInstances,
        entity_id: usize,
    ) {
        let render_instance = render_instances.get_dynamic_or_init::<VItemPrimitive>(entity_id);
        render_instance.update_from(ctx, self);
    }
}

// MARK: Extract impl
impl ExtractFrom<VItem> for VItemPrimitive {
    fn update_from(&mut self, ctx: &WgpuContext, data: &VItem) {
        self.update(
            ctx,
            &data.get_render_points(),
            &data.fill_rgbas,
            &data.stroke_rgbas,
            &data.stroke_widths,
        );
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
    fn lerp(&self, target: &Self, t: f32) -> Self {
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
    fn get_partial(&self, range: std::ops::Range<f32>) -> Self {
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
            vpoints: vec![Vec3::ZERO; 3].into(),
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
pub struct Polygon(pub Vec<Vec3>);

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

pub struct Rectangle(pub f32, pub f32);

impl Blueprint<VItem> for Rectangle {
    fn build(self) -> VItem {
        let half_width = self.0 / 2.0;
        let half_height = self.1 / 2.0;
        Polygon(vec![
            vec3(-half_width, -half_height, 0.0),
            vec3(half_width, -half_height, 0.0),
            vec3(half_width, half_height, 0.0),
            vec3(-half_width, half_height, 0.0),
        ])
        .build()
    }
}

pub struct Line(pub Vec3, pub Vec3);

impl Blueprint<VItem> for Line {
    fn build(self) -> VItem {
        VItem::from_vpoints(vec![self.0, (self.0 + self.1) / 2.0, self.1])
    }
}

pub struct Square(pub f32);

impl Blueprint<VItem> for Square {
    fn build(self) -> VItem {
        Rectangle(self.0, self.0).build()
    }
}

pub struct Arc {
    pub angle: f32,
    pub radius: f32,
}

impl Blueprint<VItem> for Arc {
    fn build(self) -> VItem {
        const NUM_SEGMENTS: usize = 8;
        let len = 2 * NUM_SEGMENTS + 1;

        let mut points = (0..len)
            .map(|i| {
                let angle = self.angle * i as f32 / (len - 1) as f32;
                let (mut x, mut y) = (angle.cos(), angle.sin());
                if x.abs() < 1.8e-7 {
                    x = 0.0;
                }
                if y.abs() < 1.8e-7 {
                    y = 0.0;
                }
                vec2(x, y).extend(0.0) * self.radius
            })
            .collect::<Vec<_>>();

        let theta = self.angle / NUM_SEGMENTS as f32;
        points.iter_mut().skip(1).step_by(2).for_each(|p| {
            *p /= (theta / 2.0).cos();
        });
        // trace!("start: {:?}, end: {:?}", points[0], points[len - 1]);
        VItem::from_vpoints(points)
    }
}

pub struct ArcBetweenPoints {
    pub start: Vec3,
    pub end: Vec3,
    pub angle: f32,
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
pub struct Circle(pub f32);

impl Blueprint<VItem> for Circle {
    fn build(self) -> VItem {
        Arc {
            angle: std::f32::consts::TAU,
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
pub struct Ellipse(pub f32, pub f32);

impl Blueprint<VItem> for Ellipse {
    fn build(self) -> VItem {
        let mut mobject = Circle(1.0).build();
        mobject.vpoints.scale(vec3(self.0, self.1, 1.0));
        mobject
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_arc() {
        let arc = Arc {
            angle: std::f32::consts::PI / 2.0,
            radius: 1.0,
        }
        .build();
        println!("{:?}", arc.vpoints);
    }
}
