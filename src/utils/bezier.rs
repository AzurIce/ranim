use glam::{Vec3, Vec3Swizzles};

use crate::{
    prelude::Interpolatable,
    utils::math::{cross2d, intersection},
};

/// A path builder based on quadratic beziers
pub struct PathBuilder {
    start_point: Option<Vec3>,
    points: Vec<Vec3>,
}

impl PathBuilder {
    pub fn new() -> Self {
        Self {
            start_point: None,
            points: Vec::new(),
        }
    }

    /// Starts a new subpath and push the point as the start_point
    pub fn move_to(&mut self, point: Vec3) -> &mut Self {
        self.start_point = Some(point);
        if let Some(end) = self.points.last() {
            self.points.extend_from_slice(&[*end, point]);
        } else {
            self.points.push(point);
        }
        self
    }

    fn assert_started(&self) {
        assert!(
            self.start_point.is_some() || self.points.is_empty(),
            "A path have to start with move_to"
        );
    }

    /// Append a line
    pub fn line_to(&mut self, p: Vec3) -> &mut Self {
        self.assert_started();
        let mid = (self.points.last().unwrap() + p) / 2.0;
        self.points.extend_from_slice(&[mid, p]);
        self
    }

    /// Append a quadratic bezier
    pub fn quad_to(&mut self, h: Vec3, p: Vec3) -> &mut Self {
        self.assert_started();
        self.points.extend_from_slice(&[h, p]);
        self
    }

    /// Append a cubic bezier
    pub fn cubic_to(&mut self, h1: Vec3, h2: Vec3, p: Vec3) -> &mut Self {
        self.assert_started();
        let cur = self.points.last().unwrap();
        if cur.distance_squared(h1) < f32::EPSILON || h1.distance_squared(h2) < f32::EPSILON {
            return self.quad_to(h2, p);
        }
        if h2.distance_squared(p) < f32::EPSILON {
            return self.quad_to(h1, p);
        }

        let quads = approx_cubic_with_quadratic([*cur, h1, h2, p]);
        self.points
            .extend(quads.into_iter().flat_map(|beziers| beziers[1..].to_vec()));

        self
    }

    pub fn close_path(&mut self) -> &mut Self {
        self.assert_started();
        if let Some(start) = self.start_point {
            self.line_to(start);
        }
        self
    }

    pub fn vpoints(&self) -> &[Vec3] {
        &self.points
    }
}

pub fn split_cubic_bezier(bezier: &[Vec3; 4], t: f32) -> ([Vec3; 4], [Vec3; 4]) {
    let [p0, h0, h1, p1] = bezier;

    let split_point = &cubic_bezier_eval(bezier, t);

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

pub fn cubic_bezier_eval(bezier: &[Vec3; 4], t: f32) -> Vec3 {
    let t = t.clamp(0.0, 1.0);
    let p0 = bezier[0].lerp(bezier[1], t);
    let p1 = bezier[1].lerp(bezier[2], t);
    let p2 = bezier[2].lerp(bezier[3], t);

    let p0 = p0.lerp(p1, t);
    let p1 = p1.lerp(p2, t);

    p0.lerp(p1, t)
}

pub fn quad_bezier_eval(bezier: &[Vec3; 3], t: f32) -> Vec3 {
    let t = t.clamp(0.0, 1.0);
    let p0 = bezier[0].lerp(bezier[1], t);
    let p1 = bezier[1].lerp(bezier[2], t);

    p0.lerp(p1, t)
}

/// Approx a cubic bezier with quadratic bezier
///
/// [Vec3; 4] is [p1, h1, h2, p2]
pub fn approx_cubic_with_quadratic(cubic: [Vec3; 4]) -> Vec<[Vec3; 3]> {
    let [p1, h1, h2, p2] = cubic;

    let p = h1 - p1;
    let q = h2 - 2. * h1 + p1;
    let r = p2 - 3. * h2 + 3. * h1 - p1;

    let a = cross2d(q.xy(), r.xy());
    let b = cross2d(p.xy(), r.xy());
    let c = cross2d(p.xy(), q.xy());
    let disc = b * b - 4. * a * c;
    let sqrt_disc = disc.sqrt();

    println!("{} {} {}, disc: {}", a, b, c, disc);
    let root = if a == 0.0 && b == 0.0 || a != 0.0 && disc < 0.0 {
        0.5
    } else if a == 0.0 {
        -c / b
    } else {
        let mut root = (-b + sqrt_disc) / (2. * a);
        if root <= 0.0 || root >= 1.0 {
            root = (b + sqrt_disc) / (-2. * a);
        }
        if root <= 0.0 || root >= 1.0 {
            root = 0.5
        }
        root
    };
    println!("{root}");

    let cut_point = cubic_bezier_eval(&cubic, root);
    let cut_tangent = quad_bezier_eval(&[h1 - p1, h2 - h1, p2 - h2], root);
    let p1_tangent = h1 - p1;
    let p2_tangent = p2 - h2;

    let i1 = intersection(p1, p1_tangent, cut_point, cut_tangent);
    let i2 = intersection(p2, p2_tangent, cut_point, cut_tangent);
    // TODO: Make this better
    // There is a possibility that cut_tangent equals to p1_tangent or p2_tangent
    // Example:
    // ```rust
    // PathBuilder::new()
    //     .move_to(vec3(0.0, 2.0, 0.0))
    //     .cubic_to(
    //         vec3(-2.0, 2.0, 0.0),
    //         vec3(1.0, 4.0, 0.0),
    //         vec3(0.0, 0.0, 0.0),
    //     )
    // ```
    let (Some(i1), Some(i2)) = (i1, i2) else {
        let root = if root > 0.5 {
            root / 2.0
        } else {
            0.5 + (1.0 - root) / 2.0
        };
        let cut_point = cubic_bezier_eval(&cubic, root);
        let cut_tangent = quad_bezier_eval(&[h1 - p1, h2 - h1, p2 - h2], root);
        let p1_tangent = h1 - p1;
        let p2_tangent = p2 - h2;
        let i1 = intersection(p1, p1_tangent, cut_point, cut_tangent).unwrap();
        let i2 = intersection(p2, p2_tangent, cut_point, cut_tangent).unwrap();
        return vec![[p1, i1, cut_point], [cut_point, i2, p2]];
    };

    vec![[p1, i1, cut_point], [cut_point, i2, p2]]
}

// pub fn divide_segment_to_n_part(segment: PathSeg, n: usize) -> Vec<PathSeg> {
//     let alpha = (0..=n).map(|i| i as f64 / n as f64);
//     match segment {
//         PathSeg::Line(line) => alpha
//             .tuple_windows()
//             .map(|(a, b)| PathSeg::Line(line.subsegment(a..b)))
//             .collect(),
//         PathSeg::Quad(quad) => alpha
//             .tuple_windows()
//             .map(|(a, b)| PathSeg::Quad(quad.subsegment(a..b)))
//             .collect(),
//         PathSeg::Cubic(cubic) => alpha
//             .tuple_windows()
//             .map(|(a, b)| PathSeg::Cubic(cubic.subsegment(a..b)))
//             .collect(),
//     }
// }

// pub fn divide_elements(mut elements: Vec<PathEl>) -> Vec<Vec<PathEl>> {
//     // trace!("divide_bez_path {:?}", elements);
//     let mut paths = vec![];

//     while let Some(i) = elements
//         .iter()
//         .skip(1)
//         .position(|p| matches!(p, PathEl::MoveTo(_)))
//     {
//         // trace!("elements: {:?}", elements);
//         let path = elements.drain(0..i + 1).collect::<Vec<_>>();
//         if path
//             .iter()
//             .filter(|e| !matches!(e, PathEl::MoveTo(_) | PathEl::ClosePath))
//             .count()
//             == 0
//         {
//             continue;
//         }
//         paths.push(path);
//     }
//     if elements
//         .iter()
//         .filter(|e| !matches!(e, PathEl::MoveTo(_) | PathEl::ClosePath))
//         .count()
//         != 0
//     {
//         paths.push(elements.into_iter().collect());
//     }
//     // trace!("result: {:?}", paths);
//     paths
// }

// pub fn align_subpath(a: &mut Vec<PathEl>, b: &mut Vec<PathEl>) {
//     // trace!("align_subpath from {} to {}", a.len(), b.len());
//     // trace!("a: {:?}", a);
//     // trace!("b: {:?}", b);
//     let bez_a = BezPath::from_vec(a.clone());
//     let bez_b = BezPath::from_vec(b.clone());

//     let seg_a = bez_a.segments().collect_vec();
//     let seg_b = bez_b.segments().collect_vec();
//     let (seg_a, seg_b) = align_segments(seg_a, seg_b);

//     *a = BezPath::from_path_segments(seg_a.into_iter())
//         .elements()
//         .to_vec();
//     *b = BezPath::from_path_segments(seg_b.into_iter())
//         .elements()
//         .to_vec();
//     // trace!("result: a: {:?}", a);
//     // trace!("result: b: {:?}", b);
//     // let segments
// }

// pub fn align_segments(a: Vec<PathSeg>, b: Vec<PathSeg>) -> (Vec<PathSeg>, Vec<PathSeg>) {
//     let self_len = a.len();
//     let other_len = b.len();
//     // trace!(
//     //     "aligning BezPath segments from {} to {}",
//     //     self_len,
//     //     other_len
//     // );

//     let (mut a, mut b) = if self_len != other_len {
//         // println!(">>>> aligning BezPath {} {}", self_len, other_len);
//         let len = self_len.max(other_len);

//         let a = extend_segments(a, len);
//         let b = extend_segments(b, len);
//         // println!("<<<< aligned BezPath {} {}", self_segs.len(), other_segs.len());

//         (a, b)
//     } else {
//         (a, b)
//     };
//     a.iter_mut().zip(b.iter_mut()).for_each(|(a, b)| {
//         // a.align_with(b);
//     });
//     // trace!("self_segs: {:?}", a);
//     // trace!("other_segs: {:?}", b);
//     (a, b)
// }

// pub fn extend_segments(segments: Vec<PathSeg>, len: usize) -> Vec<PathSeg> {
//     // trace!("extend_segments from {} to {}", segments.len(), len);
//     let mut lens = segments
//         .iter()
//         .map(|&seg| match seg {
//             kurbo::PathSeg::Line(Line { p0, p1 }) => p0.distance(p1),
//             kurbo::PathSeg::Quad(QuadBez { p0, p2, .. }) => p0.distance(p2),
//             kurbo::PathSeg::Cubic(CubicBez { p0, p3, .. }) => p0.distance(p3),
//         })
//         .collect::<Vec<_>>();
//     // println!("get_matched_segments {} from {} {}", len, self.inner.segments().try_len().unwrap_or(0), lens.len());

//     let n = len - lens.len();
//     let mut ipc = vec![0; lens.len()];
//     for _ in 0..n {
//         let i = lens
//             .iter()
//             .position_max_by(|x, y| x.partial_cmp(y).unwrap_or(Ordering::Equal))
//             .unwrap();
//         ipc[i] += 1;
//         lens[i] *= ipc[i] as f64 / (ipc[i] + 1) as f64;
//     }

//     let mut new_segments = Vec::with_capacity(len);
//     segments.into_iter().zip(ipc).for_each(|(seg, ipc)| {
//         if ipc > 0 {
//             let divided = divide_segment_to_n_part(seg, ipc + 1);
//             new_segments.extend(divided)
//         } else {
//             new_segments.push(seg)
//         }
//     });

//     new_segments
// }
