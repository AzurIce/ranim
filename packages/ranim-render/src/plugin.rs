use bevy_app::{App, Plugin, PluginGroup, PluginGroupBuilder};
use bevy_ecs::prelude::*;

use crate::{
    pipelines::{mesh_item, vitem},
    primitives::{mesh_items::MeshItemsBuffer, vitems::VItemsBuffer},
    schedule::{RenderPrepare, ViewRender, ViewSystems},
    utils::WgpuContext,
};

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
enum BuiltinSystems {
    VItemDepth,
    MeshDepth,
    VItemColor,
    MeshColor,
}

/// Installs vector-item GPU buffers and their prepare, compute, and draw systems.
///
/// The renderer installs shared schedules and GPU resources before this plugin.
pub struct VItemRenderPlugin;

impl Plugin for VItemRenderPlugin {
    fn build(&self, app: &mut App) {
        let buffer = VItemsBuffer::new(app.world().resource::<WgpuContext>());
        app.insert_resource(buffer)
            .add_systems(RenderPrepare, vitem::prepare_vitems)
            .add_systems(
                ViewRender,
                (
                    vitem::compute.in_set(ViewSystems::Compute),
                    vitem::depth
                        .in_set(ViewSystems::Depth)
                        .in_set(BuiltinSystems::VItemDepth),
                    vitem::color
                        .in_set(ViewSystems::Color)
                        .in_set(BuiltinSystems::VItemColor),
                ),
            );
    }
}

/// Installs mesh GPU buffers and their prepare and draw systems.
///
/// The renderer installs shared schedules and GPU resources before this plugin.
pub struct MeshRenderPlugin;

impl Plugin for MeshRenderPlugin {
    fn build(&self, app: &mut App) {
        let buffer = MeshItemsBuffer::new(app.world().resource::<WgpuContext>());
        app.insert_resource(buffer)
            .add_systems(RenderPrepare, mesh_item::prepare_mesh_items)
            .add_systems(
                ViewRender,
                (
                    mesh_item::depth
                        .in_set(ViewSystems::Depth)
                        .in_set(BuiltinSystems::MeshDepth),
                    mesh_item::color
                        .in_set(ViewSystems::Color)
                        .in_set(BuiltinSystems::MeshColor),
                ),
            );
    }
}

/// Both built-in item renderers with their default drawing order.
///
/// Vector items draw before meshes within each depth and color phase. All
/// depth systems still complete before any color system runs.
/// Supply extraction plugins separately when constructing the renderer.
pub struct BuiltinRenderPlugins;

impl PluginGroup for BuiltinRenderPlugins {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>()
            .add(VItemRenderPlugin)
            .add(MeshRenderPlugin)
            .add(configure_builtin_render_order)
    }
}

fn configure_builtin_render_order(app: &mut App) {
    app.configure_sets(
        ViewRender,
        (BuiltinSystems::VItemDepth, BuiltinSystems::MeshDepth).chain(),
    )
    .configure_sets(
        ViewRender,
        (BuiltinSystems::VItemColor, BuiltinSystems::MeshColor).chain(),
    );
}
