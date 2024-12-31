use glam::Vec3;
use itertools::Itertools;
use vello::kurbo::{ParamCurve, PathSeg, QuadBez};

use crate::prelude::Interpolatable;

pub fn point_on_cubic_bezier(points: &[Vec3; 4], t: f32) -> Vec3 {
    let t = t.clamp(0.0, 1.0);
    let p0 = points[0].lerp(points[1], t);
    let p1 = points[1].lerp(points[2], t);
    let p2 = points[2].lerp(points[3], t);

    let p0 = p0.lerp(p1, t);
    let p1 = p1.lerp(p2, t);

    p0.lerp(p1, t)
}

pub fn split_cubic_bezier(bezier: &[Vec3; 4], t: f32) -> ([Vec3; 4], [Vec3; 4]) {
    let [p0, h0, h1, p1] = bezier;

    let split_point = &point_on_cubic_bezier(bezier, t);

    let h00 = p0.lerp(*h0, t);
    let h01 = p0.lerp(*h0, t).lerp(h0.lerp(*h1, t), t);
    let h10 = h0.lerp(*h1, t).lerp(h1.lerp(*p1, t), t);
    let h11 = h1.lerp(*p1, t);

    ([*p0, h00, h01, *split_point], [*split_point, h10, h11, *p1])
}

pub fn trim_cubic_bezier(bezier: &[Vec3; 4], a: f32, b: f32) -> [Vec3; 4] {
    // trace!("trim_cubic_bezier: {:?}, {:?}, {:?}", bezier, a, b);
    let (a, b) = if a > b { (b, a) } else { (a, b) };
    let end_on_b = split_cubic_bezier(bezier, b).0;
    // trace!("end_on_b: {:?}", end_on_b);
    let a = a / b;
    let result = split_cubic_bezier(&end_on_b, a).1;
    // trace!("result: {:?}", result);
    result
}

/// Returns the point on a quadratic bezier curve at the given parameter.
pub fn point_on_quadratic_bezier<T: Interpolatable>(points: &[T; 3], t: f32) -> T {
    let t = t.clamp(0.0, 1.0);
    let p1 = points[0].lerp(&points[1], t);
    let p2 = points[1].lerp(&points[2], t);
    p1.lerp(&p2, t)
}

/// Returns the control points of the given part of a quadratic bezier curve.
pub fn partial_quadratic_bezier<T: Interpolatable>(points: &[T; 3], a: f32, b: f32) -> [T; 3] {
    let a = a.clamp(0.0, 1.0);
    let b = b.clamp(0.0, 1.0);

    let h0 = point_on_quadratic_bezier(points, a);
    let h2 = point_on_quadratic_bezier(points, b);

    let h1_prime = points[1].lerp(&points[2], a);
    let end_prop = (b - a) / (1.0 - a);
    let h1 = h0.lerp(&h1_prime, end_prop);
    [h0, h1, h2]
}


pub fn divide_segment_to_n_part(segment: PathSeg, n: usize) -> Vec<PathSeg> {
    let alpha = (0..=n).map(|i| i as f64 / n as f64);
    match segment {
        PathSeg::Line(line) => alpha
            .tuple_windows()
            .map(|(a, b)| PathSeg::Line(line.subsegment(a..b)))
            .collect(),
        PathSeg::Quad(quad) => alpha
            .tuple_windows()
            .map(|(a, b)| PathSeg::Quad(quad.subsegment(a..b)))
            .collect(),
        PathSeg::Cubic(cubic) => alpha
            .tuple_windows()
            .map(|(a, b)| PathSeg::Cubic(cubic.subsegment(a..b)))
            .collect(),
    }
}