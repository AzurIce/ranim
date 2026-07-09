//! Anchors and bounds.
//!
//! Ranim has an anchor system based on generics: an anchor can be any type `T`,
//! and types that implement [`Locate<T>`] can convert that anchor to a [`DVec3`]
//! point.
//!
//! Ranim provides built-in anchors and bounds:
//! - [`DVec3`]: the point itself in 3D space.
//! - [`Centroid`]: the average point of all points.
//! - [`BoundsAnchor`]: a normalized point inside semantic bounds.
//! - [`DRange3`]: an axis-aligned local range.
//! - [`DCoordSystem3`]: a local coordinate system in world space.
//! - [`DBounds3`]: a coordinate system plus a local semantic range.

use glam::DVec3;
use tracing::warn;

pub use crate::utils::math::{DBounds3, DCoordSystem3, DRange3};

/// Locate a point.
pub trait Locate<T: ?Sized> {
    /// Locate self on the target.
    fn locate(&self, target: &T) -> DVec3;
}

impl<T: ?Sized> Locate<T> for DVec3 {
    fn locate(&self, _target: &T) -> DVec3 {
        *self
    }
}

/// The centroid.
///
/// Average of all points.
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

/// A normalized point inside bounds.
///
/// `(-1, -1, -1)` is the minimum corner, `(0, 0, 0)` is the center, and
/// `(1, 1, 1)` is the maximum corner.
///
/// ```text
///      +Y
///      |
///      |
///      +----- +X
///    /
/// +Z
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BoundsAnchor(pub DVec3);

impl BoundsAnchor {
    /// Center anchor.
    pub const CENTER: Self = Self(DVec3::ZERO);
    /// Minimum corner anchor.
    pub const MIN: Self = Self(DVec3::NEG_ONE);
    /// Maximum corner anchor.
    pub const MAX: Self = Self(DVec3::ONE);

    /// Locates this anchor inside explicit bounds.
    pub fn locate_in(self, bounds: DBounds3) -> DVec3 {
        bounds.locate(self)
    }
}

impl<T: SemanticBounds + ?Sized> Locate<T> for BoundsAnchor {
    fn locate(&self, target: &T) -> DVec3 {
        target.semantic_bounds().locate(*self)
    }
}

/// User-facing default operation bounds for anchors, alignment and scale-to-size.
///
/// Semantic bounds describe the box a user/API operation should treat as the
/// item's size. They are not render coverage. If the caller wants the bounds of
/// rendered primitives, they should extract the item to [`crate::core_item::CoreItem`]
/// and query those core items' semantic bounds.
///
/// This is the default bounds used by convenience APIs. Bounds-aware operations
/// can also take an explicit [`DBounds3`] when the caller wants to use extracted
/// primitive bounds or any custom reference bounds instead.
pub trait SemanticBounds {
    /// Get the semantic bounds.
    fn semantic_bounds(&self) -> DBounds3;

    /// Get the size of the semantic bounds.
    fn semantic_bounds_size(&self) -> DVec3 {
        self.semantic_bounds().size()
    }

    /// Get the center of the semantic bounds.
    fn semantic_bounds_center(&self) -> DVec3 {
        self.semantic_bounds().center()
    }
}

impl SemanticBounds for DVec3 {
    fn semantic_bounds(&self) -> DBounds3 {
        DBounds3::point(*self)
    }
}

impl SemanticBounds for DBounds3 {
    fn semantic_bounds(&self) -> DBounds3 {
        *self
    }
}

impl<T: SemanticBounds> SemanticBounds for [T] {
    fn semantic_bounds(&self) -> DBounds3 {
        let bounds = self
            .iter()
            .map(|x| x.semantic_bounds())
            .reduce(DBounds3::union)
            .unwrap_or(DBounds3::ZERO);
        if bounds.is_zero_size() {
            warn!("Empty semantic bounds, is the slice empty?")
        }
        bounds
    }
}

impl<T: SemanticBounds> SemanticBounds for Vec<T> {
    fn semantic_bounds(&self) -> DBounds3 {
        self.as_slice().semantic_bounds()
    }
}
