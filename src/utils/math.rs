use glam::{DVec2, DVec3, IVec2, dvec2};

pub fn cross2d(a: DVec2, b: DVec2) -> f64 {
    a.x * b.y - b.x * a.y
}

pub fn intersection(p1: DVec3, v1: DVec3, p2: DVec3, v2: DVec3) -> Option<DVec3> {
    // println!("p1: {:?}, v1: {:?}, p2: {:?}, v2: {:?}", p1, v1, p2, v2);
    let cross = v1.cross(v2);
    let denom = cross.length_squared();
    if denom < f64::EPSILON {
        return None;
    }

    let diff = p2 - p1;
    let t = (diff).cross(v2).dot(cross) / denom;
    let s = (diff).cross(v1).dot(cross) / denom;

    let point1 = p1 + v1 * t;
    let point2 = p2 + v2 * s;

    if (point1 - point2).length_squared() < f64::EPSILON {
        Some(point1)
    } else {
        None
    }
}

/// A rectangle in 2D space
#[derive(Debug, Clone, Copy)]
pub struct Rect {
    min: DVec2,
    max: DVec2,
}

impl Rect {
    pub fn union(&self, other: &Self) -> Self {
        let min = self.min.min(other.min);
        let max = self.max.max(other.max);
        Self { min, max }
    }
    pub fn intersection(&self, other: &Self) -> Self {
        let min = self.min.max(other.min);
        let max = self.max.min(other.max);
        Self { min, max }
    }

    pub fn center(&self) -> DVec2 {
        (self.min + self.max) / 2.0
    }

    /// Get the point of the rectangle
    /// ```text
    /// (-1,-1)-----(0,-1)-----(1,-1)
    ///    |          |          |
    /// (-1, 0)-----(0, 0)-----(1, 0)
    ///    |          |          |
    /// (-1, 1)-----(0, 1)-----(1, 1)
    /// ```
    pub fn point(&self, edge: IVec2) -> DVec2 {
        let min = self.min;
        let center = self.center();
        let max = self.max;

        let x = match edge.x {
            -1 => min.y,
            0 => center.y,
            1 => max.y,
            _ => unreachable!(),
        };
        let y = match edge.y {
            -1 => min.y,
            0 => center.y,
            1 => max.y,
            _ => unreachable!(),
        };

        dvec2(x, y)
    }
}

/// Interpolate between two integers
///
/// return integer and the sub progress to the next integer
pub fn interpolate_usize(a: usize, b: usize, t: f64) -> (usize, f64) {
    assert!(b >= a);
    let t = t.clamp(0.0, 1.0);
    let v = b - a;

    let p = v as f64 * t;

    (a + p.floor() as usize, p.fract())
}

#[cfg(test)]
mod test {
    use core::f64;

    use super::*;

    #[test]
    fn test_interpolate_usize() {
        let test = |(x, t): (usize, f64), (expected_x, expected_t): (usize, f64)| {
            assert_eq!(x, expected_x);
            assert!((t - expected_t).abs() < f64::EPSILON);
        };

        test(interpolate_usize(0, 10, 0.0), (0, 0.0));
        test(interpolate_usize(0, 10, 0.5), (5, 0.0));
        test(interpolate_usize(0, 10, 1.0), (10, 0.0));

        test(interpolate_usize(0, 1, 0.0), (0, 0.0));
        test(interpolate_usize(0, 1, 0.5), (0, 0.5));
        test(interpolate_usize(0, 1, 1.0), (1, 0.0));

        test(interpolate_usize(0, 2, 0.0), (0, 0.0));
        test(interpolate_usize(0, 2, 0.2), (0, 0.4));
        test(interpolate_usize(0, 2, 0.4), (0, 0.8));
        test(interpolate_usize(0, 2, 0.6), (1, 0.2));
        test(interpolate_usize(0, 2, 0.8), (1, 0.6));
        test(interpolate_usize(0, 2, 1.0), (2, 0.0));
    }

    #[test]
    fn test_intersection() {
        use glam::dvec3;

        // 1. 垂直相交
        let p1 = dvec3(0.0, 0.0, 0.0);
        let v1 = dvec3(1.0, 0.0, 0.0);
        let p2 = dvec3(0.0, 1.0, 0.0);
        let v2 = dvec3(0.0, -1.0, 0.0);
        assert_eq!(intersection(p1, v1, p2, v2), Some(dvec3(0.0, 0.0, 0.0)));

        // 2. 斜交
        let p1 = dvec3(1.0, 1.0, 0.0);
        let v1 = dvec3(1.0, 2.0, 0.0);
        let p2 = dvec3(3.0, 1.0, 0.0);
        let v2 = dvec3(-1.0, 2.0, 0.0);
        assert_eq!(intersection(p1, v1, p2, v2), Some(dvec3(2.0, 3.0, 0.0)));

        // 3. 重合直线（应返回 None）
        let p1 = dvec3(0.0, 0.0, 0.0);
        let v1 = dvec3(1.0, 1.0, 1.0);
        let p2 = dvec3(1.0, 1.0, 1.0);
        let v2 = dvec3(2.0, 2.0, 2.0);
        assert!(intersection(p1, v1, p2, v2).is_none());

        // 4. 平行直线（应返回 None）
        let p1 = dvec3(0.0, 0.0, 0.0);
        let v1 = dvec3(1.0, 1.0, 0.0);
        let p2 = dvec3(1.0, 0.0, 0.0);
        let v2 = dvec3(1.0, 1.0, 0.0);
        assert!(intersection(p1, v1, p2, v2).is_none());

        // 5. 异面直线（应返回 None）
        let p1 = dvec3(0.0, 0.0, 0.0);
        let v1 = dvec3(1.0, 0.0, 1.0);
        let p2 = dvec3(0.0, 1.0, 0.0);
        let v2 = dvec3(1.0, 0.0, -1.0);
        assert!(intersection(p1, v1, p2, v2).is_none());
    }
}
