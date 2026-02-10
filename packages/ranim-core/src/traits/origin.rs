use glam::DVec3;

use crate::{
    proj::{CoordinateSystem, ProjectionPlane},
    traits::Shift,
};

/// Trait for items that have a point on it defining its position.
pub trait Origin: Shift {
    /// Returns the position of the item's origin.
    fn origin(&self) -> DVec3;
    /// Moves the item with its origin to a new position.
    fn move_to(&mut self, origin: DVec3) -> &mut Self {
        let offset = origin - self.origin();
        self.shift(offset)
    }
}

/// An item that has a local coordinate system.
pub trait LocalCoordinate: Origin {
    /// Returns the local coordinate system of the item.
    fn coord(&self) -> CoordinateSystem;
    /// Returns the projection of the local coordinate system.
    fn proj(&self) -> ProjectionPlane {
        self.coord().proj
    }
}

impl Origin for DVec3 {
    fn origin(&self) -> DVec3 {
        *self
    }

    fn move_to(&mut self, origin: DVec3) -> &mut Self {
        *self = origin;
        self
    }
}
