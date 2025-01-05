use std::cmp::Ordering;

use glam::Vec3;
use itertools::Itertools;
use vello::kurbo::{self, BezPath, CubicBez, Line, ParamCurve, PathEl, PathSeg, QuadBez};

use crate::prelude::{Alignable, Interpolatable};

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

pub fn divide_elements(mut elements: Vec<PathEl>) -> Vec<Vec<PathEl>> {
    // trace!("divide_bez_path {:?}", elements);
    let mut paths = vec![];

    while let Some(i) = elements
        .iter()
        .skip(1)
        .position(|p| matches!(p, PathEl::MoveTo(_)))
    {
        // trace!("elements: {:?}", elements);
        let path = elements.drain(0..i + 1).collect::<Vec<_>>();
        if path
            .iter()
            .filter(|e| !matches!(e, PathEl::MoveTo(_) | PathEl::ClosePath))
            .count()
            == 0
        {
            continue;
        }
        paths.push(path);
    }
    if elements
        .iter()
        .filter(|e| !matches!(e, PathEl::MoveTo(_) | PathEl::ClosePath))
        .count()
        != 0
    {
        paths.push(elements.into_iter().collect());
    }
    // trace!("result: {:?}", paths);
    paths
}

pub fn align_subpath(a: &mut Vec<PathEl>, b: &mut Vec<PathEl>) {
    // trace!("align_subpath from {} to {}", a.len(), b.len());
    // trace!("a: {:?}", a);
    // trace!("b: {:?}", b);
    let bez_a = BezPath::from_vec(a.clone());
    let bez_b = BezPath::from_vec(b.clone());

    let seg_a = bez_a.segments().collect_vec();
    let seg_b = bez_b.segments().collect_vec();
    let (seg_a, seg_b) = align_segments(seg_a, seg_b);

    *a = BezPath::from_path_segments(seg_a.into_iter())
        .elements()
        .to_vec();
    *b = BezPath::from_path_segments(seg_b.into_iter())
        .elements()
        .to_vec();
    // trace!("result: a: {:?}", a);
    // trace!("result: b: {:?}", b);
    // let segments
}

pub fn align_segments(a: Vec<PathSeg>, b: Vec<PathSeg>) -> (Vec<PathSeg>, Vec<PathSeg>) {
    let self_len = a.len();
    let other_len = b.len();
    // trace!(
    //     "aligning BezPath segments from {} to {}",
    //     self_len,
    //     other_len
    // );

    let (mut a, mut b) = if self_len != other_len {
        // println!(">>>> aligning BezPath {} {}", self_len, other_len);
        let len = self_len.max(other_len);

        let a = extend_segments(a, len);
        let b = extend_segments(b, len);
        // println!("<<<< aligned BezPath {} {}", self_segs.len(), other_segs.len());

        (a, b)
    } else {
        (a, b)
    };
    a.iter_mut().zip(b.iter_mut()).for_each(|(a, b)| {
        a.align_with(b);
    });
    // trace!("self_segs: {:?}", a);
    // trace!("other_segs: {:?}", b);
    (a, b)
}

pub fn extend_segments(segments: Vec<PathSeg>, len: usize) -> Vec<PathSeg> {
    // trace!("extend_segments from {} to {}", segments.len(), len);
    let mut lens = segments
        .iter()
        .map(|&seg| match seg {
            kurbo::PathSeg::Line(Line { p0, p1 }) => p0.distance(p1),
            kurbo::PathSeg::Quad(QuadBez { p0, p2, .. }) => p0.distance(p2),
            kurbo::PathSeg::Cubic(CubicBez { p0, p3, .. }) => p0.distance(p3),
        })
        .collect::<Vec<_>>();
    // println!("get_matched_segments {} from {} {}", len, self.inner.segments().try_len().unwrap_or(0), lens.len());

    let n = len - lens.len();
    let mut ipc = vec![0; lens.len()];
    for _ in 0..n {
        let i = lens
            .iter()
            .position_max_by(|x, y| x.partial_cmp(y).unwrap_or(Ordering::Equal))
            .unwrap();
        ipc[i] += 1;
        lens[i] *= ipc[i] as f64 / (ipc[i] + 1) as f64;
    }

    let mut new_segments = Vec::with_capacity(len);
    segments.into_iter().zip(ipc).for_each(|(seg, ipc)| {
        if ipc > 0 {
            let divided = divide_segment_to_n_part(seg, ipc + 1);
            new_segments.extend(divided)
        } else {
            new_segments.push(seg)
        }
    });

    new_segments
}
