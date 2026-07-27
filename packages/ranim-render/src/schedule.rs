use bevy_ecs::{prelude::*, schedule::ScheduleLabel};
use ranim_core::core_item::{camera_frame::CameraFrame, mesh_item::MeshItem, vitem::VItem};

use crate::{
    ResolutionInfo,
    pipelines::{
        MeshItemColorPipeline, MeshItemDepthPipeline, OITResolvePipeline, VItemColorPipeline,
        VItemComputePipeline, VItemDepthPipeline,
    },
    primitives::{
        mesh_items::MeshItemsBuffer,
        viewport::{ViewportGpuPacket, ViewportUniform},
        vitems::VItemsBuffer,
    },
    resource::{PipelinesPool, RenderTextureState, RenderTextures},
    utils::WgpuContext,
    world::SceneOrder,
};

#[derive(ScheduleLabel, Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct RenderPrepare;

#[derive(ScheduleLabel, Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct RenderGraph;

#[derive(ScheduleLabel, Debug, Clone, PartialEq, Eq, Hash)]
struct ViewRender;

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
enum PrepareSystems {
    Collect,
    PrepareResources,
    Upload,
    PrepareBindGroups,
}

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
enum RenderGraphSystems {
    Begin,
    Render,
    Submit,
    Finish,
}

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
enum ViewSystems {
    Compute,
    Depth,
    Color,
}

#[derive(Resource, Clone, Copy)]
pub(crate) struct RenderDimensions {
    pub width: u32,
    pub height: u32,
}

#[derive(Resource)]
pub(crate) struct FrameTarget {
    render_view: wgpu::TextureView,
    depth_stencil_view: wgpu::TextureView,
    depth_bind_group: wgpu::BindGroup,
    clear_color: wgpu::Color,
    texture_state: RenderTextureState,
}

impl FrameTarget {
    pub(crate) fn new(textures: &RenderTextures, clear_color: wgpu::Color) -> Self {
        Self {
            render_view: textures.render_view.clone(),
            depth_stencil_view: textures.depth_stencil_view.clone(),
            depth_bind_group: textures.depth_bind_group.clone(),
            clear_color,
            texture_state: textures.state(),
        }
    }
}

#[derive(Resource, Default)]
struct FrameEncoder(Option<wgpu::CommandEncoder>);

#[cfg(feature = "profiling")]
#[derive(Resource)]
pub(crate) struct RenderProfiler(pub wgpu_profiler::GpuProfiler);

pub(crate) fn install_schedules(world: &mut World) {
    world.init_resource::<FrameEncoder>();
    let mut prepare = Schedule::new(RenderPrepare);
    prepare.configure_sets(
        (
            PrepareSystems::Collect,
            PrepareSystems::PrepareResources,
            PrepareSystems::Upload,
            PrepareSystems::PrepareBindGroups,
        )
            .chain(),
    );
    prepare.add_systems(
        (prepare_vitems, prepare_mesh_items)
            .in_set(PrepareSystems::PrepareResources)
            .ambiguous_with_all(),
    );
    world.add_schedule(prepare);

    let mut view = Schedule::new(ViewRender);
    view.configure_sets((ViewSystems::Compute, ViewSystems::Depth, ViewSystems::Color).chain());
    view.add_systems(vitem_compute.in_set(ViewSystems::Compute));
    view.add_systems((vitem_depth, mesh_depth).in_set(ViewSystems::Depth));
    view.add_systems((vitem_color, mesh_color).in_set(ViewSystems::Color));
    world.add_schedule(view);

    let mut graph = Schedule::new(RenderGraph);
    graph.configure_sets(
        (
            RenderGraphSystems::Begin,
            RenderGraphSystems::Render,
            RenderGraphSystems::Submit,
            RenderGraphSystems::Finish,
        )
            .chain(),
    );
    graph.add_systems(begin_frame.in_set(RenderGraphSystems::Begin));
    graph.add_systems(
        (clear, view_driver, oit_resolve)
            .chain()
            .in_set(RenderGraphSystems::Render),
    );
    graph.add_systems(submit.in_set(RenderGraphSystems::Submit));
    graph.add_systems(finish_frame.in_set(RenderGraphSystems::Finish));
    world.add_schedule(graph);
}

fn prepare_vitems(
    ctx: Res<WgpuContext>,
    mut buffer: ResMut<VItemsBuffer>,
    items: Query<(&SceneOrder, &VItem)>,
) {
    let mut items = items.iter().collect::<Vec<_>>();
    items.sort_by_key(|(order, _)| order.0);
    buffer.update(&ctx, items.into_iter().map(|(_, item)| item));
}

fn prepare_mesh_items(
    ctx: Res<WgpuContext>,
    mut buffer: ResMut<MeshItemsBuffer>,
    items: Query<(&SceneOrder, &MeshItem)>,
) {
    let mut items = items.iter().collect::<Vec<_>>();
    items.sort_by_key(|(order, _)| order.0);
    buffer.update(&ctx, items.into_iter().map(|(_, item)| item));
}

fn begin_frame(ctx: Res<WgpuContext>, mut encoder: ResMut<FrameEncoder>) {
    encoder.0 = Some(
        ctx.device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default()),
    );
}

fn clear(mut encoder: ResMut<FrameEncoder>, target: Res<FrameTarget>) {
    let pass_desc = wgpu::RenderPassDescriptor {
        label: Some("Clear Pass"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            depth_slice: None,
            view: &target.render_view,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(target.clear_color),
                store: wgpu::StoreOp::Store,
            },
        })],
        depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
            view: &target.depth_stencil_view,
            depth_ops: Some(wgpu::Operations {
                load: wgpu::LoadOp::Clear(1.0),
                store: wgpu::StoreOp::Store,
            }),
            stencil_ops: None,
        }),
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    };
    encoder.0.as_mut().unwrap().begin_render_pass(&pass_desc);
}

fn view_driver(world: &mut World) {
    let mut cameras = world
        .query::<(&SceneOrder, &CameraFrame)>()
        .iter(world)
        .map(|(order, camera)| (order.0, camera.clone()))
        .collect::<Vec<_>>();
    cameras.sort_by_key(|(order, _)| *order);
    assert!(!cameras.is_empty(), "a frame must contain a CameraFrame");

    for (_, camera) in cameras {
        let dimensions = *world.resource::<RenderDimensions>();
        let uniform =
            ViewportUniform::from_camera_frame(&camera, dimensions.width, dimensions.height);
        world.resource_scope(|world, mut viewport: Mut<ViewportGpuPacket>| {
            viewport.update(world.resource::<WgpuContext>(), &uniform);
        });
        world.run_schedule(ViewRender);
    }
}

fn vitem_compute(
    mut encoder: ResMut<FrameEncoder>,
    ctx: Res<WgpuContext>,
    pipelines: Res<PipelinesPool>,
    merged: Res<VItemsBuffer>,
) {
    if merged.item_count() == 0 {
        return;
    }
    let mut pass = encoder
        .0
        .as_mut()
        .unwrap()
        .begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("Merged VItem Map Points Compute Pass"),
            timestamp_writes: None,
        });
    pass.set_pipeline(&pipelines.get_or_init::<VItemComputePipeline>(&ctx));
    pass.set_bind_group(0, merged.compute_bind_group.as_ref().unwrap(), &[]);
    pass.dispatch_workgroups(merged.total_points().div_ceil(256), 1, 1);
}

fn vitem_depth(
    mut encoder: ResMut<FrameEncoder>,
    ctx: Res<WgpuContext>,
    pipelines: Res<PipelinesPool>,
    resolution: Res<ResolutionInfo>,
    target: Res<FrameTarget>,
    viewport: Res<ViewportGpuPacket>,
    merged: Res<VItemsBuffer>,
) {
    if merged.item_count() == 0 {
        return;
    }
    let mut pass = encoder
        .0
        .as_mut()
        .unwrap()
        .begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Merged VItem Depth Render Pass"),
            color_attachments: &[],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &target.depth_stencil_view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
    pass.set_pipeline(&pipelines.get_or_init::<VItemDepthPipeline>(&ctx));
    pass.set_bind_group(0, &resolution.bind_group, &[]);
    pass.set_bind_group(1, &viewport.uniforms_bind_group.bind_group, &[]);
    pass.set_bind_group(2, merged.render_bind_group.as_ref().unwrap(), &[]);
    pass.draw(0..4, 0..merged.item_count());
}

fn mesh_depth(
    mut encoder: ResMut<FrameEncoder>,
    ctx: Res<WgpuContext>,
    pipelines: Res<PipelinesPool>,
    resolution: Res<ResolutionInfo>,
    target: Res<FrameTarget>,
    viewport: Res<ViewportGpuPacket>,
    merged: Res<MeshItemsBuffer>,
) {
    if merged.item_count() == 0 {
        return;
    }
    let mut pass = encoder
        .0
        .as_mut()
        .unwrap()
        .begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Merged MeshItem Depth Render Pass"),
            color_attachments: &[],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &target.depth_stencil_view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
    pass.set_pipeline(&pipelines.get_or_init::<MeshItemDepthPipeline>(&ctx));
    pass.set_bind_group(0, &resolution.bind_group, &[]);
    pass.set_bind_group(1, &viewport.uniforms_bind_group.bind_group, &[]);
    pass.set_bind_group(2, merged.render_bind_group.as_ref().unwrap(), &[]);
    pass.set_vertex_buffer(0, merged.vertices_buffer.buffer.slice(..));
    pass.set_vertex_buffer(1, merged.mesh_ids_buffer.buffer.slice(..));
    pass.set_vertex_buffer(2, merged.vertex_colors_buffer.buffer.slice(..));
    pass.set_vertex_buffer(3, merged.vertex_normals_buffer.buffer.slice(..));
    pass.set_index_buffer(
        merged.indices_buffer.buffer.slice(..),
        wgpu::IndexFormat::Uint32,
    );
    pass.draw_indexed(0..merged.total_indices(), 0, 0..1);
}

fn vitem_color(
    mut encoder: ResMut<FrameEncoder>,
    ctx: Res<WgpuContext>,
    pipelines: Res<PipelinesPool>,
    resolution: Res<ResolutionInfo>,
    target: Res<FrameTarget>,
    viewport: Res<ViewportGpuPacket>,
    merged: Res<VItemsBuffer>,
) {
    if merged.item_count() == 0 {
        return;
    }
    let mut pass = encoder
        .0
        .as_mut()
        .unwrap()
        .begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Merged VItem Color Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &target.render_view,
                resolve_target: None,
                depth_slice: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &target.depth_stencil_view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
    pass.set_pipeline(&pipelines.get_or_init::<VItemColorPipeline>(&ctx));
    pass.set_bind_group(0, &resolution.bind_group, &[]);
    pass.set_bind_group(1, &viewport.uniforms_bind_group.bind_group, &[]);
    pass.set_bind_group(2, merged.render_bind_group.as_ref().unwrap(), &[]);
    pass.draw(0..4, 0..merged.item_count());
}

fn mesh_color(
    mut encoder: ResMut<FrameEncoder>,
    ctx: Res<WgpuContext>,
    pipelines: Res<PipelinesPool>,
    resolution: Res<ResolutionInfo>,
    target: Res<FrameTarget>,
    viewport: Res<ViewportGpuPacket>,
    merged: Res<MeshItemsBuffer>,
) {
    if merged.item_count() == 0 {
        return;
    }
    let mut pass = encoder
        .0
        .as_mut()
        .unwrap()
        .begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Merged MeshItem Color Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &target.render_view,
                resolve_target: None,
                depth_slice: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &target.depth_stencil_view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
    pass.set_pipeline(&pipelines.get_or_init::<MeshItemColorPipeline>(&ctx));
    pass.set_bind_group(0, &resolution.bind_group, &[]);
    pass.set_bind_group(1, &viewport.uniforms_bind_group.bind_group, &[]);
    pass.set_bind_group(2, merged.render_bind_group.as_ref().unwrap(), &[]);
    pass.set_vertex_buffer(0, merged.vertices_buffer.buffer.slice(..));
    pass.set_vertex_buffer(1, merged.mesh_ids_buffer.buffer.slice(..));
    pass.set_vertex_buffer(2, merged.vertex_colors_buffer.buffer.slice(..));
    pass.set_vertex_buffer(3, merged.vertex_normals_buffer.buffer.slice(..));
    pass.set_index_buffer(
        merged.indices_buffer.buffer.slice(..),
        wgpu::IndexFormat::Uint32,
    );
    pass.draw_indexed(0..merged.total_indices(), 0, 0..1);
}

fn oit_resolve(
    mut encoder: ResMut<FrameEncoder>,
    ctx: Res<WgpuContext>,
    pipelines: Res<PipelinesPool>,
    resolution: Res<ResolutionInfo>,
    target: Res<FrameTarget>,
) {
    let mut pass = encoder
        .0
        .as_mut()
        .unwrap()
        .begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("OIT Resolve Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &target.render_view,
                resolve_target: None,
                depth_slice: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
    pass.set_pipeline(&pipelines.get_or_init::<OITResolvePipeline>(&ctx));
    pass.set_bind_group(0, &resolution.bind_group, &[]);
    pass.set_bind_group(1, &target.depth_bind_group, &[]);
    pass.draw(0..3, 0..1);
    drop(pass);
    encoder
        .0
        .as_mut()
        .unwrap()
        .clear_buffer(&resolution.pixel_count_buffer.buffer, 0, None);
}

fn submit(
    ctx: Res<WgpuContext>,
    mut encoder: ResMut<FrameEncoder>,
    #[cfg(feature = "profiling")] mut profiler: ResMut<RenderProfiler>,
) {
    #[allow(unused_mut)]
    let mut encoder = encoder.0.take().expect("frame encoder was not initialized");
    #[cfg(feature = "profiling")]
    profiler.0.resolve_queries(&mut encoder);
    ctx.queue.submit(Some(encoder.finish()));
    #[cfg(feature = "profiling")]
    {
        profiler.0.end_frame().unwrap();
        ctx.device
            .poll(wgpu::PollType::wait_indefinitely())
            .unwrap();
        if let Some(results) = profiler
            .0
            .process_finished_frame(ctx.queue.get_timestamp_period())
        {
            let mut gpu_profiler = crate::PUFFIN_GPU_PROFILER.lock().unwrap();
            wgpu_profiler::puffin::output_frame_to_puffin(&mut gpu_profiler, &results);
            gpu_profiler.new_frame();
        }
    }
}

fn finish_frame(target: Res<FrameTarget>) {
    target.texture_state.mark_dirty();
}
