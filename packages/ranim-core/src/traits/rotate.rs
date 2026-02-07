use glam::{DMat3, DVec3};

use crate::{
    anchor::{Locate, Pivot},
    utils::wrap_point_func_with_point,
};

pub trait RotateImpl {
    /// Rotate the item by a given angle about a given axis at the given point.
    ///
    /// See [`Anchor`]
    fn rotate_at_point(&mut self, angle: f64, axis: DVec3, point: DVec3) -> &mut Self;
}

impl RotateImpl for DVec3 {
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

impl<T: Rotate> RotateImpl for [T] {
    fn rotate_at_point(&mut self, angle: f64, axis: DVec3, point: DVec3) -> &mut Self {
        self.iter_mut().for_each(|x| {
            x.rotate_at_point(angle, axis, point);
        });
        self
    }
}

/// A trait for rotating operations
pub trait Rotate: RotateImpl {
    /// Rotate the mobject by a given angle about a given axis at center.
    ///
    /// This is equivalent to [`Rotate::rotate_by_anchor`] with [`Anchor::CENTER`].
    fn rotate(&mut self, angle: f64, axis: DVec3) -> &mut Self
    where
        Self: Locate<Pivot>,
    {
        self.rotate_at(angle, axis, Pivot)
    }
    /// Rotate the item by a given angle about a given axis at anchor.
    ///
    /// See [`Anchor`]
    fn rotate_at<T>(&mut self, angle: f64, axis: DVec3, anchor: T) -> &mut Self
    where
        Self: Locate<T>,
    {
        let point = Locate::<T>::locate(self, anchor);
        RotateImpl::rotate_at_point(self, angle, axis, point)
    }
}

impl<T: RotateImpl + ?Sized> Rotate for T {}
