pub mod rate_functions;

use bezier_rs::{Bezier, Identifier, Join, Subpath, SubpathTValue};
use glam::{vec2, vec3, vec4, Mat3, Vec2, Vec3, Vec3Swizzles, Vec4};
use log::trace;

use crate::rabject::vmobject::{VMobjectPoint, VMobjectStrokeVertex};

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

/// Projects a 3D point onto a plane defined by a unit normal vector.
pub fn project(p: Vec3, unit_normal: Vec3) -> Vec3 {
    // trace!("projecting {:?} by {:?}", p, unit_normal);
    // trace!("dot: {:?}", unit_normal.dot(p));
    // trace!("res: {:?}", p - unit_normal * unit_normal.dot(p));
    p - unit_normal * unit_normal.dot(p)
}

pub fn generate_basis(unit_normal: Vec3) -> (Vec3, Vec3) {
    // trace!("generating basis for {:?}", unit_normal);
    let u = if unit_normal.x != 0.0 || unit_normal.y != 0.0 {
        vec3(-unit_normal.y, unit_normal.x, 0.0)
    } else {
        vec3(1.0, 0.0, 0.0)
    }
    .normalize();
    let v = unit_normal.cross(u).normalize();
    (u, v)
}

pub fn convert_to_2d(p: Vec3, origin: Vec3, basis: (Vec3, Vec3)) -> Vec2 {
    // trace!("converting {:?} by {:?} and {:?}", p, origin, basis);
    let p_local = p - origin;
    vec2(basis.0.dot(p_local), basis.1.dot(p_local))
}

pub fn convert_to_3d(p: Vec2, origin: Vec3, basis: (Vec3, Vec3)) -> Vec3 {
    origin + basis.0 * p.x + basis.1 * p.y
}

pub fn rotation_between_vectors(v1: Vec3, v2: Vec3) -> Mat3 {
    // trace!("rotation_between_vectors: v1: {:?}, v2: {:?}", v1, v2);
    
    if (v2 - v1).length() < std::f32::EPSILON {
        return Mat3::IDENTITY;
    }
    let mut axis = v1.cross(v2);
    if axis.length() < std::f32::EPSILON {
        axis = v1.cross(Vec3::Y);
    }
    if axis.length() < std::f32::EPSILON {
        axis = v1.cross(Vec3::Z);
    }
    // trace!("axis: {:?}", axis);

    let angle = angle_between_vectors(v1, v2);
    // trace!("angle: {:?}", angle);
    Mat3::from_axis_angle(axis, angle)
}

pub fn angle_between_vectors(v1: Vec3, v2: Vec3) -> f32 {
    if v1.length() == 0.0 || v2.length() == 0.0 {
        return 0.0;
    }

    (v1.dot(v2) / (v1.length() * v2.length()))
        .clamp(-1.0, 1.0)
        .acos()
}

const MAX_STEPS: usize = 128;

// pub fn beziers_to_fill(beziers: &Vec<Bezier>, fill_color: Vec4) -> Vec<VMobjectVertex> {
//     trace!("converting subpath to vertex: {:?}", beziers.len());

//     let subpath: Subpath<Id> = Subpath::from_beziers(beziers, true);
//     if subpath.len() == 0 {
//         return vec![VMobjectVertex::default(); 3];
//     }

//     let mut vertices = vec![];
//     for i in 0..MAX_STEPS {
//         let t = i as f64 / (MAX_STEPS - 1) as f64;
//         vertices.push(subpath.evaluate(SubpathTValue::GlobalEuclidean(t)));
//     }

//     vertices
//         .windows(3)
//         .flatten()
//         .map(|p| VMobjectVertex::new(vec3(p.x as f32, p.y as f32, 0.0), fill_color))
//         .collect::<Vec<_>>()
// }

/// Convert a series of points to stroke vertices.
///
/// points are Anchor-Handle-Anchor-Handle-...-Anchor series
// pub fn points_to_stroke(points: &Vec<VMobjectPoint>) -> Vec<VMobjectStrokeVertex> {

// }

pub fn beziers_to_stroke(
    beziers: &Vec<Bezier>,
    width: SubpathWidth,
    stroke_color: Vec4,
    closed: bool,
) -> Vec<VMobjectStrokeVertex> {
    trace!("converting subpath to vertex: {:?}", beziers.len());

    let subpath: Subpath<Id> = Subpath::from_beziers(beziers, closed);
    if subpath.len() == 0 {
        return vec![VMobjectStrokeVertex::default(); 3];
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
        .map(|p| VMobjectStrokeVertex {
            pos: vec4(p.x as f32, p.y as f32, 0.0, 1.0),
            stroke_color,
        })
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
