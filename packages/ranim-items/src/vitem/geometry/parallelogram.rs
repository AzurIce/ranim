use ranim_core::{
    components::vpoint::VPointVec,
    glam::DVec3,
    traits::{Aabb, RotateTransform, ScaleTransform, ShiftTransform, Discard},
};

use crate::vitem::{VItem, VPath};
use crate::vitem::geometry::Polygon;

/// A parallelogram.
#[derive(Debug, Clone, ranim_macros::Interpolatable)]
pub struct Parallelogram {
    /// Origin of the paralleogram
    pub origin: DVec3,
    /// vectors representing two edges of the paralleogram
    pub basis: (DVec3, DVec3),
}

impl VItem<Parallelogram> {
    /// Create a new parallelogram with the given origin and basis vectors.
    pub fn new(origin: DVec3, basis: (DVec3, DVec3)) -> Self {
        Self::new_with(Parallelogram { origin, basis })
    }
}

impl Parallelogram {
    /// Create a new parallelogram with the given origin and basis vectors.
    pub fn new(origin: DVec3, basis: (DVec3, DVec3)) -> Self {
        Self { origin, basis }
    }

    /// Get the vertices of the parallelogram.
    pub fn vertices(&self) -> [DVec3; 4] {
        let &Parallelogram {
            origin,
            basis: (u, v),
        } = self;
        [origin, origin + u, origin + u + v, origin + v]
    }
}

impl Aabb for Parallelogram {
    fn aabb(&self) -> [DVec3; 2] {
        self.vertices().aabb()
    }
}

impl ShiftTransform for Parallelogram {
    fn shift(&mut self, offset: DVec3) -> &mut Self {
        self.origin += offset;
        self
    }
}

impl RotateTransform for Parallelogram {
    fn rotate_on_axis(&mut self, axis: DVec3, angle: f64) -> &mut Self {
        self.origin.rotate_on_axis(axis, angle);
        self.basis.0.rotate_on_axis(axis, angle);
        self.basis.1.rotate_on_axis(axis, angle);
        self
    }
}

impl ScaleTransform for Parallelogram {
    fn scale(&mut self, scale: DVec3) -> &mut Self {
        self.origin.scale(scale).discard();
        self.basis.0 *= scale;
        self.basis.1 *= scale;
        self
    }
}

impl VPath for Parallelogram {
    fn normal(&self) -> DVec3 {
        self.basis.0.cross(self.basis.1).normalize()
    }
    fn build_vpoint_vec(&self) -> VPointVec {
        Polygon::from(self.clone()).build_vpoint_vec()
    }
}

impl From<Parallelogram> for Polygon {
    fn from(value: Parallelogram) -> Self {
        Polygon::new(value.vertices().to_vec())
    }
}
