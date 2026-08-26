use glam::DVec3;

/// Rotating operations.
///
/// This trait is blanket-implemented for all `T: ApplyTransform<Rigid>`
/// (see [`super::ApplyTransform`]).
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
