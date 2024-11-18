use bezier_rs::{Identifier, Join, Subpath, SubpathTValue};
use glam::{vec3, vec4};

use crate::pipeline::simple;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Id(u128);

impl Identifier for Id {
    fn new() -> Self {
        Self(uuid::Uuid::new_v4().as_u128())
    }
}

pub fn sub_path_to_vertex<Id: Identifier>(subpath: Subpath<Id>) -> Vec<simple::Vertex> {
    const MAX_STEPS: usize = 256;

    // https://github.com/3b1b/manim/blob/master/manimlib/shaders/quadratic_bezier/stroke/geom.glsl
    let inner_path = subpath.offset(1.0, Join::Bevel);
    let outer_path = subpath.offset(0.0, Join::Bevel);
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
        .map(|p| simple::Vertex::new(vec3(p.x as f32, p.y as f32, 0.0), vec4(1.0, 0.0, 0.0, 1.0)))
        .collect::<Vec<_>>()
}
