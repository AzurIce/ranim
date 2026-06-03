//! Vector item constructors and helpers.
//!
//! The canonical [`VItem`] type lives in `ranim-core`. This module re-exports it
//! for compatibility with existing `ranim_items::vitem::VItem` imports, while
//! geometry, SVG, text, and Typst modules provide higher-level constructors.

/// Geometry items.
pub mod geometry;
/// SVG item.
pub mod svg;
/// Simple text items.
pub mod text;
/// Typst items.
pub mod typst;

pub use ranim_core::core_item::vitem::{DEFAULT_STROKE_WIDTH, VItem};
