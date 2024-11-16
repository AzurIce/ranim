use core::f32;
use std::any::TypeId;

use bezier_rs::{
    Bezier, BezierHandles, Identifier, Join, ManipulatorGroup, Subpath, SubpathTValue,
};
use glam::{dvec2, vec3, vec4, DVec2, DVec3, Vec3};
use itertools::Itertools;

use crate::{
    pipeline::{
        simple::{SimplePipeline, SimpleVertex},
        RenderPipeline,
    },
    Renderable,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Id(u128);

impl Identifier for Id {
    fn new() -> Self {
        Self(uuid::Uuid::new_v4().as_u128())
    }
}

/// A part of a circle
pub struct Arc {
    /// Angle in radians of the arc
    pub angle: f64,
}

impl Renderable for Arc {
    type Vertex = SimpleVertex;

    fn pipeline_id(&self) -> TypeId {
        std::any::TypeId::of::<SimplePipeline>()
    }

    fn vertex_data(&self) -> Vec<Self::Vertex> {
        const NUM_SEGMENTS: usize = 8;
        let len = 2 * NUM_SEGMENTS + 1;

        let angle_step = self.angle / (len - 1) as f64;
        let mut points = (0..len)
            .map(|i| {
                let angle = i as f64 * angle_step;
                println!("{i}/{len} angle: {:?}", angle / std::f64::consts::PI);
                dvec2(angle.cos() as f64, angle.sin() as f64)
            })
            .collect::<Vec<_>>();

        let theta = self.angle / NUM_SEGMENTS as f64;
        points.iter_mut().skip(1).step_by(2).for_each(|p| {
            *p /= (theta / 2.0).cos();
        });
        println!("start: {:?}, end: {:?}", points[0], points[len - 1]);

        let beziers = points
            .iter()
            .step_by(2)
            .zip(points.iter().skip(1).step_by(2))
            .zip(points.iter().skip(2).step_by(2))
            .map(|((p1, p2), p3)| Bezier::from_quadratic_dvec2(*p1, *p2, *p3))
            .collect::<Vec<_>>();
        let subpath: Subpath<Id> = Subpath::from_beziers(&beziers, false);

        subpath.vertex_data()
    }
}

pub struct Polygon {
    vertices: Vec<DVec2>,
}

impl Polygon {
    pub fn from_verticies(vertices: Vec<DVec2>) -> Self {
        Self { vertices }
    }
}

impl Renderable for Polygon {
    type Vertex = SimpleVertex;

    fn pipeline_id(&self) -> TypeId {
        std::any::TypeId::of::<SimplePipeline>()
    }

    fn vertex_data(&self) -> Vec<Self::Vertex> {
        // TODO: Handle 0 len
        if self.vertices.len() == 0 {
            return vec![];
        }

        let mut vertices = self.vertices.clone();
        // vertices.push(*vertices.first().unwrap());

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
        subpath.vertex_data()
    }
}

struct MobjectVertexData {
    point: [f32; 3],
    colors: [f32; 4],
}

pub struct Mobject {
    data: Vec<MobjectVertexData>,
}

struct VMobjectVertexData {
    point: [f32; 3],
    stroke_rgba: [f32; 4],
    stroke_width: f32,
    joint_angle: f32,
    fill_rgba: [f32; 4],
    base_normal: [f32; 3],
    fill_border_width: f32,
}

pub struct VMobject {
    data: Vec<VMobjectVertexData>,
}

impl<ManipulatorGroupId: Identifier> Renderable for Subpath<ManipulatorGroupId> {
    type Vertex = SimpleVertex;

    fn pipeline_id(&self) -> TypeId {
        std::any::TypeId::of::<SimplePipeline>()
    }

    fn vertex_data(&self) -> Vec<Self::Vertex> {
        const POLYLINE_FACTOR: usize = 100;
        const MAX_STEPS: usize = 256;

        // https://github.com/3b1b/manim/blob/master/manimlib/shaders/quadratic_bezier/stroke/geom.glsl
        let inner_path = self.offset(1.0, Join::Bevel);
        let outer_path = self.offset(0.0, Join::Bevel);
        let mut vertices = vec![];
        for i in 0..MAX_STEPS {
            let t = i as f64 / (MAX_STEPS - 1) as f64;
            vertices.push(inner_path.evaluate(SubpathTValue::GlobalEuclidean(t)));
            vertices.push(outer_path.evaluate(SubpathTValue::GlobalEuclidean(t)));
        }

        println!(
            "inner_start: {:?}, inner_end: {:?}",
            inner_path.evaluate(SubpathTValue::GlobalEuclidean(0.0)),
            inner_path.evaluate(SubpathTValue::GlobalEuclidean(1.0))
        );
        println!(
            "outer_start: {:?}, outer_end: {:?}",
            outer_path.evaluate(SubpathTValue::GlobalEuclidean(0.0)),
            outer_path.evaluate(SubpathTValue::GlobalEuclidean(1.0))
        );

        vertices
            .windows(3)
            .flatten()
            .map(|p| SimpleVertex::new(vec3(p.x as f32, p.y as f32, 0.0), vec4(1.0, 0.0, 0.0, 1.0)))
            .collect()
    }
}
