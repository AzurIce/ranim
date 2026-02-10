use std::ops::Deref;

use glam::{DVec3, dvec3};

use crate::traits::{LocalCoordinate, Origin, Rotate, RotateLocal, Shift, ShiftLocal};

/// The projection of an item.
#[derive(Debug, Clone, Copy, PartialEq, ranim_macros::Interpolatable)]
pub struct ProjectionPlane {
    /// The basis vector in the u direction.
    basis_u: DVec3,
    /// The basis vector in the v direction.
    basis_v: DVec3,
}

impl Default for ProjectionPlane {
    fn default() -> Self {
        Self {
            basis_u: DVec3::X,
            basis_v: DVec3::Y,
        }
    }
}

impl ProjectionPlane {
    /// The basis vector in the u direction.
    pub fn basis_u(&self) -> DVec3 {
        self.basis_u.normalize()
    }
    /// The basis vector in the v direction.
    pub fn basis_v(&self) -> DVec3 {
        self.basis_v.normalize()
    }
    /// The basis vectors
    pub fn basis(&self) -> (DVec3, DVec3) {
        (self.basis_u(), self.basis_v())
    }
    /// The corrected basis vector in the u direction.
    /// This is same as [`ProjectionPlane::basis_u`].
    pub fn corrected_basis_u(&self) -> DVec3 {
        self.basis_u.normalize()
    }
    /// The corrected basis vector in the v direction.
    /// This is recalculated to ensure orthogonality.
    pub fn corrected_basis_v(&self) -> DVec3 {
        let normal = self.basis_u.cross(self.basis_v);
        normal.cross(self.basis_u).normalize()
    }
    /// Rotate the projection.
    pub fn rotate(&mut self, angle: f64, axis: DVec3) {
        self.basis_u = self.basis_u.rotate_axis(axis, angle).normalize();
        self.basis_v = self.basis_v.rotate_axis(axis, angle).normalize();
    }
    /// Get the normal vector of the projection target plane.
    #[inline]
    pub fn normal(&self) -> DVec3 {
        self.basis_u.cross(self.basis_v).normalize()
    }
    /// Convert vector from global to local coordinate system.
    pub fn v2c(&self, p: DVec3) -> DVec3 {
        let (u, v) = self.basis();
        let w = self.normal();
        dvec3(p.dot(u), p.dot(v), p.dot(w))
    }
    /// Convert vector from local to global coordinate system.
    pub fn c2v(&self, c: DVec3) -> DVec3 {
        let (u, v) = self.basis();
        let w = self.normal();
        let c = c.normalize();
        u * c.x + v * c.y + w * c.z
    }
}

/// A local coordinate system
#[derive(Debug, Clone, Copy, Default, PartialEq, ranim_macros::Interpolatable)]
pub struct CoordinateSystem {
    /// The origin of the coordinate system.
    pub origin: DVec3,
    /// The projection of the coordinate system.
    pub proj: ProjectionPlane,
}

impl CoordinateSystem {
    /// Convert point from global to local coordinate system.
    pub fn p2c(&self, p: DVec3) -> DVec3 {
        let origin = self.origin;
        let (u, v) = self.basis();
        let w = self.normal();
        let p = p - origin;
        dvec3(p.dot(u), p.dot(v), p.dot(w))
    }
    /// Convert point from local to global coordinate system.
    pub fn c2p(&self, c: DVec3) -> DVec3 {
        let origin = self.origin;
        let (u, v) = self.basis();
        let w = self.normal();
        let c = c - origin;
        u * c.x + v * c.y + w * c.z + origin
    }
}

impl Deref for CoordinateSystem {
    type Target = ProjectionPlane;
    fn deref(&self) -> &Self::Target {
        &self.proj
    }
}

impl Origin for CoordinateSystem {
    fn origin(&self) -> DVec3 {
        self.origin
    }
}

impl LocalCoordinate for CoordinateSystem {
    fn coord(&self) -> CoordinateSystem {
        *self
    }
}

impl Shift for CoordinateSystem {
    fn shift(&mut self, offset: DVec3) -> &mut Self {
        self.origin += offset;
        self
    }
}

impl ShiftLocal for CoordinateSystem {}

impl Rotate for CoordinateSystem {
    fn rotate_at_point(&mut self, angle: f64, axis: DVec3, point: DVec3) -> &mut Self {
        self.proj.rotate(angle, axis);
        self.origin.rotate_at_point(angle, axis, point);
        self
    }
}

impl RotateLocal for CoordinateSystem {
    fn rotate_local(&mut self, angle: f64) -> &mut Self {
        let axis = self.normal();
        self.proj.rotate(angle, axis);
        self
    }
}

impl From<ProjectionPlane> for CoordinateSystem {
    fn from(proj: ProjectionPlane) -> Self {
        Self {
            origin: DVec3::ZERO,
            proj,
        }
    }
}
