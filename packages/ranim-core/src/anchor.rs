//! Anchor
//!
//! Ranim has an anchor system based on generics, an anchor can be any type `T`,
//! and types that implements [`Locate<T>`] can use [`Locate::locate`] to convert the anchor to a [`DVec3`] point.
//!
//! Ranim provides some built-in anchors and related [`Locate`] implementations:
//! - [`DVec3`]: The point itself in 3d space.
//! - [`Centroid`]: The avg point of all points.
//!   Note that sometime the center of Aabb is not the centroid.
//!   (0, 0, 0) is the center point.
//! - [`AabbPoint`]: A point based on [`Aabb`]'s size, the number in each axis means the fraction of the size of the [`Aabb`].

use glam::DVec3;
use tracing::warn;

/// Locate a point.
pub trait Locate<T: ?Sized> {
    /// Locate self on the target
    fn locate(&self, target: &T) -> DVec3;
}

impl<T: ?Sized> Locate<T> for DVec3 {
    fn locate(&self, _target: &T) -> DVec3 {
        *self
    }
}

/// The centroid.
///
/// Avg of all points.
pub struct Centroid;

impl Locate<DVec3> for Centroid {
    fn locate(&self, target: &DVec3) -> DVec3 {
        *target
    }
}

impl<T> Locate<[T]> for Centroid
where
    Centroid: Locate<T>,
{
    fn locate(&self, target: &[T]) -> DVec3 {
        target.iter().map(|x| self.locate(x)).sum::<DVec3>() / target.len() as f64
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
    // /// Left point (-X)
    // pub const LEFT: Self = Self(DVec3::NEG_X);
    // /// Right point (+X)
    // pub const RIGHT: Self = Self(DVec3::X);
    // /// Top point (+Y)
    // pub const TOP: Self = Self(DVec3::Y);
    // /// Bottom point (-Y)
    // pub const BOTTOM: Self = Self(DVec3::NEG_Y);
    // /// Top Right point (+X, +Y)
    // pub const TOP_RIGHT: Self = Self(dvec3(1.0, 1.0, 0.0));
    // /// Top Left point (-X, +Y)
    // pub const TOP_LEFT: Self = Self(dvec3(-1.0, 1.0, 0.0));
    // /// Bottom Right point (+X, -Y)
    // pub const BOTTOM_RIGHT: Self = Self(dvec3(1.0, -1.0, 0.0));
    // /// Bottom Left point (-X, -Y)
    // pub const BOTTOM_LEFT: Self = Self(dvec3(-1.0, -1.0, 0.0));
}

impl<T: Aabb + ?Sized> Locate<T> for AabbPoint {
    fn locate(&self, target: &T) -> DVec3 {
        let center = target.aabb_center();
        let half_size = target.aabb_size() / 2.0;
        center + self.0 * half_size
    }
}

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
