//! Shared metadata for render-world entities.

use bevy_ecs::prelude::Component;

/// Global insertion order of a rendered item in the evaluated frame.
///
/// Extraction systems copy [`ranim_core::frame::FrameItem::order`] so coplanar
/// geometry resolves consistently across all typed input batches. Multiple
/// render entities extracted from the same input item may share this order.
///
/// The renderer bounds depth bias using the complete
/// [`ranim_core::frame::EvaluatedFrame::len`], including inputs whose types are
/// not extracted by any installed plugin.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct SceneOrder(
    /// Zero-based insertion order in the complete evaluated frame.
    pub usize,
);
