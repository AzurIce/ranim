use glam::{DMat3, DVec2, DVec3, IVec2, dvec2};

/// Double-precision 3D coordinate system.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DCoordSystem3 {
    /// World-space origin of the local coordinate system.
    pub origin: DVec3,
    /// Basis columns that map local vectors to world vectors.
    pub basis: DMat3,
}

impl DCoordSystem3 {
    /// World coordinate system.
    pub const WORLD: Self = Self {
        origin: DVec3::ZERO,
        basis: DMat3::IDENTITY,
    };

    /// Creates a coordinate system from an origin and a basis matrix.
    pub fn new(origin: DVec3, basis: DMat3) -> Self {
        Self { origin, basis }
    }

    /// Creates a coordinate system from origin and basis columns.
    pub fn from_basis(origin: DVec3, x: DVec3, y: DVec3, z: DVec3) -> Self {
        Self {
            origin,
            basis: DMat3::from_cols(x, y, z),
        }
    }

    /// Converts a local-space point to world space.
    pub fn local_to_world_point(self, point: DVec3) -> DVec3 {
        self.origin + self.basis * point
    }

    /// Converts a world-space point to local space.
    pub fn world_to_local_point(self, point: DVec3) -> DVec3 {
        self.basis.inverse() * (point - self.origin)
    }

    /// Converts a local-space vector to world space.
    pub fn local_to_world_vector(self, vector: DVec3) -> DVec3 {
        self.basis * vector
    }

    /// Converts a world-space vector to local space.
    pub fn world_to_local_vector(self, vector: DVec3) -> DVec3 {
        self.basis.inverse() * vector
    }
}

impl Default for DCoordSystem3 {
    fn default() -> Self {
        Self::WORLD
    }
}

/// Double-precision 3D axis-aligned range in a coordinate system.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DRange3 {
    /// Minimum corner.
    pub min: DVec3,
    /// Maximum corner.
    pub max: DVec3,
}

impl DRange3 {
    /// Zero-sized range at the origin.
    pub const ZERO: Self = Self {
        min: DVec3::ZERO,
        max: DVec3::ZERO,
    };

    /// Creates a range from any two corners, ordering them as min/max.
    pub fn new(a: DVec3, b: DVec3) -> Self {
        Self {
            min: a.min(b),
            max: a.max(b),
        }
    }

    /// Creates a range from explicit min/max corners.
    pub fn from_min_max(min: DVec3, max: DVec3) -> Self {
        Self::new(min, max)
    }

    /// Creates a zero-sized range at a point.
    pub fn point(point: DVec3) -> Self {
        Self {
            min: point,
            max: point,
        }
    }

    /// Returns the range size.
    pub fn size(self) -> DVec3 {
        self.max - self.min
    }

    /// Returns the range center.
    pub fn center(self) -> DVec3 {
        (self.min + self.max) / 2.0
    }

    /// Returns whether the range has zero extent on every axis.
    pub fn is_zero_size(self) -> bool {
        self.min == self.max
    }

    /// Locates a normalized anchor inside the range.
    pub fn locate(self, anchor: crate::anchor::BoundsAnchor) -> DVec3 {
        self.center() + anchor.0 * self.size() / 2.0
    }

    /// Returns a translated range.
    pub fn translated(self, offset: DVec3) -> Self {
        Self {
            min: self.min + offset,
            max: self.max + offset,
        }
    }

    /// Returns the union of two ranges.
    pub fn union(self, other: Self) -> Self {
        Self {
            min: self.min.min(other.min),
            max: self.max.max(other.max),
        }
    }

    /// Returns the range as `[min, max]`.
    pub fn as_array(self) -> [DVec3; 2] {
        [self.min, self.max]
    }

    /// Returns all eight corners of the range.
    pub fn corners(self) -> [DVec3; 8] {
        let min = self.min;
        let max = self.max;
        [
            DVec3::new(min.x, min.y, min.z),
            DVec3::new(max.x, min.y, min.z),
            DVec3::new(min.x, max.y, min.z),
            DVec3::new(max.x, max.y, min.z),
            DVec3::new(min.x, min.y, max.z),
            DVec3::new(max.x, min.y, max.z),
            DVec3::new(min.x, max.y, max.z),
            DVec3::new(max.x, max.y, max.z),
        ]
    }
}

impl From<[DVec3; 2]> for DRange3 {
    fn from(value: [DVec3; 2]) -> Self {
        Self::new(value[0], value[1])
    }
}

impl From<DRange3> for [DVec3; 2] {
    fn from(value: DRange3) -> Self {
        value.as_array()
    }
}

impl Default for DRange3 {
    fn default() -> Self {
        Self::ZERO
    }
}

/// Double-precision 3D bounds: a coordinate system plus a local range.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DBounds3 {
    /// Coordinate system in which `range` is expressed.
    coord_system: DCoordSystem3,
    /// Local range in `coord_system`.
    range: DRange3,
}

impl DBounds3 {
    /// Zero-sized world-space bounds at the origin.
    pub const ZERO: Self = Self {
        coord_system: DCoordSystem3::WORLD,
        range: DRange3::ZERO,
    };

    /// Creates world-space bounds from any two corners, ordering them as min/max.
    pub fn new(a: DVec3, b: DVec3) -> Self {
        Self::from_range(DRange3::new(a, b))
    }

    /// Creates world-space bounds from explicit min/max corners.
    pub fn from_min_max(min: DVec3, max: DVec3) -> Self {
        Self::new(min, max)
    }

    /// Creates world-space bounds from a range.
    pub fn from_range(range: DRange3) -> Self {
        Self {
            coord_system: DCoordSystem3::WORLD,
            range,
        }
    }

    /// Creates bounds from a coordinate system and a local range.
    pub fn in_coord_system(coord_system: DCoordSystem3, range: DRange3) -> Self {
        Self {
            coord_system,
            range,
        }
    }

    /// Creates zero-sized world-space bounds at a point.
    pub fn point(point: DVec3) -> Self {
        Self::from_range(DRange3::point(point))
    }

    /// Returns the coordinate system.
    pub fn coord_system(self) -> DCoordSystem3 {
        self.coord_system
    }

    /// Returns the local range.
    pub fn local_range(self) -> DRange3 {
        self.range
    }

    /// Returns the local minimum corner.
    pub fn local_min(self) -> DVec3 {
        self.range.min
    }

    /// Returns the local maximum corner.
    pub fn local_max(self) -> DVec3 {
        self.range.max
    }

    /// Returns the local bounds size.
    pub fn size(self) -> DVec3 {
        self.range.size()
    }

    /// Returns the world-space center point.
    pub fn center(self) -> DVec3 {
        self.coord_system.local_to_world_point(self.range.center())
    }

    /// Locates a normalized anchor inside the bounds and returns a world-space point.
    pub fn locate(self, anchor: crate::anchor::BoundsAnchor) -> DVec3 {
        self.coord_system
            .local_to_world_point(self.range.locate(anchor))
    }

    /// Returns bounds translated by a world-space offset.
    pub fn translated(self, offset: DVec3) -> Self {
        Self {
            coord_system: DCoordSystem3 {
                origin: self.coord_system.origin + offset,
                ..self.coord_system
            },
            range: self.range,
        }
    }

    /// Returns a world-space axis-aligned range covering these bounds.
    pub fn world_range(self) -> DRange3 {
        self.range
            .corners()
            .into_iter()
            .map(|point| DRange3::point(self.coord_system.local_to_world_point(point)))
            .reduce(DRange3::union)
            .unwrap_or(DRange3::ZERO)
    }

    /// Returns the minimum corner of the world-space axis-aligned range.
    pub fn world_min(self) -> DVec3 {
        self.world_range().min
    }

    /// Returns the maximum corner of the world-space axis-aligned range.
    pub fn world_max(self) -> DVec3 {
        self.world_range().max
    }

    /// Returns the size of the world-space axis-aligned range.
    pub fn world_size(self) -> DVec3 {
        self.world_range().size()
    }

    /// Returns whether the local range has zero extent on every axis.
    pub fn is_zero_size(self) -> bool {
        self.range.is_zero_size()
    }

    /// Returns the union of two bounds as world-space bounds.
    pub fn union(self, other: Self) -> Self {
        Self::from_range(self.world_range().union(other.world_range()))
    }

    /// Returns the world-space range as `[min, max]`.
    pub fn as_array(self) -> [DVec3; 2] {
        self.world_range().as_array()
    }
}

impl From<[DVec3; 2]> for DBounds3 {
    fn from(value: [DVec3; 2]) -> Self {
        Self::new(value[0], value[1])
    }
}

impl From<DRange3> for DBounds3 {
    fn from(value: DRange3) -> Self {
        Self::from_range(value)
    }
}

impl From<DBounds3> for [DVec3; 2] {
    fn from(value: DBounds3) -> Self {
        value.as_array()
    }
}

impl Default for DBounds3 {
    fn default() -> Self {
        Self::ZERO
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::anchor::BoundsAnchor;
    use glam::dvec3;

    #[test]
    fn coord_system_round_trips_points_and_vectors() {
        let coord_system = DCoordSystem3::from_basis(
            dvec3(10.0, -2.0, 1.0),
            dvec3(0.0, 2.0, 0.0),
            dvec3(-3.0, 0.0, 0.0),
            dvec3(0.0, 0.0, 4.0),
        );

        let local_point = dvec3(1.5, 2.0, -0.25);
        let world_point = coord_system.local_to_world_point(local_point);
        assert!((coord_system.world_to_local_point(world_point) - local_point).length() < 1.0e-9);

        let local_vector = dvec3(-2.0, 0.5, 3.0);
        let world_vector = coord_system.local_to_world_vector(local_vector);
        assert!(
            (coord_system.world_to_local_vector(world_vector) - local_vector).length() < 1.0e-9
        );
    }

    #[test]
    fn bounds_keep_local_size_and_expose_world_range() {
        let coord_system =
            DCoordSystem3::from_basis(dvec3(10.0, 0.0, 0.0), DVec3::Y, DVec3::NEG_X, DVec3::Z);
        let bounds = DBounds3::in_coord_system(
            coord_system,
            DRange3::new(dvec3(0.0, 0.0, -1.0), dvec3(2.0, 4.0, 1.0)),
        );

        assert_eq!(bounds.size(), dvec3(2.0, 4.0, 2.0));
        assert_eq!(bounds.center(), dvec3(8.0, 1.0, 0.0));

        let world_range = bounds.world_range();
        assert_eq!(world_range.min, dvec3(6.0, 0.0, -1.0));
        assert_eq!(world_range.max, dvec3(10.0, 2.0, 1.0));
        assert_eq!(bounds.world_size(), dvec3(4.0, 2.0, 2.0));
    }

    #[test]
    fn bounds_anchor_uses_local_range_and_returns_world_point() {
        let coord_system =
            DCoordSystem3::from_basis(dvec3(1.0, 2.0, 3.0), DVec3::Y, DVec3::Z, DVec3::X);
        let bounds = DBounds3::in_coord_system(
            coord_system,
            DRange3::new(dvec3(-1.0, -2.0, -3.0), dvec3(1.0, 2.0, 3.0)),
        );

        assert_eq!(
            bounds.locate(BoundsAnchor::MAX),
            coord_system.local_to_world_point(dvec3(1.0, 2.0, 3.0))
        );
        assert_eq!(bounds.locate(BoundsAnchor::CENTER), dvec3(1.0, 2.0, 3.0));
    }
}

/// Cross product of 2d points
pub fn cross2d(a: DVec2, b: DVec2) -> f64 {
    a.x * b.y - b.x * a.y
}

/// Get the intersection point of two ray
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
    /// Get the union of two rectangle
    pub fn union(&self, other: &Self) -> Self {
        let min = self.min.min(other.min);
        let max = self.max.max(other.max);
        Self { min, max }
    }
    /// Get the intersection of two rectangle
    pub fn intersection(&self, other: &Self) -> Self {
        let min = self.min.max(other.min);
        let max = self.max.min(other.max);
        Self { min, max }
    }
    /// Get the center of the rectangle
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
