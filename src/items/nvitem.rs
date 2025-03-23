use color::{AlphaColor, Srgb, palette::css};
use glam::{Vec3, Vec4, vec2, vec3, vec4};
use itertools::Itertools;

use crate::{
    components::{
        ComponentVec, HasTransform3dComponent, Transformable,
        nvpoint::{NVPoint, NVPointSliceMethods},
        rgba::Rgba,
        vpoint::{VPoint, VPointSliceMethods},
        width::Width,
    },
    context::WgpuContext,
    prelude::{Alignable, Empty, Fill, Interpolatable, Opacity, Partial, Stroke},
    render::primitives::{
        ExtractFrom, RenderInstance, RenderInstances, nvitem::NVItemPrimitive,
        vitem::VItemPrimitive,
    },
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
pub struct NVItem {
    pub nvpoints: ComponentVec<NVPoint>,
    pub stroke_widths: ComponentVec<Width>,
    pub stroke_rgbas: ComponentVec<Rgba>,
    pub fill_rgbas: ComponentVec<Rgba>,
}

impl HasTransform3dComponent for NVItem {
    type Component = NVPoint;
    fn transform_3d(&self) -> &ComponentVec<Self::Component> {
        &self.nvpoints
    }

    fn transform_3d_mut(&mut self) -> &mut ComponentVec<Self::Component> {
        &mut self.nvpoints
    }
}

impl NVItem {
    // TODO: remove all constructor to blueprint impl
    pub fn from_nvpoints(nvpoints: Vec<[Vec3; 3]>) -> Self {
        let stroke_widths = vec![1.0; nvpoints.len()];
        let stroke_rgbas = vec![vec4(1.0, 0.0, 0.0, 1.0); nvpoints.len()];
        let fill_rgbas = vec![vec4(0.0, 1.0, 0.0, 0.5); nvpoints.len()];
        Self {
            nvpoints: nvpoints.into(),
            stroke_rgbas: stroke_rgbas.into(),
            stroke_widths: stroke_widths.into(),
            fill_rgbas: fill_rgbas.into(),
        }
    }

    pub fn extend_vpoints(&mut self, vpoints: &[[Vec3; 3]]) {
        self.nvpoints
            .extend_from_vec(vpoints.iter().cloned().map(Into::into).collect());

        let len = self.nvpoints.len();
        self.fill_rgbas.resize_with_last((len + 1) / 2);
        self.stroke_rgbas.resize_with_last((len + 1) / 2);
        self.stroke_widths.resize_with_last((len + 1) / 2);
    }

    pub(crate) fn get_render_points(&self) -> Vec<crate::render::primitives::nvitem::NVPoint> {
        self.nvpoints
            .iter()
            .zip(self.nvpoints.get_closepath_flags().iter())
            .map(|(p, f)| crate::render::primitives::nvitem::NVPoint {
                prev_handle: p.0[0].extend(1.0),
                anchor: p.0[1].extend(1.0),
                next_handle: p.0[2].extend(1.0),
                closepath: if *f { 1.0 } else { 0.0 },
                _padding: [0.0, 0.0, 0.0],
            })
            .collect()
    }
}

// MARK: Entity impl
impl Entity for NVItem {
    fn get_render_instance_for_entity<'a>(
        &self,
        render_instances: &'a RenderInstances,
        entity_id: usize,
    ) -> Option<&'a dyn RenderInstance> {
        render_instances
            .get_dynamic::<NVItemPrimitive>(entity_id)
            .map(|x| x as &dyn RenderInstance)
    }
    fn prepare_render_instance_for_entity(
        &self,
        ctx: &WgpuContext,
        render_instances: &mut RenderInstances,
        entity_id: usize,
    ) {
        let render_instance = render_instances.get_dynamic_or_init::<NVItemPrimitive>(entity_id);
        render_instance.update_from(ctx, self);
    }
}

// MARK: Extract impl
impl ExtractFrom<NVItem> for NVItemPrimitive {
    fn update_from(&mut self, ctx: &WgpuContext, data: &NVItem) {
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
impl Alignable for NVItem {
    fn is_aligned(&self, other: &Self) -> bool {
        self.nvpoints.is_aligned(&other.nvpoints)
            && self.stroke_widths.is_aligned(&other.stroke_widths)
            && self.stroke_rgbas.is_aligned(&other.stroke_rgbas)
            && self.fill_rgbas.is_aligned(&other.fill_rgbas)
    }
    fn align_with(&mut self, other: &mut Self) {
        self.nvpoints.align_with(&mut other.nvpoints);
        self.stroke_rgbas.align_with(&mut other.stroke_rgbas);
        self.stroke_widths.align_with(&mut other.stroke_widths);
        self.fill_rgbas.align_with(&mut other.fill_rgbas);
    }
}

impl Interpolatable for NVItem {
    fn lerp(&self, target: &Self, t: f32) -> Self {
        let vpoints = self.nvpoints.lerp(&target.nvpoints, t);
        let stroke_rgbas = self.stroke_rgbas.lerp(&target.stroke_rgbas, t);
        let stroke_widths = self.stroke_widths.lerp(&target.stroke_widths, t);
        let fill_rgbas = self.fill_rgbas.lerp(&target.fill_rgbas, t);
        Self {
            nvpoints: vpoints,
            stroke_widths,
            stroke_rgbas,
            fill_rgbas,
        }
    }
}

impl Opacity for NVItem {
    fn set_opacity(&mut self, opacity: f32) -> &mut Self {
        self.stroke_rgbas.set_opacity(opacity);
        self.fill_rgbas.set_opacity(opacity);
        self
    }
}

impl Partial for NVItem {
    fn get_partial(&self, range: std::ops::Range<f32>) -> Self {
        let vpoints = self.nvpoints.get_partial(range.clone());
        let stroke_rgbas = self.stroke_rgbas.get_partial(range.clone());
        let stroke_widths = self.stroke_widths.get_partial(range.clone());
        let fill_rgbas = self.fill_rgbas.get_partial(range.clone());
        Self {
            nvpoints: vpoints,
            stroke_widths,
            stroke_rgbas,
            fill_rgbas,
        }
    }
}

impl Empty for NVItem {
    fn empty() -> Self {
        Self {
            nvpoints: vec![NVPoint::default(); 2].into(),
            stroke_widths: vec![0.0, 0.0].into(),
            stroke_rgbas: vec![Vec4::ZERO; 2].into(),
            fill_rgbas: vec![Vec4::ZERO; 2].into(),
        }
    }
}

impl Fill for NVItem {
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

impl Stroke for NVItem {
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

#[derive(Default)]
pub struct NVItemBuilder {
    start_point: Option<Vec3>,
    nvpoints: Vec<[Vec3; 3]>,
}

impl NVItemBuilder {
    pub fn is_empty(&self) -> bool {
        self.nvpoints.is_empty()
    }
    pub fn new() -> Self {
        Self::default()
    }
    pub fn len(&self) -> usize {
        self.nvpoints.len()
    }

    /// Starts a new subpath and push the point as the start_point
    pub fn move_to(&mut self, point: Vec3) -> &mut Self {
        self.start_point = Some(point);
        // Close the previous path
        if let Some(end) = self.nvpoints.last_mut() {
            end[2] = end[1];
        }
        self.nvpoints.push([point; 3]);
        self
    }

    fn assert_started(&self) {
        assert!(
            self.start_point.is_some() || self.nvpoints.is_empty(),
            "A path have to start with move_to"
        );
    }

    /// Append a line
    pub fn line_to(&mut self, p: Vec3) -> &mut Self {
        self.assert_started();
        let last = self.nvpoints.last_mut().unwrap();
        let mid = (last[1] + p) / 2.0;
        last[2] = mid;
        self.nvpoints.push([mid, p, p]);
        self
    }

    /// Append a quadratic bezier
    pub fn quad_to(&mut self, h: Vec3, p: Vec3) -> &mut Self {
        self.assert_started();
        let last = self.nvpoints.last_mut().unwrap();
        if last[1].distance_squared(h) < f32::EPSILON || h.distance_squared(p) < f32::EPSILON {
            return self.line_to(p);
        }
        let prev_h = (last[1] + h * 2.0) / 3.0;
        let next_h = (2.0 * h + p) / 3.0;
        last[2] = prev_h;
        self.nvpoints.push([next_h, p, p]);
        self
    }

    /// Append a cubic bezier
    pub fn cubic_to(&mut self, h1: Vec3, h2: Vec3, p: Vec3) -> &mut Self {
        self.assert_started();
        let last = self.nvpoints.last_mut().unwrap();
        if last[1].distance_squared(h1) < f32::EPSILON || h1.distance_squared(h2) < f32::EPSILON {
            return self.quad_to(h2, p);
        }
        if h2.distance_squared(p) < f32::EPSILON {
            return self.quad_to(h1, p);
        }

        last[2] = h1;
        self.nvpoints.push([h2, p, p]);

        self
    }

    pub fn close_path(&mut self) -> &mut Self {
        self.assert_started();
        if self.nvpoints.last().unwrap()[1] == self.start_point.unwrap() {
            return self;
        }
        self.line_to(self.start_point.unwrap());
        self
    }

    pub fn nvpoints(&self) -> &[[Vec3; 3]] {
        &self.nvpoints
    }
}

impl Blueprint<NVItem> for NVItemBuilder {
    fn build(self) -> NVItem {
        NVItem::from_nvpoints(self.nvpoints().to_vec())
    }
}

/// A polygon defined by a list of corner points (Counter Clock wise).
pub struct Polygon(pub Vec<Vec3>);

impl Blueprint<NVItem> for Polygon {
    fn build(mut self) -> NVItem {
        assert!(self.0.len() > 2);

        // Close the polygon
        self.0.push(self.0[0]);

        let anchors = self.0;

        let mut builder = NVItemBuilder::new();
        builder.move_to(anchors[0]);
        anchors.iter().skip(1).for_each(|p| {
            builder.line_to(*p);
        });
        builder.close_path();
        builder.build()
    }
}

pub struct Rectangle(pub f32, pub f32);

impl Blueprint<NVItem> for Rectangle {
    fn build(self) -> NVItem {
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

impl Blueprint<NVItem> for Line {
    fn build(self) -> NVItem {
        let mid = (self.0 + self.1) / 2.0;
        NVItem::from_nvpoints(vec![[self.0, self.0, mid], [mid, self.1, self.1]])
    }
}

pub struct Square(pub f32);

impl Blueprint<NVItem> for Square {
    fn build(self) -> NVItem {
        Rectangle(self.0, self.0).build()
    }
}

pub struct Arc {
    pub angle: f32,
    pub radius: f32,
}

impl Blueprint<NVItem> for Arc {
    fn build(self) -> NVItem {
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

        let mut builder = NVItemBuilder::new();
        builder.move_to(points[0]);
        points
            .iter()
            .skip(1)
            .step_by(2)
            .zip(points.iter().skip(2).step_by(2))
            .for_each(|(h, p)| {
                builder.quad_to(*h, *p);
            });
        builder.build()
    }
}

pub struct ArcBetweenPoints {
    pub start: Vec3,
    pub end: Vec3,
    pub angle: f32,
}

impl Blueprint<NVItem> for ArcBetweenPoints {
    fn build(self) -> NVItem {
        let radius = (self.start.distance(self.end) / 2.0) / self.angle.sin();
        let arc = Arc {
            angle: self.angle,
            radius,
        };
        let mut item = arc.build();
        item.nvpoints.put_start_and_end_on(self.start, self.end);
        item
    }
}

/// A circle
pub struct Circle(pub f32);

impl Blueprint<NVItem> for Circle {
    fn build(self) -> NVItem {
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

impl Blueprint<NVItem> for Dot {
    fn build(self) -> NVItem {
        Circle(match self {
            Dot::Small => 0.04,
            Dot::Normal => 0.08,
        })
        .build()
    }
}

// width, height
pub struct Ellipse(pub f32, pub f32);

impl Blueprint<NVItem> for Ellipse {
    fn build(self) -> NVItem {
        let mut mobject = Circle(1.0).build();
        mobject.nvpoints.scale(vec3(self.0, self.1, 1.0));
        mobject
    }
}

#[cfg(test)]
mod test {
    use super::*;

    // #[test]
    // fn test_arc() {
    //     let arc = Arc {
    //         angle: std::f32::consts::PI / 2.0,
    //         radius: 1.0,
    //     }
    //     .build();
    //     println!("{:?}", arc.nvpoints);
    // }
}
