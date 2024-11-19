use bezier_rs::{Bezier, Subpath};
use glam::{dvec2, DVec2};
use itertools::Itertools;

use crate::{
    pipeline::simple,
    utils::{sub_path_to_vertex, Id},
};

/// A part of a circle
// #[mobject(SimplePipeline)]
#[derive(Debug, Clone)]
pub struct Arc {
    /// Angle in radians of the arc
    pub angle: f64,
}

impl Into<Vec<simple::Vertex>> for Arc {
    fn into(self) -> Vec<simple::Vertex> {
        const NUM_SEGMENTS: usize = 8;
        let len = 2 * NUM_SEGMENTS + 1;

        let angle_step = self.angle / (len - 1) as f64;
        let mut points = (0..len)
            .map(|i| {
                let angle = i as f64 * angle_step;
                // println!("{i}/{len} angle: {:?}", angle / std::f64::consts::PI);
                dvec2(angle.cos() as f64, angle.sin() as f64)
            })
            .collect::<Vec<_>>();

        let theta = self.angle / NUM_SEGMENTS as f64;
        points.iter_mut().skip(1).step_by(2).for_each(|p| {
            *p /= (theta / 2.0).cos();
        });
        // println!("start: {:?}, end: {:?}", points[0], points[len - 1]);

        let beziers = points
            .iter()
            .step_by(2)
            .zip(points.iter().skip(1).step_by(2))
            .zip(points.iter().skip(2).step_by(2))
            .map(|((p1, p2), p3)| Bezier::from_quadratic_dvec2(*p1, *p2, *p3))
            .collect::<Vec<_>>();
        let subpath: Subpath<Id> = Subpath::from_beziers(&beziers, false);

        sub_path_to_vertex(subpath)
    }
}

#[derive(Debug, Clone)]
pub struct Polygon {
    pub vertices: Vec<DVec2>,
}

impl Into<Vec<simple::Vertex>> for Polygon {
    fn into(self) -> Vec<simple::Vertex> {
        // TODO: Handle 0 len
        if self.vertices.len() == 0 {
            return vec![].into();
        }

        let vertices = self.vertices.clone();

        let anchors = vertices;
        let handles = anchors
            .windows(2)
            .map(|window| 0.5 * (window[0] + window[1]))
            .collect::<Vec<_>>();
        println!("anchors: {:?}", anchors.len());
        println!("handles: {:?}", handles.len());

        assert_eq!(anchors.len(), handles.len() + 1);

        let points = anchors
            .into_iter()
            .interleave(handles.into_iter())
            .collect::<Vec<_>>();
        println!("points: {:?}, {:?}", points.len(), points);
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
            .map(|((p1, p2), p3)| {
                println!("({:?}, {:?}, {:?})", p1, p2, p3);
                Bezier::from_quadratic_dvec2(*p1, *p2, *p3)
            })
            .collect::<Vec<_>>();
        println!("beziers: {:?}", beziers.len());
        let subpath: Subpath<Id> = Subpath::from_beziers(&beziers, true);
        sub_path_to_vertex(subpath)
    }
}

impl Polygon {
    pub fn from_verticies(vertices: Vec<DVec2>) -> Self {
        Self { vertices }
    }
}
