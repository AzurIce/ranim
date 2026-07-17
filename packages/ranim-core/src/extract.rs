//! Query-based extraction protocols from the main world to the render world.
//!
//! The traits here are renderer-neutral: they only describe which main-world
//! components an extractor reads and what bundle it writes into the render
//! world. The systems, schedule and registration entry points that execute
//! extraction live in `ranim-render`.

use bevy_ecs::{
    bundle::Bundle,
    query::{QueryFilter, QueryItem, ReadOnlyQueryData},
};

use crate::core_item::{camera_frame::CameraFrame, mesh_item::MeshItem, vitem::VItem};

/// Reusable output buffer for query-based 1:N extraction.
pub struct ExtractOutput<B: Bundle> {
    items: Vec<(usize, B)>,
}

impl<B: Bundle> Default for ExtractOutput<B> {
    fn default() -> Self {
        Self { items: Vec::new() }
    }
}

impl<B: Bundle> ExtractOutput<B> {
    /// Emit a render bundle with an explicit stable part key.
    pub fn emit(&mut self, part: usize, bundle: B) {
        self.items.push((part, bundle));
    }

    /// Emit a render bundle using the next sequential part key.
    pub fn push(&mut self, bundle: B) {
        self.emit(self.items.len(), bundle);
    }

    /// Drain the emitted items. Used by the extraction systems in `ranim-render`.
    #[doc(hidden)]
    pub fn drain(&mut self) -> impl Iterator<Item = (usize, B)> + '_ {
        self.items.drain(..)
    }

    /// Clear the buffer for reuse. Used by the extraction systems in `ranim-render`.
    #[doc(hidden)]
    pub fn clear(&mut self) {
        self.items.clear();
    }
}

/// Extract one render bundle from arbitrary read-only main-world query data.
///
/// The output bundle is inserted directly onto the render root entity that
/// represents the matched main-world entity — no extra primitive entity is
/// created for 1:1 extraction. Returning `None` removes the previously
/// extracted output from the render root.
pub trait ExtractComponent: Send + Sync + 'static {
    /// Main-world data read by this extractor.
    type QueryData: ReadOnlyQueryData;
    /// Additional filter applied to the source entity.
    type QueryFilter: QueryFilter;
    /// Components inserted into the render root entity.
    type Out: Bundle;

    /// Return the current render bundle, or `None` to remove the previous output.
    fn extract_component(item: QueryItem<'_, '_, Self::QueryData>) -> Option<Self::Out>;
}

/// Extract zero or more render bundles from arbitrary read-only query data.
///
/// Each emitted bundle becomes (or updates in place) an independent primitive
/// entity owned by the render root, keyed by its explicit part key.
pub trait ExtractMany: Send + Sync + 'static {
    /// Main-world data read by this extractor.
    type QueryData: ReadOnlyQueryData;
    /// Additional filter applied to the source entity.
    type QueryFilter: QueryFilter;
    /// Components inserted into each extracted primitive entity.
    type Out: Bundle;

    /// Emit the current render bundles into the reusable output buffer.
    fn extract_many(
        item: QueryItem<'_, '_, Self::QueryData>,
        output: &mut ExtractOutput<Self::Out>,
    );
}

impl ExtractComponent for CameraFrame {
    type QueryData = &'static CameraFrame;
    type QueryFilter = ();
    type Out = CameraFrame;

    fn extract_component(item: QueryItem<'_, '_, Self::QueryData>) -> Option<Self::Out> {
        Some(item.clone())
    }
}

impl ExtractComponent for VItem {
    type QueryData = &'static VItem;
    type QueryFilter = ();
    type Out = VItem;

    fn extract_component(item: QueryItem<'_, '_, Self::QueryData>) -> Option<Self::Out> {
        Some(item.clone())
    }
}

impl ExtractComponent for MeshItem {
    type QueryData = &'static MeshItem;
    type QueryFilter = ();
    type Out = MeshItem;

    fn extract_component(item: QueryItem<'_, '_, Self::QueryData>) -> Option<Self::Out> {
        Some(item.clone())
    }
}
