use glam::{vec2, vec3, vec4, Vec2, Vec3, Vec4, Vec4Swizzles};
use itertools::Itertools;
use log::trace;

use crate::{
    components::{
        rgba::Rgba, vpoint::VPoint, width::Width, ComponentData, HasTransform3d, TransformAnchor,
    },
    context::WgpuContext,
    prelude::{Alignable, Empty, Fill, Interpolatable, Opacity, Partial, Stroke},
    render::{
        primitives::{vitem::VItemPrimitive, Extract},
        CameraFrame,
    },
};

use super::{Blueprint, Entity};

#[derive(Debug, Clone)]
pub struct VItem {
    pub vpoints: ComponentData<VPoint>,
    pub stroke_widths: ComponentData<Width>,
    pub stroke_rgbas: ComponentData<Rgba>,
    pub fill_rgbas: ComponentData<Rgba>,
}

impl HasTransform3d for VItem {
    fn get(&self) -> &ComponentData<impl crate::components::Transform3d + Default + Clone> {
        &self.vpoints
    }
    fn get_mut(
        &mut self,
    ) -> &mut ComponentData<impl crate::components::Transform3d + Default + Clone> {
        &mut self.vpoints
    }
}

impl VItem {
    pub fn from_vpoints(vpoints: Vec<Vec3>) -> Self {
        let stroke_widths = vec![1.0; vpoints.len()];
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
    type Primitive = VItemPrimitive;

    fn clip_box(&self, camera: &CameraFrame) -> [Vec2; 4] {
        let corners = self.vpoints.get_bounding_box_corners().map(|p| {
            let mut p = camera.view_projection_matrix() * p.extend(1.0);
            p /= p.w;
            p.xy()
        });
        let (mut min_x, mut max_x, mut min_y, mut max_y) = (1.0f32, -1.0f32, 1.0f32, -1.0f32);
        for p in corners {
            min_x = min_x.min(p.x);
            max_x = max_x.max(p.x);
            min_y = min_y.min(p.y);
            max_y = max_y.max(p.y);
        }
        let max_width = self
            .stroke_widths
            .iter()
            .cloned()
            .reduce(|acc, w| acc.max(w))
            .map(|w| w.0)
            .unwrap_or(0.0);
        let radii = Vec2::splat(max_width) / camera.half_frame_size();
        min_x -= radii.x;
        min_y -= radii.y;
        max_x += radii.x;
        max_y += radii.y;

        [
            vec2(min_x, min_y),
            vec2(min_x, max_y),
            vec2(max_x, min_y),
            vec2(max_x, max_y),
        ]
    }
}

// MARK: Extract impl
impl Extract<VItem> for VItemPrimitive {
    fn update(&mut self, ctx: &WgpuContext, data: &VItem) {
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
    fn set_fill_color(&mut self, color: bevy_color::Srgba) -> &mut Self {
        self.fill_rgbas.set_all(color);
        self
    }
    fn set_fill_opacity(&mut self, opacity: f32) -> &mut Self {
        self.fill_rgbas.set_opacity(opacity);
        self
    }
}

impl Stroke for VItem {
    fn set_stroke_color(&mut self, color: bevy_color::Srgba) -> &mut Self {
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

/// A polygon defined by a list of corner points (Clock wise).
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
        let points = anchors
            .into_iter()
            .interleave(handles.into_iter())
            .collect::<Vec<_>>();
        trace!("points: {:?}", points);
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
            vec3(-half_width, half_height, 0.0),
            vec3(half_width, half_height, 0.0),
            vec3(half_width, -half_height, 0.0),
        ])
        .build()
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
        mobject
            .vpoints
            .scale(vec3(self.0, self.1, 1.0), TransformAnchor::origin());
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
