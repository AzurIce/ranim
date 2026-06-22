use bevy::{
    core_pipeline::core_3d::Transparent3d,
    prelude::*,
    render::{
        ExtractSchedule, Render, RenderApp, RenderSystems,
        render_phase::AddRenderCommand,
        render_resource::SpecializedRenderPipelines,
        sync_component::SyncComponentPlugin,
    },
};

use crate::{
    component::RanimVItem,
    render::{
        DrawRanimVItem, RanimVItemGpuBuffers, RanimVItemPipeline, RenderRanimVItems,
        extract_ranim_vitems, prepare_ranim_vitem_buffers, prepare_ranim_vitem_pipeline,
        queue_ranim_vitems,
    },
    shader::{RANIM_VITEM_SHADER, RANIM_VITEM_SHADER_HANDLE},
};

/// Configuration for [`RanimBevyPlugin`].
#[derive(Clone, Debug, Default)]
pub struct RanimBevyPlugin;

impl Plugin for RanimBevyPlugin {
    fn build(&self, app: &mut App) {
        let mut shaders = app.world_mut().resource_mut::<Assets<Shader>>();
        let _ = shaders.insert(
            RANIM_VITEM_SHADER_HANDLE.id(),
            Shader::from_wgsl(RANIM_VITEM_SHADER, file!()),
        );

        app.add_plugins(SyncComponentPlugin::<RanimVItem>::default());

        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            tracing::warn!("RanimBevyPlugin requires Bevy's RenderApp; add it after DefaultPlugins");
            return;
        };

        render_app
            .add_render_command::<Transparent3d, DrawRanimVItem>()
            .init_resource::<RenderRanimVItems>()
            .init_resource::<RanimVItemGpuBuffers>()
            .init_resource::<SpecializedRenderPipelines<RanimVItemPipeline>>()
            .add_systems(ExtractSchedule, extract_ranim_vitems)
            .add_systems(
                Render,
                (
                    prepare_ranim_vitem_pipeline.in_set(RenderSystems::PrepareResources),
                    prepare_ranim_vitem_buffers
                        .in_set(RenderSystems::PrepareResources)
                        .after(prepare_ranim_vitem_pipeline),
                    queue_ranim_vitems.in_set(RenderSystems::QueueMeshes),
                ),
            );
    }
}
