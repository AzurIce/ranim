use glam::{DAffine3, DVec3};

use crate::traits::{AabbPoint, Discard, Locate};

/// A trait for affine transformations (linear transformation + shifting).
pub trait AffineTransform {
    /// Applies a linear transformation to the object.
    fn affine_transform_at_point(&mut self, mat: DAffine3, origin: DVec3) -> &mut Self;
}

impl AffineTransform for DVec3 {
    fn affine_transform_at_point(&mut self, mat: DAffine3, origin: DVec3) -> &mut Self {
        *self = mat.transform_point3(*self - origin) + origin;
        if self.x.abs() < 1e-10 {
            self.x = 0.0;
        }
        if self.y.abs() < 1e-10 {
            self.y = 0.0;
        }
        if self.z.abs() < 1e-10 {
            self.z = 0.0;
        }
        self
    }
}

impl<T: AffineTransformExt> AffineTransform for [T] {
    fn affine_transform_at_point(&mut self, mat: DAffine3, origin: DVec3) -> &mut Self {
        self.iter_mut()
            .for_each(|p| p.affine_transform_at_point(mat, origin).discard());
        self
    }
}

/// Useful extensions for linear transformations.
///
/// This trait is implemented automatically for types that implement [`AffineTransform`], you should not implement it yourself.
pub trait AffineTransformExt: AffineTransform {
    /// Applies a linear transformation to the object at the given anchor.
    fn affine_transform_at(&mut self, mat: DAffine3, anchor: impl Locate<Self>) -> &mut Self {
        self.affine_transform_at_point(mat, anchor.locate(self));
        self
    }

    /// Applies a linear transformation to the object at the center of its AABB.
    fn affine_transform(&mut self, mat: DAffine3) -> &mut Self
    where
        AabbPoint: Locate<Self>,
    {
        self.affine_transform_at(mat, AabbPoint::CENTER);
        self
    }
}

impl<T: AffineTransform + ?Sized> AffineTransformExt for T {}
