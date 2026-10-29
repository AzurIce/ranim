//! Shared metadata for render-world entities.

use bevy_ecs::prelude::*;

/// Source position of an entity produced by frame extraction.
///
/// Extraction systems copy [`ranim_core::frame::FrameItem::order`] into
/// [`Self::order`] and assign successive [`Self::part`] values when one input
/// produces several ordered entities. The renderer resolves these keys into
/// [`SceneOrder`] after extraction, before preparing GPU data.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct SourceOrder {
    /// Global insertion order of the source frame item.
    pub order: usize,
    /// Output position within the source item.
    pub part: usize,
}

/// Resolved painter's order of a render-world entity.
///
/// The renderer assigns consecutive ranks from [`SourceOrder`] after
/// extraction. GPU preparation systems use this rank to resolve coplanar
/// geometry consistently across every typed input batch and expanded output.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct SceneOrder(
    /// Zero-based rank in the extracted scene.
    pub usize,
);

pub(crate) fn resolve_scene_order(world: &mut World) {
    let mut ordered = world
        .query::<(Entity, &SourceOrder)>()
        .iter(world)
        .map(|(entity, order)| (*order, entity))
        .collect::<Vec<_>>();
    ordered.sort_unstable_by_key(|(order, entity)| (*order, *entity));
    for (rank, (_, entity)) in ordered.into_iter().enumerate() {
        let order = SceneOrder(rank);
        if world.get::<SceneOrder>(entity) != Some(&order) {
            world.entity_mut(entity).insert(order);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expanded_inputs_keep_their_parts_between_surrounding_sources() {
        let mut world = World::new();
        let later = world.spawn(SourceOrder { order: 7, part: 0 }).id();
        let second = world.spawn(SourceOrder { order: 3, part: 1 }).id();
        let first = world.spawn(SourceOrder { order: 3, part: 0 }).id();
        let earlier = world.spawn(SourceOrder { order: 0, part: 0 }).id();

        resolve_scene_order(&mut world);

        for (rank, entity) in [earlier, first, second, later].into_iter().enumerate() {
            assert_eq!(world.get::<SceneOrder>(entity), Some(&SceneOrder(rank)));
        }
    }
}
