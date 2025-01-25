use std::ops::Deref;

use glam::{vec2, vec3, vec4, Vec3, Vec4};
use itertools::Itertools;
use log::trace;

use crate::{
    components::{rgba::Rgba, vpoint::VPoint, width::Width, ComponentData, TransformAnchor},
    render::primitives::vitem::VItemPrimitive,
};

use super::{Blueprint, Entity};

#[derive(Debug, Clone)]
pub struct VItem {
    pub vpoints: ComponentData<VPoint>,
    pub stroke_widths: ComponentData<Width>,
    pub stroke_rgbas: ComponentData<Rgba>,
    pub fill_rgbas: ComponentData<Rgba>,
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
}

pub struct ExtractedVItem {
    pub points: Vec<Vec4>,
    pub stroke_widths: Vec<f32>,
    pub stroke_rgbas: Vec<Vec4>,
    pub fill_rgbas: Vec<Vec4>,
}

impl Entity for VItem {
    type ExtractData = ExtractedVItem;
    type Primitive = VItemPrimitive;
    fn extract(&self) -> Option<Self::ExtractData> {
        Some(ExtractedVItem {
            points: self
                .vpoints
                .iter()
                .zip(self.vpoints.get_closepath_flags().iter())
                .map(|(p, f)| vec4(p.x, p.y, p.z, if *f { 1.0 } else { 0.0 }))
                .collect(),
            stroke_widths: self.stroke_widths.iter().map(|w| *w.deref()).collect(),
            stroke_rgbas: self.stroke_rgbas.iter().map(|c| *c.deref()).collect(),
            fill_rgbas: self.fill_rgbas.iter().map(|c| *c.deref()).collect(),
        })
    }
}

// impl Rabject for VItem {
//     type ExtractData = ExtractedVItem;
//     type RenderResource = VItemPrimitive;
//     fn extract(&self) -> Self::ExtractData {
//         Renderable::extract(self)
//     }
// }

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
    pub stroke_width: f32,
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
