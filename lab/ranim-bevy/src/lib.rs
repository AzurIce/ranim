//! Experimental Bevy integration for Ranim.
//!
//! This crate keeps Bevy as an optional host. Ranim data remains plain
//! `ranim-core` data, while Bevy owns extraction, visibility, render phases,
//! and drawing.

use std::borrow::Cow;

use bevy::{
    asset::uuid_handle,
    camera::visibility::{self, NoFrustumCulling, ViewVisibility, Visibility, VisibilityClass},
    core_pipeline::core_3d::{Transparent3d, TransparentSortingInfo3d, CORE_3D_DEPTH_FORMAT},
    ecs::system::{lifetimeless::SRes, SystemParamItem},
    pbr::{
        MeshPipeline, MeshPipelineKey, SetMeshViewBindGroup, SetMeshViewBindingArrayBindGroup,
        ViewKeyCache,
    },
    prelude::*,
    render::{
        Extract, ExtractSchedule, Render, RenderApp, RenderSystems,
        render_phase::{
            AddRenderCommand, DrawFunctions, PhaseItem, PhaseItemExtraIndex, RenderCommand,
            RenderCommandResult, SetItemPipeline, TrackedRenderPass, ViewSortedRenderPhases,
        },
        render_resource::{
            BindGroup, BindGroupEntries, BindGroupLayoutDescriptor, BindGroupLayoutEntries,
            BlendState, Buffer, BufferInitDescriptor, BufferUsages, ColorTargetState, ColorWrites,
            CompareFunction, DepthBiasState, DepthStencilState, FragmentState, MultisampleState,
            PipelineCache, PrimitiveState, PrimitiveTopology,
            RenderPipelineDescriptor, ShaderStages, ShaderType, SpecializedRenderPipeline,
            SpecializedRenderPipelines, StencilFaceState, StencilState, VertexState,
            binding_types::storage_buffer_read_only,
        },
        renderer::{RenderDevice, RenderQueue},
        sync_component::{SyncComponent, SyncComponentPlugin},
        sync_world::{MainEntity, MainEntityHashMap},
        view::{ExtractedView, Msaa, RenderVisibleEntities},
    },
};
use bytemuck::{Pod, Zeroable};
use ranim_core::{
    CameraFrame, VItem,
    glam::Vec3 as RanimVec3,
    store::CoreItemStore,
};
use ranim_render::scene::VItemRenderData;

const RANIM_VITEM_SHADER_HANDLE: Handle<Shader> =
    uuid_handle!("1c7031bb-904d-47c2-9b79-30e6f3fe0a91");

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

#[derive(Clone, Debug)]
struct RenderRanimVItem {
    main_entity: MainEntity,
    item: VItemRenderData,
    world_from_local: Mat4,
}

#[derive(Resource, Default, Deref, DerefMut)]
struct RenderRanimVItems(MainEntityHashMap<RenderRanimVItem>);

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Pod, Zeroable, ShaderType)]
struct ItemInfo {
    point_offset: u32,
    point_count: u32,
    attr_offset: u32,
    attr_count: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Pod, Zeroable, ShaderType)]
struct PlaneData {
    normal: GpuVec4,
    origin: GpuVec4,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Pod, Zeroable, ShaderType)]
struct GpuVec4 {
    x: f32,
    y: f32,
    z: f32,
    w: f32,
}

impl From<Vec4> for GpuVec4 {
    fn from(value: Vec4) -> Self {
        Self {
            x: value.x,
            y: value.y,
            z: value.z,
            w: value.w,
        }
    }
}

impl From<(RanimVec3, f32)> for GpuVec4 {
    fn from((value, w): (RanimVec3, f32)) -> Self {
        Self {
            x: value.x,
            y: value.y,
            z: value.z,
            w,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Pod, Zeroable, ShaderType)]
struct InstanceInfo {
    world_from_local: Mat4,
    item_index: u32,
    _padding: [u32; 3],
}

struct BufferSlot<T> {
    values: Vec<T>,
    buffer: Option<Buffer>,
}

impl<T> Default for BufferSlot<T> {
    fn default() -> Self {
        Self {
            values: Vec::new(),
            buffer: None,
        }
    }
}

impl<T: Pod> BufferSlot<T> {
    fn set(
        &mut self,
        render_device: &RenderDevice,
        render_queue: &RenderQueue,
        label: &'static str,
        usage: BufferUsages,
        values: Vec<T>,
    ) {
        self.values = if values.is_empty() {
            vec![T::zeroed()]
        } else {
            values
        };

        let data = bytemuck::cast_slice(&self.values);
        let needs_recreate = self
            .buffer
            .as_ref()
            .is_none_or(|buffer| buffer.size() < data.len() as u64);

        if needs_recreate {
            self.buffer = Some(render_device.create_buffer_with_data(&BufferInitDescriptor {
                label: Some(label),
                contents: data,
                usage,
            }));
        } else if let Some(buffer) = &self.buffer {
            render_queue.write_buffer(buffer, 0, data);
        }
    }

    fn buffer(&self) -> Option<&Buffer> {
        self.buffer.as_ref()
    }
}

#[derive(Resource, Default)]
pub struct RanimVItemGpuBuffers {
    item_infos: BufferSlot<ItemInfo>,
    planes: BufferSlot<PlaneData>,
    points: BufferSlot<GpuVec4>,
    fill_rgbas: BufferSlot<GpuVec4>,
    stroke_rgbas: BufferSlot<GpuVec4>,
    stroke_widths: BufferSlot<f32>,
    instances: BufferSlot<InstanceInfo>,
    entity_instance_indices: MainEntityHashMap<u32>,
    bind_group: Option<BindGroup>,
    item_count: u32,
}

#[derive(Resource, Clone)]
struct RanimVItemPipeline {
    shader: Handle<Shader>,
    mesh_pipeline: MeshPipeline,
    items_layout: BindGroupLayoutDescriptor,
}

impl SpecializedRenderPipeline for RanimVItemPipeline {
    type Key = MeshPipelineKey;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        let mut shader_defs = Vec::new();
        if key.contains(MeshPipelineKey::OIT_ENABLED) {
            shader_defs.push("OIT_ENABLED".into());
        }
        if key.contains(MeshPipelineKey::DEPTH_PREPASS) {
            shader_defs.push("DEPTH_PREPASS".into());
        }
        if key.msaa_samples() > 1 {
            shader_defs.push("MULTISAMPLED".into());
        }

        let blend = if key.contains(MeshPipelineKey::OIT_ENABLED) {
            None
        } else {
            Some(BlendState::ALPHA_BLENDING)
        };
        let view_layout = self.mesh_pipeline.get_view_layout(key.into());

        RenderPipelineDescriptor {
            label: Some(Cow::Borrowed("ranim_vitem_pipeline")),
            layout: vec![
                view_layout.main_layout,
                view_layout.binding_array_layout,
                self.items_layout.clone(),
            ],
            vertex: VertexState {
                shader: self.shader.clone(),
                shader_defs: shader_defs.clone(),
                entry_point: Some(Cow::Borrowed("vs_main")),
                buffers: vec![],
                ..default()
            },
            fragment: Some(FragmentState {
                shader: self.shader.clone(),
                shader_defs,
                entry_point: Some(Cow::Borrowed("fs_main")),
                targets: vec![Some(ColorTargetState {
                    format: key.target_format(),
                    blend,
                    write_mask: ColorWrites::ALL,
                })],
                ..default()
            }),
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleStrip,
                cull_mode: None,
                ..default()
            },
            depth_stencil: Some(DepthStencilState {
                format: CORE_3D_DEPTH_FORMAT,
                depth_write_enabled: Some(false),
                depth_compare: Some(CompareFunction::GreaterEqual),
                stencil: StencilState {
                    front: StencilFaceState::IGNORE,
                    back: StencilFaceState::IGNORE,
                    read_mask: 0,
                    write_mask: 0,
                },
                bias: DepthBiasState::default(),
            }),
            multisample: MultisampleState {
                count: key.msaa_samples(),
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            ..default()
        }
    }
}

type DrawRanimVItem = (
    SetItemPipeline,
    SetMeshViewBindGroup<0>,
    SetMeshViewBindingArrayBindGroup<1>,
    SetRanimVItemBindGroup<2>,
    DrawRanimVItemQuad,
);

fn prepare_ranim_vitem_pipeline(
    mut commands: Commands,
    mesh_pipeline: Res<MeshPipeline>,
    pipeline: Option<Res<RanimVItemPipeline>>,
) {
    if pipeline.is_some() {
        return;
    }

    let items_layout = BindGroupLayoutDescriptor::new(
        "ranim_vitem_items_layout",
        &BindGroupLayoutEntries::sequential(
            ShaderStages::VERTEX_FRAGMENT,
            (
                storage_buffer_read_only::<ItemInfo>(false),
                storage_buffer_read_only::<PlaneData>(false),
                storage_buffer_read_only::<GpuVec4>(false),
                storage_buffer_read_only::<GpuVec4>(false),
                storage_buffer_read_only::<GpuVec4>(false),
                storage_buffer_read_only::<f32>(false),
                storage_buffer_read_only::<InstanceInfo>(false),
            ),
        ),
    );

    commands.insert_resource(RanimVItemPipeline {
        shader: RANIM_VITEM_SHADER_HANDLE,
        mesh_pipeline: mesh_pipeline.clone(),
        items_layout,
    });
}

fn extract_ranim_vitems(
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

fn prepare_ranim_vitem_buffers(
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

fn queue_ranim_vitems(
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

pub struct SetRanimVItemBindGroup<const I: usize>;

impl<P: PhaseItem, const I: usize> RenderCommand<P> for SetRanimVItemBindGroup<I> {
    type Param = SRes<RanimVItemGpuBuffers>;
    type ViewQuery = ();
    type ItemQuery = ();

    #[inline]
    fn render<'w>(
        _item: &P,
        _view: (),
        _entity: Option<()>,
        buffers: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let buffers = buffers.into_inner();
        let Some(bind_group) = &buffers.bind_group else {
            return RenderCommandResult::Skip;
        };
        pass.set_bind_group(I, bind_group, &[]);
        RenderCommandResult::Success
    }
}

pub struct DrawRanimVItemQuad;

impl<P: PhaseItem> RenderCommand<P> for DrawRanimVItemQuad {
    type Param = ();
    type ViewQuery = ();
    type ItemQuery = ();

    #[inline]
    fn render<'w>(
        item: &P,
        _view: (),
        _entity: Option<()>,
        _param: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        pass.draw(0..4, item.batch_range().clone());
        RenderCommandResult::Success
    }
}

fn vitem_world_center(item: &RenderRanimVItem) -> Vec3 {
    if item.item.points.is_empty() {
        return item.world_from_local.transform_point3(Vec3::ZERO);
    }

    let sum = item.item.points.iter().fold(Vec3::ZERO, |sum, point| {
        sum + Vec3::new(point.x, point.y, point.z)
    });
    let local_center = sum / item.item.points.len() as f32;
    item.world_from_local.transform_point3(local_center)
}

fn vitem_render_data(vitem: &VItem) -> VItemRenderData {
    let points = vitem.get_render_points();
    let attr_count = points.len().div_ceil(2);

    VItemRenderData {
        points,
        normal: vitem.normal.map(|normal| normal.as_vec3()),
        fill_rgbas: vitem
            .fill_rgbas
            .resize_by_sample(attr_count)
            .into_iter()
            .map(|rgba| rgba.0)
            .collect(),
        stroke_rgbas: vitem
            .stroke_rgbas
            .resize_by_sample(attr_count)
            .into_iter()
            .map(|rgba| rgba.0)
            .collect(),
        stroke_widths: vitem
            .stroke_widths
            .resize_by_sample(attr_count)
            .into_iter()
            .map(|width| width.0)
            .collect(),
    }
}

fn vitem_normal_from_points(points: &[Vec4]) -> Vec3 {
    if points.len() < 3 {
        return Vec3::Z;
    }
    let p0 = points[0].truncate();
    let p1 = points[1].truncate();
    let p2 = points[2].truncate();
    let normal = (p1 - p0).cross(p2 - p0);
    if normal.length_squared() < 1e-6 {
        Vec3::Z
    } else {
        normal.normalize()
    }
}

fn basis_from_normal(normal: RanimVec3) -> (RanimVec3, RanimVec3) {
    let arbitrary = if normal.x.abs() > 0.99 {
        RanimVec3::Y
    } else {
        RanimVec3::X
    };
    let basis_u = normal.cross(arbitrary).normalize();
    let basis_v = normal.cross(basis_u);
    (basis_u, basis_v)
}

fn resize_vec4_by_sample(values: &[Vec4], target_len: usize) -> Vec<Vec4> {
    if target_len == 0 {
        return Vec::new();
    }
    if values.is_empty() {
        return vec![Vec4::ZERO; target_len];
    }
    if values.len() == target_len {
        return values.to_vec();
    }

    let step = values.len() as f32 / target_len as f32;
    (0..target_len)
        .map(|idx| {
            let source_idx = (idx as f32 * step).floor() as usize;
            values[source_idx.min(values.len() - 1)]
        })
        .collect()
}

fn resize_f32_by_sample(values: &[f32], target_len: usize) -> Vec<f32> {
    if target_len == 0 {
        return Vec::new();
    }
    if values.is_empty() {
        return vec![0.0; target_len];
    }
    if values.len() == target_len {
        return values.to_vec();
    }

    let step = values.len() as f32 / target_len as f32;
    (0..target_len)
        .map(|idx| {
            let source_idx = (idx as f32 * step).floor() as usize;
            values[source_idx.min(values.len() - 1)]
        })
        .collect()
}

/// Fill a [`CoreItemStore`] from Ranim VItems.
pub fn collect_vitems_into_store(items: impl IntoIterator<Item = VItem>) -> CoreItemStore {
    let mut store = CoreItemStore::new();
    store.camera_frames.push(CameraFrame::default());
    store.vitems.extend(items);
    store
}

const RANIM_VITEM_SHADER: &str = r#"
#import bevy_pbr::mesh_view_bindings::view

#ifdef OIT_ENABLED
#import bevy_core_pipeline::oit::oit_draw
#endif

struct ItemInfo {
    point_offset: u32,
    point_count: u32,
    attr_offset: u32,
    attr_count: u32,
}

struct PlaneData {
    normal: vec4<f32>,
    origin: vec4<f32>,
}

struct InstanceInfo {
    world_from_local: mat4x4<f32>,
    item_index: u32,
    _padding: array<u32, 3>,
}

@group(2) @binding(0) var<storage> item_infos: array<ItemInfo>;
@group(2) @binding(1) var<storage> planes: array<PlaneData>;
@group(2) @binding(2) var<storage> points: array<vec4<f32>>;
@group(2) @binding(3) var<storage> fill_rgbas: array<vec4<f32>>;
@group(2) @binding(4) var<storage> stroke_rgbas: array<vec4<f32>>;
@group(2) @binding(5) var<storage> stroke_widths: array<f32>;
@group(2) @binding(6) var<storage> instances: array<InstanceInfo>;

struct Basis {
    u: vec3<f32>,
    v: vec3<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) local_pos: vec2<f32>,
    @location(1) @interpolate(flat) item_index: u32,
}

fn basis_from_normal(n: vec3<f32>) -> Basis {
    let arbitrary = select(vec3(1.0, 0.0, 0.0), vec3(0.0, 1.0, 0.0), abs(n.x) > 0.99);
    let basis_u = normalize(cross(n, arbitrary));
    let basis_v = cross(n, basis_u);
    return Basis(basis_u, basis_v);
}

fn world_point_for_plane(local_pos: vec2<f32>, instance_index: u32) -> vec3<f32> {
    let instance = instances[instance_index];
    let plane = planes[instance.item_index];
    let basis = basis_from_normal(plane.normal.xyz);
    let local_plane_point = plane.origin.xyz + basis.u * local_pos.x + basis.v * local_pos.y;
    return (instance.world_from_local * vec4<f32>(local_plane_point, 1.0)).xyz;
}

@vertex
fn vs_main(
    @builtin(vertex_index) vertex_index: u32,
    @builtin(instance_index) instance_index: u32,
) -> VertexOutput {
    let item_index = instances[instance_index].item_index;
    let info = item_infos[item_index];
    var min_xy = vec2<f32>(3.402823e38);
    var max_xy = vec2<f32>(-3.402823e38);
    var stroke_max = 0.0;

    for (var i = 0u; i < info.point_count; i = i + 1u) {
        let point = points[info.point_offset + i];
        min_xy = min(min_xy, point.xy);
        max_xy = max(max_xy, point.xy);
    }
    for (var i = 0u; i < info.attr_count; i = i + 1u) {
        stroke_max = max(stroke_max, stroke_widths[info.attr_offset + i]);
    }

    let pad = max(stroke_max * 2.0, 0.08);
    min_xy -= vec2<f32>(pad);
    max_xy += vec2<f32>(pad);

    let corner = array<vec2<f32>, 4>(
        vec2<f32>(min_xy.x, min_xy.y),
        vec2<f32>(max_xy.x, min_xy.y),
        vec2<f32>(min_xy.x, max_xy.y),
        vec2<f32>(max_xy.x, max_xy.y),
    );

    var out: VertexOutput;
    out.local_pos = corner[vertex_index];
    out.item_index = item_index;
    out.clip_position = view.clip_from_world * vec4<f32>(world_point_for_plane(out.local_pos, instance_index), 1.0);
    return out;
}

fn item_point(info: ItemInfo, local_idx: u32) -> vec2<f32> {
    return points[info.point_offset + local_idx].xy;
}

fn item_is_closed(info: ItemInfo, local_idx: u32) -> bool {
    return bool(points[info.point_offset + local_idx].z);
}

fn item_fill_rgba(info: ItemInfo, anchor_idx: u32) -> vec4<f32> {
    return fill_rgbas[info.attr_offset + min(anchor_idx, info.attr_count - 1u)];
}

fn item_stroke_rgba(info: ItemInfo, anchor_idx: u32) -> vec4<f32> {
    return stroke_rgbas[info.attr_offset + min(anchor_idx, info.attr_count - 1u)];
}

fn item_stroke_width(info: ItemInfo, anchor_idx: u32) -> f32 {
    return stroke_widths[info.attr_offset + min(anchor_idx, info.attr_count - 1u)];
}

fn cross_2d(a: vec2<f32>, b: vec2<f32>) -> f32 {
    return a.x * b.y - a.y * b.x;
}

fn blend_color(f: vec4<f32>, b: vec4<f32>) -> vec4<f32> {
    let a = f.a + b.a * (1.0 - f.a);
    if a <= 0.0 {
        return vec4<f32>(0.0);
    }
    return vec4<f32>(
        (f.rgb * f.a + b.rgb * b.a * (1.0 - f.a)) / a,
        a
    );
}

fn solve_cubic(a: f32, b: f32, c: f32) -> vec3<f32> {
    let p = b - a * a / 3.0;
    let p3 = p * p * p;
    let q = a * (2.0 * a * a - 9.0 * b) / 27.0 + c;
    let d = q * q + 4.0 * p3 / 27.0;
    let offset = -a / 3.0;
    if (d >= 0.0) {
        let z = sqrt(d);
        let x = (vec2<f32>(z, -z) - q) / 2.0;
        let uv = sign(x) * pow(abs(x), vec2<f32>(1.0 / 3.0));
        return vec3<f32>(offset + uv.x + uv.y);
    }
    let v = acos(-sqrt(-27.0 / p3) * q / 2.0) / 3.0;
    let m = cos(v);
    let n = sin(v) * 1.732050808;
    return vec3<f32>(m + m, -n - m, n - m) * sqrt(-p / 3.0) + offset;
}

fn distance_bezier(pos: vec2<f32>, A: vec2<f32>, _B: vec2<f32>, C: vec2<f32>) -> f32 {
    var B = mix(_B + vec2<f32>(1e-4), _B, abs(sign(_B * 2.0 - A - C)));
    let a = B - A;
    let b = A - B * 2.0 + C;
    let c = a * 2.0;
    let d = A - pos;
    let k = vec3<f32>(3.0 * dot(a, b), 2.0 * dot(a, a) + dot(d, b), dot(d, a)) / dot(b, b);
    let solved = solve_cubic(k.x, k.y, k.z);
    let t = vec3<f32>(
        clamp(solved.x, 0.0, 1.0),
        clamp(solved.y, 0.0, 1.0),
        clamp(solved.z, 0.0, 1.0),
    );
    var ppos = A + (c + b * t.x) * t.x;
    var dis = length(ppos - pos);
    ppos = A + (c + b * t.y) * t.y;
    dis = min(dis, length(ppos - pos));
    ppos = A + (c + b * t.z) * t.z;
    dis = min(dis, length(ppos - pos));
    return dis;
}

fn distance_line(pos: vec2<f32>, A: vec2<f32>, B: vec2<f32>) -> f32 {
    let e = B - A;
    let w = pos - A;
    let b = w - e * clamp(dot(w, e) / dot(e, e), 0.0, 1.0);
    return length(b);
}

fn sign_line(p: vec2<f32>, A: vec2<f32>, B: vec2<f32>) -> f32 {
    let cond = vec3<bool>(
        (p.y >= A.y),
        (p.y < B.y),
        (cross_2d(B - A, p - A) > 0.0),
    );
    return select(1.0, -1.0, all(cond) || !any(cond));
}

fn sign_bezier(p: vec2<f32>, A: vec2<f32>, B: vec2<f32>, C: vec2<f32>) -> f32 {
    let a = C - A;
    let b = B - A;
    let c = p - A;
    let denominator = a.x * b.y - b.x * a.y;
    let bary = vec2<f32>(cross_2d(c, b), cross_2d(a, c)) / denominator;
    let d = vec2<f32>(bary.y * 0.5, 0.0) + 1.0 - bary.x - bary.y;
    let sign_inside = select(1.0, sign(d.x * d.x - d.y), d.x > d.y);
    let sign_left = sign_line(p, A, C);
    return sign_inside * sign_left;
}

struct SubpathAttr {
    end_idx: u32,
    nearest_idx: u32,
    d: f32,
    sgn: f32,
}

fn get_subpath_attr(pos: vec2<f32>, info: ItemInfo, start_local_idx: u32) -> SubpathAttr {
    var attr: SubpathAttr;
    attr.end_idx = info.point_count;
    attr.nearest_idx = 0u;
    attr.d = 3.402823e38;
    attr.sgn = 1.0;

    let n = (info.point_count - 1u) / 2u * 2u;
    for (var i = start_local_idx; i < n; i = i + 2u) {
        let a = item_point(info, i);
        let b = item_point(info, i + 1u);
        let c = item_point(info, i + 2u);
        if length(b - a) == 0.0 {
            attr.end_idx = i;
            break;
        }

        let v1 = normalize(b - a);
        let v2 = normalize(c - b);
        let is_line = abs(cross_2d(v1, v2)) < 0.0001 && dot(v1, v2) > 0.0;
        let dist = select(distance_bezier(pos, a, b, c), distance_line(pos, a, c), is_line);
        if dist < attr.d {
            attr.d = dist;
            attr.nearest_idx = i;
        }
        if item_is_closed(info, i) {
            attr.sgn *= select(sign_bezier(pos, a, b, c), sign_line(pos, a, c), is_line);
        }
    }

    return attr;
}

fn render_vitem(pos: vec2<f32>, info: ItemInfo) -> vec4<f32> {
    var idx = 0u;
    var d = 3.402823e38;
    var sgn = 1.0;

    var start_idx = 0u;
    while start_idx < info.point_count {
        let attr = get_subpath_attr(pos, info, start_idx);
        if attr.d < d {
            idx = attr.nearest_idx;
            d = attr.d;
        }
        sgn *= attr.sgn;
        start_idx = attr.end_idx + 2u;
    }

    let sgn_d = sgn * d;
    let e = item_point(info, idx + 1u) - item_point(info, idx);
    let w = pos - item_point(info, idx);
    let ratio = clamp(dot(w, e) / dot(e, e), 0.0, 1.0);
    let anchor_index = idx / 2u;
    let antialias_radius = 0.015 / 4.0;

    var fill_rgba = select(
        vec4<f32>(0.0),
        mix(item_fill_rgba(info, anchor_index), item_fill_rgba(info, anchor_index + 1u), ratio),
        item_is_closed(info, idx)
    );
    fill_rgba.a *= smoothstep(1.0, -1.0, sgn_d / antialias_radius);

    var stroke_width = mix(
        item_stroke_width(info, anchor_index),
        item_stroke_width(info, anchor_index + 1u),
        ratio
    );
    var stroke_rgba = mix(
        item_stroke_rgba(info, anchor_index),
        item_stroke_rgba(info, anchor_index + 1u),
        ratio
    );
    stroke_rgba.a *= smoothstep(1.0, -1.0, (d - stroke_width) / antialias_radius);

    return blend_color(stroke_rgba, fill_rgba);
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let info = item_infos[in.item_index];
    let color = render_vitem(in.local_pos, info);
    if color.a < 0.01 {
        discard;
    }
#ifdef OIT_ENABLED
    oit_draw(in.clip_position, color);
    discard;
#endif
    return color;
}
"#;
