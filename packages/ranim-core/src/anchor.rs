//! Anchor
//!
//! Ranim has an anchor system based on generics, an anchor can be any type `T`,
//! and types that implements [`Locate<T>`] can use [`Locate::locate`] to convert the anchor to a [`DVec3`] point.
//!
//! Ranim provides some built-in anchors and related [`Locate`] implementations:
//! - [`Pivot`]: Every operation's default pivot point.
//! - [`AabbPoint`]: A point based on [`Aabb`]'s size, the number in each axis means the fraction of the size of the [`Aabb`].
//!   (0, 0, 0) is the center point.

use glam::DVec3;
use tracing::warn;

/// Locate a point.
pub trait Locate<T> {
    fn locate(&self, target: T) -> DVec3;
}

impl<T: ?Sized> Locate<DVec3> for T {
    fn locate(&self, target: DVec3) -> DVec3 {
        target
    }
}

/// A point based on [`Aabb`], the number in each axis means the fraction of the size of the [`Aabb`].
/// (0, 0, 0) is the center point.
/// ```text
///      +Y
///      |
///      |
///      +----- +X
///    /
/// +Z
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AabbPoint(pub DVec3);

impl AabbPoint {
    /// Center point, shorthand of `Anchor(DVec3::ZERO)`.
    pub const CENTER: Self = Self(DVec3::ZERO);
}

impl<T: Aabb + ?Sized> Locate<AabbPoint> for T {
    fn locate(&self, point: AabbPoint) -> DVec3 {
        let center = self.aabb_center();
        let half_size = self.aabb_size() / 2.0;
        center + point.0 * half_size
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Pivot;

// MARK: BoundingBox
/// Axis-Aligned Bounding Box
///
/// This is the basic trait for an item.
pub trait Aabb {
    /// Get the Axis-aligned bounding box represent in `[<min>, <max>]`.
    fn aabb(&self) -> [DVec3; 2];
    /// Get the size of the Aabb.
    fn aabb_size(&self) -> DVec3 {
        let [min, max] = self.aabb();
        max - min
    }
    /// Get the center of the Aabb.
    fn aabb_center(&self) -> DVec3 {
        let [min, max] = self.aabb();
        (max + min) / 2.0
    }
}

impl Aabb for DVec3 {
    fn aabb(&self) -> [DVec3; 2] {
        [*self; 2]
    }
}

impl<T: Aabb> Aabb for [T] {
    fn aabb(&self) -> [DVec3; 2] {
        let [min, max] = self
            .iter()
            .map(|x| x.aabb())
            .reduce(|[acc_min, acc_max], [min, max]| [acc_min.min(min), acc_max.max(max)])
            .unwrap_or([DVec3::ZERO, DVec3::ZERO]);
        if min == max {
            warn!("Empty bounding box, is the slice empty?")
        }
        [min, max]
    }
}
