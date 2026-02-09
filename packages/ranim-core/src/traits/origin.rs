use glam::DVec3;

use crate::traits::Shift;

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
