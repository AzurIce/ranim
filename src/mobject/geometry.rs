use bezier_rs::Bezier;
use glam::{vec2, Vec2, Vec3};
use itertools::Itertools;
// use log::trace;

use crate::{
    pipeline::simple,
    utils::{beziers_to_vertex, SubpathWidth},
};

use super::{Mobject, ToMobject};

/// A part of a circle
// #[mobject(SimplePipeline)]
#[derive(Debug, Clone)]
pub struct Arc {
    /// Angle in radians of the arc
    pub angle: f32,
    pub radius: f32,
    pub width: SubpathWidth,
}

impl Arc {
    pub fn new(angle: f32) -> Self {
        Self {
            angle,
            radius: 1.0,
            width: SubpathWidth::Middle(1.0),
        }
    }
    pub fn with_radius(mut self, radius: f32) -> Self {
        self.radius = radius;
        self
    }
    pub fn with_width(mut self, width: SubpathWidth) -> Self {
        self.width = width;
        self
    }
}

impl ToMobject for Arc {
    type Pipeline = simple::Pipeline;

    fn to_mobject(self) -> Mobject<simple::Vertex> {
        const NUM_SEGMENTS: usize = 8;
        let len = 2 * NUM_SEGMENTS + 1;

        let angle_step = self.angle / (len - 1) as f32;
        let mut points = (0..len)
            .map(|i| {
                let angle = i as f32 * angle_step;
                vec2(angle.cos() as f32, angle.sin() as f32)
            })
            .collect::<Vec<_>>();

        let theta = self.angle / NUM_SEGMENTS as f32;
        points.iter_mut().skip(1).step_by(2).for_each(|p| {
            *p /= (theta / 2.0).cos();
        });
        // trace!("start: {:?}, end: {:?}", points[0], points[len - 1]);

        let beziers = points
            .iter()
            .step_by(2)
            .zip(points.iter().skip(1).step_by(2))
            .zip(points.iter().skip(2).step_by(2))
            .map(|((&p1, &p2), &p3)| {
                let [p1, p2, p3] = [p1 * self.radius, p2 * self.radius, p3 * self.radius];
                Bezier::from_quadratic_dvec2(p1.as_dvec2(), p2.as_dvec2(), p3.as_dvec2())
            })
            .collect::<Vec<_>>();

        // trace!("beziers: {:?}", beziers.len());
        Mobject::new::<Self::Pipeline>(beziers_to_vertex(beziers, self.width, false))
    }
}

pub struct ArcBetweenPoints {
    pub start: Vec3,
    pub end: Vec3,
    pub angle: f32,
    pub width: SubpathWidth,
}

impl ArcBetweenPoints {
    pub fn new(start: Vec3, end: Vec3, angle: f32) -> Self {
        Self {
            start,
            end,
            angle,
            width: SubpathWidth::Middle(1.0),
        }
    }
    pub fn with_width(mut self, width: SubpathWidth) -> Self {
        self.width = width;
        self
    }
}

impl ToMobject for ArcBetweenPoints {
    type Pipeline = simple::Pipeline;

    fn to_mobject(self) -> Mobject<simple::Vertex> {
        let radius = (self.start.distance(self.end) / 2.0) / self.angle.sin();
        let arc = Arc::new(self.angle).with_radius(radius).with_width(self.width);
        let mut mobject = arc.to_mobject();
        mobject.put_start_and_end_on(self.start, self.end);
        mobject
    }
}

#[derive(Debug, Clone)]
pub struct Polygon {
    pub vertices: Vec<Vec2>,
    pub width: SubpathWidth,
}

impl Polygon {
    pub fn new(vertices: Vec<Vec2>) -> Self {
        Self {
            vertices,
            width: SubpathWidth::Middle(1.0),
        }
    }
    pub fn with_width(mut self, width: SubpathWidth) -> Self {
        self.width = width;
        self
    }
}

impl ToMobject for Polygon {
    type Pipeline = simple::Pipeline;

    fn to_mobject(self) -> Mobject<simple::Vertex> {
        // TODO: Handle 0 len
        if self.vertices.len() == 0 {
            return Mobject::new::<Self::Pipeline>(vec![]);
        }

        let vertices = self.vertices.clone();

        let anchors = vertices;
        let handles = anchors
            .windows(2)
            .map(|window| 0.5 * (window[0] + window[1]))
            .collect::<Vec<_>>();

        assert_eq!(anchors.len(), handles.len() + 1);

        let points = anchors
            .into_iter()
            .interleave(handles.into_iter())
            .collect::<Vec<_>>();
        let beziers = points
            .iter()
            .step_by(2)
            .zip(
                points
                    .iter()
                    .skip(1)
                    .chain(points.iter().take(1))
                    .step_by(2),
            )
            .zip(
                points
                    .iter()
                    .skip(2)
                    .chain(points.iter().take(2))
                    .step_by(2),
            )
            .map(|((&p1, &p2), &p3)| {
                Bezier::from_quadratic_dvec2(p1.as_dvec2(), p2.as_dvec2(), p3.as_dvec2())
            })
            .collect::<Vec<_>>();
        // println!("beziers: {:?}", beziers.len());
        Mobject::new::<Self::Pipeline>(beziers_to_vertex(beziers, self.width, true))
    }
}
