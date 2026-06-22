use bevy::{
    camera::visibility::{self, NoFrustumCulling, Visibility, VisibilityClass},
    prelude::*,
    render::sync_component::SyncComponent,
};
use ranim_core::VItem;

/// A Bevy component containing a Ranim vector item.
///
/// Entities with this component are normal Bevy render objects: they get
/// visibility components, sync into the render world, and queue into Bevy's
/// `Transparent3d` render phase.
#[derive(Component, Clone, Debug)]
#[require(Transform, Visibility, VisibilityClass, NoFrustumCulling)]
#[component(on_add = visibility::add_visibility_class::<RanimVItem>)]
pub struct RanimVItem {
    /// The vector item to render.
    pub item: VItem,
}

impl RanimVItem {
    /// Create a component from a Ranim [`VItem`].
    pub fn new(item: VItem) -> Self {
        Self { item }
    }
}

impl From<VItem> for RanimVItem {
    fn from(item: VItem) -> Self {
        Self::new(item)
    }
}

impl SyncComponent for RanimVItem {
    type Target = Self;
}
