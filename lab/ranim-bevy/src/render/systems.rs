use bevy::{
    camera::visibility::ViewVisibility,
    core_pipeline::core_3d::{Transparent3d, TransparentSortingInfo3d},
    pbr::{MeshPipeline, MeshPipelineKey, ViewKeyCache},
    prelude::*,
    render::{
        Extract,
        render_phase::{
            DrawFunctions, PhaseItemExtraIndex, ViewSortedRenderPhases,
        },
        render_resource::{
            BindGroupEntries, BufferUsages, PipelineCache, SpecializedRenderPipelines,
        },
        renderer::{RenderDevice, RenderQueue},
        view::{ExtractedView, Msaa, RenderVisibleEntities},
    },
};
use ranim_core::glam::Vec3 as RanimVec3;

use crate::component::RanimVItem;

use super::{
    commands::DrawRanimVItem,
    gpu::{GpuVec4, InstanceInfo, ItemInfo, PlaneData, RanimVItemGpuBuffers},
    item::{RenderRanimVItem, RenderRanimVItems, vitem_render_data, vitem_world_center},
    pipeline::RanimVItemPipeline,
    utils::{basis_from_normal, resize_f32_by_sample, resize_vec4_by_sample, vitem_normal_from_points},
};

pub(crate) fn prepare_ranim_vitem_pipeline(
    mut commands: Commands,
    mesh_pipeline: Res<MeshPipeline>,
    pipeline: Option<Res<RanimVItemPipeline>>,
) {
    if pipeline.is_some() {
        return;
    }

    commands.insert_resource(RanimVItemPipeline::new(mesh_pipeline.clone()));
}

pub(crate) fn extract_ranim_vitems(
    query: Extract<Query<(Entity, &ViewVisibility, &GlobalTransform, &RanimVItem)>>,
    mut render_items: ResMut<RenderRanimVItems>,
) {
    render_items.clear();

    for (entity, view_visibility, transform, component) in &query {
        if !view_visibility.get() {
            continue;
        }

        render_items.insert(
            entity.into(),
            RenderRanimVItem {
                main_entity: entity.into(),
                item: vitem_render_data(&component.item),
                world_from_local: transform.affine().into(),
            },
        );
    }
}

pub(crate) fn prepare_ranim_vitem_buffers(
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    pipeline_cache: Res<PipelineCache>,
    pipeline: Option<Res<RanimVItemPipeline>>,
    render_items: Res<RenderRanimVItems>,
    mut buffers: ResMut<RanimVItemGpuBuffers>,
) {
    buffers.entity_instance_indices.clear();

    let items: Vec<_> = render_items
        .values()
        .filter(|item| !item.item.points.is_empty())
        .collect();

    if items.is_empty() {
        buffers.item_count = 0;
        buffers.bind_group = None;
        return;
    }

    let total_points = items.iter().map(|item| item.item.points.len()).sum();
    let total_attrs = items.iter().map(|item| item.item.attr_count()).sum();
    let mut item_infos = Vec::with_capacity(items.len());
    let mut planes = Vec::with_capacity(items.len());
    let mut points = Vec::with_capacity(total_points);
    let mut fill_rgbas = Vec::with_capacity(total_attrs);
    let mut stroke_rgbas = Vec::with_capacity(total_attrs);
    let mut stroke_widths = Vec::with_capacity(total_attrs);
    let mut instances = Vec::with_capacity(items.len());

    let mut point_offset = 0u32;
    let mut attr_offset = 0u32;
    for (item_index, item) in items.iter().enumerate() {
        let point_count = item.item.points.len() as u32;
        let attr_count = item.item.attr_count() as u32;
        item_infos.push(ItemInfo {
            point_offset,
            point_count,
            attr_offset,
            attr_count,
        });

        let normal = item
            .item
            .normal
            .unwrap_or_else(|| vitem_normal_from_points(&item.item.points));
        let origin = item
            .item
            .points
            .first()
            .map(|point| point.truncate())
            .unwrap_or(RanimVec3::ZERO);
        planes.push(PlaneData {
            normal: (normal, 0.0).into(),
            origin: (origin, 0.0).into(),
        });

        let (basis_u, basis_v) = basis_from_normal(normal);
        points.extend(item.item.points.iter().map(|point| {
            let diff = point.truncate() - origin;
            GpuVec4 {
                x: diff.dot(basis_u),
                y: diff.dot(basis_v),
                z: point.w,
                w: 0.0,
            }
        }));
        fill_rgbas.extend(
            resize_vec4_by_sample(&item.item.fill_rgbas, attr_count as usize)
                .into_iter()
                .map(GpuVec4::from),
        );
        stroke_rgbas.extend(
            resize_vec4_by_sample(&item.item.stroke_rgbas, attr_count as usize)
                .into_iter()
                .map(GpuVec4::from),
        );
        stroke_widths.extend(resize_f32_by_sample(
            &item.item.stroke_widths,
            attr_count as usize,
        ));
        instances.push(InstanceInfo {
            world_from_local: item.world_from_local,
            item_index: item_index as u32,
            _padding: [0; 3],
        });

        buffers
            .entity_instance_indices
            .insert(item.main_entity, item_index as u32);
        point_offset += point_count;
        attr_offset += attr_count;
    }

    let storage_usage = BufferUsages::STORAGE | BufferUsages::COPY_DST;
    buffers.item_infos.set(
        &render_device,
        &render_queue,
        "ranim_vitem_item_infos",
        storage_usage,
        item_infos,
    );
    buffers.planes.set(
        &render_device,
        &render_queue,
        "ranim_vitem_planes",
        storage_usage,
        planes,
    );
    buffers.points.set(
        &render_device,
        &render_queue,
        "ranim_vitem_points",
        storage_usage,
        points,
    );
    buffers.fill_rgbas.set(
        &render_device,
        &render_queue,
        "ranim_vitem_fill_rgbas",
        storage_usage,
        fill_rgbas,
    );
    buffers.stroke_rgbas.set(
        &render_device,
        &render_queue,
        "ranim_vitem_stroke_rgbas",
        storage_usage,
        stroke_rgbas,
    );
    buffers.stroke_widths.set(
        &render_device,
        &render_queue,
        "ranim_vitem_stroke_widths",
        storage_usage,
        stroke_widths,
    );
    buffers.instances.set(
        &render_device,
        &render_queue,
        "ranim_vitem_instances",
        storage_usage,
        instances,
    );

    buffers.item_count = items.len() as u32;

    let Some(pipeline) = pipeline else {
        buffers.bind_group = None;
        return;
    };

    let layout = pipeline_cache.get_bind_group_layout(&pipeline.items_layout);
    let Some((
        item_infos,
        planes,
        points,
        fill_rgbas,
        stroke_rgbas,
        stroke_widths,
        instances,
    )) = buffers
        .item_infos
        .buffer()
        .zip(buffers.planes.buffer())
        .zip(buffers.points.buffer())
        .zip(buffers.fill_rgbas.buffer())
        .zip(buffers.stroke_rgbas.buffer())
        .zip(buffers.stroke_widths.buffer())
        .zip(buffers.instances.buffer())
        .map(
            |(
                (((((item_infos, planes), points), fill_rgbas), stroke_rgbas), stroke_widths),
                instances,
            )| {
                (
                    item_infos,
                    planes,
                    points,
                    fill_rgbas,
                    stroke_rgbas,
                    stroke_widths,
                    instances,
                )
            },
        )
    else {
        buffers.bind_group = None;
        return;
    };

    buffers.bind_group = Some(render_device.create_bind_group(
        "ranim_vitem_bind_group",
        &layout,
        &BindGroupEntries::sequential((
            item_infos.as_entire_binding(),
            planes.as_entire_binding(),
            points.as_entire_binding(),
            fill_rgbas.as_entire_binding(),
            stroke_rgbas.as_entire_binding(),
            stroke_widths.as_entire_binding(),
            instances.as_entire_binding(),
        )),
    ));
}

pub(crate) fn queue_ranim_vitems(
    transparent_draw_functions: Res<DrawFunctions<Transparent3d>>,
    pipeline: Option<Res<RanimVItemPipeline>>,
    mut pipelines: ResMut<SpecializedRenderPipelines<RanimVItemPipeline>>,
    pipeline_cache: Res<PipelineCache>,
    render_items: Res<RenderRanimVItems>,
    buffers: Res<RanimVItemGpuBuffers>,
    mut transparent_render_phases: ResMut<ViewSortedRenderPhases<Transparent3d>>,
    view_key_cache: Res<ViewKeyCache>,
    views: Query<(&RenderVisibleEntities, &ExtractedView, &Msaa)>,
) {
    if buffers.item_count == 0 || render_items.is_empty() {
        return;
    }
    let Some(pipeline) = pipeline else {
        return;
    };

    let draw_function = transparent_draw_functions.read().id::<DrawRanimVItem>();

    for (visible_entities, view, msaa) in &views {
        let Some(transparent_phase) = transparent_render_phases.get_mut(&view.retained_view_entity)
        else {
            continue;
        };
        let Some(visible_entities) = visible_entities.get::<RanimVItem>() else {
            continue;
        };

        let view_key = view_key_cache
            .get(&view.retained_view_entity)
            .copied()
            .unwrap_or_else(|| {
                MeshPipelineKey::from_msaa_samples(msaa.samples())
                    | MeshPipelineKey::from_target_format(view.target_format)
                    | MeshPipelineKey::BLEND_ALPHA
            });
        let pipeline_id = pipelines.specialize(
            &pipeline_cache,
            &pipeline,
            view_key | MeshPipelineKey::BLEND_ALPHA,
        );

        for (_render_entity, visible_entity) in visible_entities.iter_visible() {
            let Some(item) = render_items.get(visible_entity) else {
                continue;
            };
            let Some(instance_index) = buffers.entity_instance_indices.get(visible_entity) else {
                continue;
            };
            transparent_phase.add_retained(Transparent3d {
                sorting_info: TransparentSortingInfo3d::Sorted {
                    mesh_center: vitem_world_center(item),
                    depth_bias: 0.0,
                },
                distance: 0.0,
                entity: (Entity::PLACEHOLDER, *visible_entity),
                pipeline: pipeline_id,
                draw_function,
                batch_range: *instance_index..(*instance_index + 1),
                extra_index: PhaseItemExtraIndex::None,
                indexed: false,
            });
        }
    }
}
