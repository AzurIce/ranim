//! Allocator-generic render items for the arena extraction path.
//!
//! `VItem` mirrors [`crate::core_item::vitem::VItem`] with allocator-owned
//! `Vec`s, so a frame can be built directly into an arena (`Vec<T, &Bump>`
//! through std `allocator_api`) instead of the global heap. The default
//! `std::alloc::Global` parameter keeps existing `VItem` uses unchanged;
//! enabling the `arena` feature gates this module on nightly's
//! `allocator_api`.

use std::alloc::{Allocator, Global};

use glam::{Mat4, Vec3, Vec4};

use crate::components::{rgba::Rgba, width::Width};

/// A render-ready VItem whose vectors live in any [`Allocator`].
///
/// Cloning requires the allocator itself to be `Clone` (e.g. `&Bump` or
/// `std::alloc::Global`).
#[derive(Clone)]
pub struct VItem<A: Allocator = Global> {
    /// The normal vector of the projection target plane in local space.
    /// If `None`, the normal will be derived from the points at render time.
    pub normal: Option<Vec3>,
    /// The points of the item in local space `(x, y, z, is_closed)`.
    pub points: Vec<Vec4, A>,
    /// The local-to-world transform applied when flattening onto the plane.
    pub transform: Mat4,
    /// Fill rgbas.
    pub fill_rgbas: Vec<Rgba, A>,
    /// Stroke rgbas.
    pub stroke_rgbas: Vec<Rgba, A>,
    /// Stroke widths.
    pub stroke_widths: Vec<Width, A>,
}
