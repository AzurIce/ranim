use glam::DVec3;

use crate::traits::Discard;

/// Rotating operations.
///
/// This trait is automatically implemented for [`DVec3`] and `[T]` where `T: RotateTransform`.
pub trait RotateTransform {
    /// Rotate the item by a given angle about a given axis.
    fn rotate_on_axis(&mut self, axis: DVec3, angle: f64) -> &mut Self;
    /// Rotate the item by a given angle about the X axis.
    fn rotate_on_x(&mut self, angle: f64) -> &mut Self {
        self.rotate_on_axis(DVec3::X, angle)
    }
    /// Rotate the item by a given angle about the Y axis.
    fn rotate_on_y(&mut self, angle: f64) -> &mut Self {
        self.rotate_on_axis(DVec3::Y, angle)
    }
    /// Rotate the item by a given angle about the Z axis.
    fn rotate_on_z(&mut self, angle: f64) -> &mut Self {
        self.rotate_on_axis(DVec3::Z, angle)
    }
}

impl RotateTransform for DVec3 {
    fn rotate_on_axis(&mut self, axis: DVec3, angle: f64) -> &mut Self {
        *self = DVec3::rotate_axis(*self, axis, angle);
        self
    }
}

impl<T: RotateTransform> RotateTransform for [T] {
    fn rotate_on_axis(&mut self, axis: DVec3, angle: f64) -> &mut Self {
        self.iter_mut()
            .for_each(|x| x.rotate_on_axis(axis, angle).discard());
        self
    }
}

impl<T: RotateTransform> RotateTransform for Vec<T> {
    fn rotate_on_axis(&mut self, axis: DVec3, angle: f64) -> &mut Self {
        self.as_mut_slice().rotate_on_axis(axis, angle);
        self
    }
}
