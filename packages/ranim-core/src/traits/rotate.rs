use glam::{DMat3, DVec3};

use crate::{
    anchor::{AabbPoint, Locate},
    utils::wrap_point_func_with_point,
};

/// Rotating operations.
///
/// This trait is automatically implemented for [`DVec3`] and `[T]` where `T: Rotate`.
pub trait Rotate {
    /// Rotate the item by a given angle about a given axis at the given point.
    ///
    /// See [`Anchor`]
    fn rotate_at_point(&mut self, angle: f64, axis: DVec3, point: DVec3) -> &mut Self;
}

impl Rotate for DVec3 {
    fn rotate_at_point(&mut self, angle: f64, axis: DVec3, point: DVec3) -> &mut Self {
        let rotation = DMat3::from_axis_angle(axis, angle);
        wrap_point_func_with_point(|p| *p = rotation * *p, point)(self);
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

impl<T: RotateExt> Rotate for [T] {
    fn rotate_at_point(&mut self, angle: f64, axis: DVec3, point: DVec3) -> &mut Self {
        self.iter_mut().for_each(|x| {
            x.rotate_at_point(angle, axis, point);
        });
        self
    }
}

/// A trait for rotating operations
///
/// This trait is implemented automatically for types that implement [`RotateImpl`], you should not implement it yourself.
pub trait RotateExt: Rotate {
    /// Rotate the mobject by a given angle about a given axis at center.
    ///
    /// This is equivalent to [`Rotate::rotate_by_anchor`] with [`AabbPoint::CENTER`].
    fn rotate(&mut self, angle: f64, axis: DVec3) -> &mut Self
    where
        AabbPoint: Locate<Self>,
    {
        self.rotate_at(angle, axis, AabbPoint::CENTER)
    }
    /// Rotate the item by a given angle about a given axis at anchor.
    ///
    /// See [`Anchor`]
    fn rotate_at<T>(&mut self, angle: f64, axis: DVec3, anchor: T) -> &mut Self
    where
        T: Locate<Self>,
    {
        let point = anchor.locate(self);
        Rotate::rotate_at_point(self, angle, axis, point)
    }
}

impl<T: Rotate + ?Sized> RotateExt for T {}
