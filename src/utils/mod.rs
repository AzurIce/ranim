pub mod rate_functions;

use bezier_rs::{Bezier, Identifier, Join, Subpath, SubpathTValue};
use glam::{vec3, vec4, Vec3, Vec4};
use log::trace;

use crate::{
    mobject::Mobject,
    renderer::{
        vmobject::{VMobjectRenderer, VMobjectVertex},
        RendererVertex,
    },
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Id(u128);

impl Id {
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4().as_u128())
    }
}

impl Identifier for Id {
    fn new() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy)]
pub enum SubpathWidth {
    Inner(f32),
    Outer(f32),
    Middle(f32),
}

impl Default for SubpathWidth {
    fn default() -> Self {
        Self::Middle(1.0)
    }
}

const MAX_STEPS: usize = 128;

pub fn beziers_to_fill(beziers: &Vec<Bezier>, fill_color: Vec4) -> Vec<VMobjectVertex> {
    trace!("converting subpath to vertex: {:?}", beziers.len());

    let subpath: Subpath<Id> = Subpath::from_beziers(beziers, true);
    if subpath.len() == 0 {
        return vec![VMobjectVertex::default(); 3];
    }

    let mut vertices = vec![];
    for i in 0..MAX_STEPS {
        let t = i as f64 / (MAX_STEPS - 1) as f64;
        vertices.push(subpath.evaluate(SubpathTValue::GlobalEuclidean(t)));
    }

    vertices
        .windows(3)
        .flatten()
        .map(|p| VMobjectVertex::new(vec3(p.x as f32, p.y as f32, 0.0), fill_color))
        .collect::<Vec<_>>()
}

pub fn beziers_to_stroke(
    beziers: &Vec<Bezier>,
    width: SubpathWidth,
    stroke_color: Vec4,
    closed: bool,
) -> Vec<VMobjectVertex> {
    trace!("converting subpath to vertex: {:?}", beziers.len());

    let subpath: Subpath<Id> = Subpath::from_beziers(beziers, closed);
    if subpath.len() == 0 {
        return vec![VMobjectVertex::default(); 3];
    }

    // https://github.com/3b1b/manim/blob/master/manimlib/shaders/quadratic_bezier/stroke/geom.glsl
    let (inner_path, outer_path) = match width {
        SubpathWidth::Inner(w) => (
            subpath.offset(w as f64, Join::Bevel),
            subpath.offset(0.0, Join::Bevel),
        ),
        SubpathWidth::Outer(w) => (
            subpath.offset(0.0, Join::Bevel),
            subpath.offset(-w as f64, Join::Bevel),
        ),
        SubpathWidth::Middle(w) => (
            subpath.offset(w as f64 / 2.0, Join::Bevel),
            subpath.offset(-w as f64 / 2.0, Join::Bevel),
        ),
    };
    trace!(
        "inner: {:?}, outer: {:?}",
        inner_path.len(),
        outer_path.len()
    );
    let mut vertices = vec![];
    for i in 0..MAX_STEPS {
        let t = i as f64 / (MAX_STEPS - 1) as f64;
        vertices.push(inner_path.evaluate(SubpathTValue::GlobalEuclidean(t)));
        trace!("{:?}", vertices.last().unwrap());
        vertices.push(outer_path.evaluate(SubpathTValue::GlobalEuclidean(t)));
        trace!("{:?}", vertices.last().unwrap());
    }

    vertices
        .windows(3)
        .flatten()
        .map(|p| VMobjectVertex::new(vec3(p.x as f32, p.y as f32, 0.0), stroke_color))
        .collect::<Vec<_>>()
}

pub fn resize_preserving_order<T: Clone>(vec: &Vec<T>, new_len: usize) -> Vec<T> {
    let indices = (0..new_len).map(|i| i * vec.len() / new_len);
    indices.map(|i| vec[i].clone()).collect()
}

pub fn extend_with_last<T: Clone + Default>(vec: &mut Vec<T>, new_len: usize) {
    let v = vec![vec.last().cloned().unwrap_or_default(); new_len - vec.len()];
    vec.extend(v.into_iter())
}

pub fn put_start_and_end_on<T: RendererVertex + Default>(
    vec: Vec<T>,
    start: Vec3,
    end: Vec3,
) -> Vec<T> {
    let mut mobject = Mobject::new::<VMobjectRenderer>(vec);
    mobject.put_start_and_end_on(start, end);
    mobject.into_points()
}
