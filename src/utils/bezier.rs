use glam::{DVec3, Vec3Swizzles};
use itertools::Itertools;

use crate::{
    prelude::Interpolatable,
    utils::math::{cross2d, intersection},
};

/// A path builder based on quadratic beziers
#[derive(Default)]
pub struct PathBuilder {
    start_point: Option<DVec3>,
    points: Vec<DVec3>,
}

impl PathBuilder {
    pub fn is_empty(&self) -> bool {
        self.points.is_empty()
    }
    pub fn new() -> Self {
        Self::default()
    }
    pub fn len(&self) -> usize {
        self.points.len()
    }

    /// Starts a new subpath and push the point as the start_point
    pub fn move_to(&mut self, point: DVec3) -> &mut Self {
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
    pub fn line_to(&mut self, p: DVec3) -> &mut Self {
        self.assert_started();
        let mid = (self.points.last().unwrap() + p) / 2.0;
        self.points.extend_from_slice(&[mid, p]);
        self
    }

    /// Append a quadratic bezier
    pub fn quad_to(&mut self, h: DVec3, p: DVec3) -> &mut Self {
        self.assert_started();
        let cur = self.points.last().unwrap();
        if cur.distance_squared(h) < f64::EPSILON || h.distance_squared(p) < f64::EPSILON {
            return self.line_to(p);
        }
        self.points.extend_from_slice(&[h, p]);
        self
    }

    /// Append a cubic bezier
    pub fn cubic_to(&mut self, h1: DVec3, h2: DVec3, p: DVec3) -> &mut Self {
        self.assert_started();
        let cur = self.points.last().unwrap();
        if cur.distance_squared(h1) < f64::EPSILON || h1.distance_squared(h2) < f64::EPSILON {
            return self.quad_to(h2, p);
        }
        if h2.distance_squared(p) < f64::EPSILON {
            return self.quad_to(h1, p);
        }

        let quads = approx_cubic_with_quadratic([*cur, h1, h2, p]);
        for quad in quads {
            self.quad_to(quad[1], quad[2]);
        }

        self
    }

    pub fn close_path(&mut self) -> &mut Self {
        self.assert_started();
        if self.points.last() == self.start_point.as_ref() {
            return self;
        }
        self.line_to(self.start_point.unwrap());
        self
    }

    pub fn vpoints(&self) -> &[DVec3] {
        &self.points
    }
}

pub fn split_cubic_bezier(bezier: &[DVec3; 4], t: f64) -> ([DVec3; 4], [DVec3; 4]) {
    let [p0, h0, h1, p1] = bezier;

    let split_point = &cubic_bezier_eval(bezier, t);

    let h00 = p0.lerp(h0, t);
    let h01 = p0.lerp(h0, t).lerp(h0.lerp(h1, t), t);
    let h10 = h0.lerp(h1, t).lerp(h1.lerp(p1, t), t);
    let h11 = h1.lerp(p1, t);

    ([*p0, h00, h01, *split_point], [*split_point, h10, h11, *p1])
}

pub fn split_quad_bezier(bezier: &[DVec3; 3], t: f64) -> ([DVec3; 3], [DVec3; 3]) {
    let [p0, h, p1] = bezier;

    let split_point = &quad_bezier_eval(bezier, t);

    let h0 = p0.lerp(h, t);
    let h1 = h.lerp(p1, t);

    ([*p0, h0, *split_point], [*split_point, h1, *p1])
}

pub fn trim_quad_bezier(bezier: &[DVec3; 3], a: f64, b: f64) -> [DVec3; 3] {
    // trace!("!!!trim_quad_bezier: {:?}, {:?}, {:?}", bezier, a, b);
    let (a, b) = if a > b { (b, a) } else { (a, b) };
    let end_on_b = split_quad_bezier(bezier, b).0;
    let a = a / b;
    split_quad_bezier(&end_on_b, a).1
}

pub fn trim_cubic_bezier(bezier: &[DVec3; 4], a: f64, b: f64) -> [DVec3; 4] {
    // trace!("trim_cubic_bezier: {:?}, {:?}, {:?}", bezier, a, b);
    let (a, b) = if a > b { (b, a) } else { (a, b) };
    let end_on_b = split_cubic_bezier(bezier, b).0;
    // trace!("end_on_b: {:?}", end_on_b);
    let a = a / b;
    split_cubic_bezier(&end_on_b, a).1
}

/// When path is empty, returns None
pub fn get_subpath_closed_flag(path: &[DVec3]) -> Option<(usize, bool)> {
    if path.len() < 3 {
        return None;
    }
    for mut chunk in &path.iter().enumerate().skip(2).chunks(2) {
        let (a, b) = (chunk.next(), chunk.next());
        // println!("{:?} {:?}", a, b);
        if let Some((ia, a)) = match (a, b) {
            (Some((ia, a)), Some((_ib, b))) => {
                // println!("chunk[{ia}, {_ib}] {:?}", [a, b]);
                if a == b { Some((ia, a)) } else { None }
            }
            (Some((ia, a)), None) => Some((ia, a)),
            _ => unreachable!(),
        } {
            // println!("### path end ###");
            if (a - path[0]).length_squared() <= 0.0001 {
                return Some((ia, true));
            } else {
                return Some((ia, false));
            }
        }
    }
    unreachable!()
}

/// Returns the point on a quadratic bezier curve at the given parameter.
pub fn point_on_quadratic_bezier<T: Interpolatable>(points: &[T; 3], t: f64) -> T {
    let t = t.clamp(0.0, 1.0);
    let p1 = points[0].lerp(&points[1], t);
    let p2 = points[1].lerp(&points[2], t);
    p1.lerp(&p2, t)
}

/// Returns the control points of the given part of a quadratic bezier curve.
pub fn partial_quadratic_bezier<T: Interpolatable>(points: &[T; 3], a: f64, b: f64) -> [T; 3] {
    let a = a.clamp(0.0, 1.0);
    let b = b.clamp(0.0, 1.0);

    let h0 = point_on_quadratic_bezier(points, a);
    let h2 = point_on_quadratic_bezier(points, b);

    let h1_prime = points[1].lerp(&points[2], a);
    let end_prop = (b - a) / (1.0 - a);
    let h1 = h0.lerp(&h1_prime, end_prop);
    [h0, h1, h2]
}

pub fn cubic_bezier_eval(bezier: &[DVec3; 4], t: f64) -> DVec3 {
    let t = t.clamp(0.0, 1.0);
    let p0 = bezier[0].lerp(bezier[1], t);
    let p1 = bezier[1].lerp(bezier[2], t);
    let p2 = bezier[2].lerp(bezier[3], t);

    let p0 = p0.lerp(p1, t);
    let p1 = p1.lerp(p2, t);

    p0.lerp(p1, t)
}

pub fn quad_bezier_eval(bezier: &[DVec3; 3], t: f64) -> DVec3 {
    let t = t.clamp(0.0, 1.0);
    let p0 = bezier[0].lerp(bezier[1], t);
    let p1 = bezier[1].lerp(bezier[2], t);

    p0.lerp(p1, t)
}

/// Approx a cubic bezier with quadratic bezier
///
/// [Vec3; 4] is [p1, h1, h2, p2]
pub fn approx_cubic_with_quadratic(cubic: [DVec3; 4]) -> Vec<[DVec3; 3]> {
    // trace!("approx cubic {:?}", cubic);
    let [p1, h1, h2, p2] = cubic;

    let p = h1 - p1;
    let q = h2 - 2. * h1 + p1;
    let r = p2 - 3. * h2 + 3. * h1 - p1;

    let a = cross2d(q.xy(), r.xy());
    let b = cross2d(p.xy(), r.xy());
    let c = cross2d(p.xy(), q.xy());
    let disc = b * b - 4. * a * c;
    let sqrt_disc = disc.sqrt();

    // println!("{} {} {}, disc: {}", a, b, c, disc);
    let mut root = if a == 0.0 && b == 0.0 || a != 0.0 && disc < 0.0 {
        0.5
    } else if a == 0.0 {
        (-c / b).clamp(0.0, 1.0)
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
    if root == 0.0 || root == 1.0 {
        root = 0.5;
    }
    // println!("{root}");

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
        let i1 =
            intersection(p1, p1_tangent, cut_point, cut_tangent).unwrap_or((p1 + cut_point) / 2.0);
        let i2 =
            intersection(p2, p2_tangent, cut_point, cut_tangent).unwrap_or((cut_point + p2) / 2.0);
        // if i1.is_none() || i2.is_none() {
        //     panic!(
        //         r"Can't find intersection for{:?}:
        //     p1({:?}), p1_tangent({:?}),
        //     cut_point({:?}), cut_tangent({:?}),
        //     p2({:?}), p2_tangent({:?})
        //     ",
        //         cubic, p1, p1_tangent, cut_point, cut_tangent, p2, p2_tangent
        //     );
        // }
        return vec![[p1, i1, cut_point], [cut_point, i2, p2]];
    };

    vec![[p1, i1, cut_point], [cut_point, i2, p2]]
}

#[cfg(test)]
mod test {
    use super::*;
    use glam::dvec3;

    #[test]
    fn test_trim_quad_bezier() {
        // 测试正常参数顺序 (a < b)
        let bezier = [
            dvec3(0.0, 0.0, 0.0),
            dvec3(1.0, 2.0, 0.0),
            dvec3(2.0, 0.0, 0.0),
        ];
        let trimmed = trim_quad_bezier(&bezier, 0.25, 0.75);

        // 验证结果点在曲线上
        let start_point = quad_bezier_eval(&bezier, 0.25);
        let end_point = quad_bezier_eval(&bezier, 0.75);
        assert!((trimmed[0] - start_point).length() < f64::EPSILON);
        assert!((trimmed[2] - end_point).length() < f64::EPSILON);

        // 测试参数顺序颠倒 (a > b)
        let trimmed_reversed = trim_quad_bezier(&bezier, 0.75, 0.25);
        assert!((trimmed_reversed[0] - start_point).length() < f64::EPSILON);
        assert!((trimmed_reversed[2] - end_point).length() < f64::EPSILON);

        // 测试边界值
        let full_curve = trim_quad_bezier(&bezier, 0.0, 1.0);
        assert!((full_curve[0] - bezier[0]).length() < f64::EPSILON);
        assert!((full_curve[2] - bezier[2]).length() < f64::EPSILON);

        // 测试零长度曲线
        let zero_length = trim_quad_bezier(&bezier, 0.5, 0.5);
        let mid_point = quad_bezier_eval(&bezier, 0.5);
        assert!((zero_length[0] - mid_point).length() < f64::EPSILON);
        assert!((zero_length[2] - mid_point).length() < f64::EPSILON);

        // 测试复杂曲线
        let complex_bezier = [
            dvec3(-1.0, 0.0, 1.0),
            dvec3(1.0, 3.0, 0.0),
            dvec3(3.0, 0.0, -1.0),
        ];
        let trimmed_complex = trim_quad_bezier(&complex_bezier, 0.2, 0.8);
        let complex_start = quad_bezier_eval(&complex_bezier, 0.2);
        let complex_end = quad_bezier_eval(&complex_bezier, 0.8);
        assert!((trimmed_complex[0] - complex_start).length() < f64::EPSILON);
        assert!((trimmed_complex[2] - complex_end).length() < f64::EPSILON);
    }
}
