//! Static semantic resize operations.
//!
//! Resize is intentionally separate from editor resize requests. It is a typed
//! programmatic API for setting an item's user-facing semantic size.
//!
//! `Resize` is derived from the same semantic model as [`super::Scale`] and
//! [`SemanticBounds`]: it changes the size represented by semantic bounds, not
//! render coverage. A circle can resize by changing its diameter, a layout item
//! can resize by changing its frame, and a raw point item can resize by scaling
//! its points.
//!
//! Bounds-aware helpers take an explicit [`DBounds3`] operation bounds. The
//! default trait methods use [`SemanticBounds::semantic_bounds`] as that
//! operation bounds.

use glam::{DVec2, DVec3};

use super::{Scale, ShiftTransform};
use crate::anchor::{BoundsAnchor, DBounds3, Locate, SemanticBounds};

/// Smallest positive extent used by resize helpers to avoid degenerate ratios.
pub const MIN_RESIZE_EXTENT: f64 = 1.0e-9;

/// Static, typed semantic resize API.
///
/// Implementations set the item's semantic size while preserving the requested
/// anchor. The `Size` type is part of the item capability: a uniform-only item
/// can implement `Resize<f64>`, while an item that supports independent axes
/// can implement `Resize<DVec3>` or another explicit size type.
pub trait Resize<Size>: Sized {
    /// Resizes around the item's center anchor.
    fn resize(&mut self, size: Size) -> &mut Self
    where
        Self: SemanticBounds,
    {
        self.resize_about(BoundsAnchor::CENTER, size)
    }

    /// Resizes while keeping the given semantic-bounds anchor fixed.
    ///
    /// This uses [`SemanticBounds::semantic_bounds`] as the default operation
    /// bounds. Use [`Resize::resize_about_bounds`] when the operation should be
    /// based on an explicitly supplied bounds.
    fn resize_about(&mut self, anchor: BoundsAnchor, size: Size) -> &mut Self
    where
        Self: SemanticBounds,
    {
        self.resize_about_bounds(self.semantic_bounds(), anchor, size)
    }

    /// Resizes while using an explicit operation bounds to locate the anchor.
    ///
    /// The supplied `bounds` can be the item's semantic bounds, extracted
    /// primitive bounds, or any custom bounds chosen by the caller.
    fn resize_about_bounds(
        &mut self,
        bounds: DBounds3,
        anchor: BoundsAnchor,
        size: Size,
    ) -> &mut Self;
}

/// Clamps a resize size to a positive non-degenerate extent.
pub fn clamped_resize_size(size: DVec3) -> DVec3 {
    DVec3::new(
        size.x.abs().max(MIN_RESIZE_EXTENT),
        size.y.abs().max(MIN_RESIZE_EXTENT),
        size.z.abs().max(MIN_RESIZE_EXTENT),
    )
}

/// Clamps an XY resize size to a positive non-degenerate extent.
pub fn clamped_resize_size2(size: DVec2) -> DVec2 {
    DVec2::new(
        size.x.abs().max(MIN_RESIZE_EXTENT),
        size.y.abs().max(MIN_RESIZE_EXTENT),
    )
}

/// Calculates per-axis scale from one size to another.
pub fn resize_scale(from: DVec3, to: DVec3) -> DVec3 {
    let from = clamped_resize_size(from);
    let to = clamped_resize_size(to);
    DVec3::new(to.x / from.x, to.y / from.y, to.z / from.z)
}

/// Calculates XY scale from one size to another.
pub fn resize_scale2(from: DVec2, to: DVec2) -> DVec2 {
    let from = clamped_resize_size2(from);
    let to = clamped_resize_size2(to);
    DVec2::new(to.x / from.x, to.y / from.y)
}

/// Runs a resize body while preserving an anchor from explicit operation bounds.
///
/// The anchor position before resizing is located in `bounds`. The anchor
/// position after resizing is located in the item's semantic bounds. This is the
/// intended helper for semantic implementations that set their own size fields
/// directly, such as circles and layout objects.
pub fn resize_preserving_anchor<T>(
    item: &mut T,
    bounds: DBounds3,
    anchor: BoundsAnchor,
    resize_body: impl FnOnce(&mut T),
) where
    T: SemanticBounds + ShiftTransform,
{
    let before = anchor.locate_in(bounds);
    resize_body(item);
    let after = anchor.locate(item);
    item.shift(before - after);
}

/// Resizes an item by scaling around an anchor in explicit operation bounds.
pub fn resize_by_bounds<T>(item: &mut T, bounds: DBounds3, anchor: BoundsAnchor, size: DVec3)
where
    T: ShiftTransform + Scale,
{
    let current_size = clamped_resize_size(bounds.size());
    let scale = resize_scale(current_size, size);
    let origin = anchor.locate_in(bounds);
    item.shift(-origin);
    item.scale(scale);
    item.shift(origin);
}

/// Resizes an item's semantic bounds by scaling around the requested anchor.
pub fn resize_by_semantic_bounds<T>(item: &mut T, anchor: BoundsAnchor, size: DVec3)
where
    T: SemanticBounds + ShiftTransform + Scale,
{
    let bounds = item.semantic_bounds();
    resize_by_bounds(item, bounds, anchor, size);
}

/// Resizes an item in XY by scaling around an anchor in explicit operation
/// bounds.
///
/// Z is not scaled. This is the intended helper for 2D items in Ranim's 3D
/// coordinate space.
pub fn resize_xy_by_bounds<T>(item: &mut T, bounds: DBounds3, anchor: BoundsAnchor, size: DVec2)
where
    T: ShiftTransform + Scale,
{
    let current_size = clamped_resize_size2(bounds.size().truncate());
    let scale = resize_scale2(current_size, size);
    let origin = anchor.locate_in(bounds);
    item.shift(-origin);
    item.scale(DVec3::new(scale.x, scale.y, 1.0));
    item.shift(origin);
}

/// Resizes an item's XY semantic bounds by scaling around the requested anchor.
///
/// Z is not scaled. This is the intended helper for 2D items in Ranim's 3D
/// coordinate space.
pub fn resize_xy_by_semantic_bounds<T>(item: &mut T, anchor: BoundsAnchor, size: DVec2)
where
    T: SemanticBounds + ShiftTransform + Scale,
{
    let bounds = item.semantic_bounds();
    resize_xy_by_bounds(item, bounds, anchor, size);
}
