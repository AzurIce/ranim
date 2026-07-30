use bevy_ecs::{prelude::*, schedule::ScheduleLabel, system::SystemParam};
use ranim_core::core_item::{camera_frame::CameraFrame, mesh_item::MeshItem, vitem::VItem};

use crate::{
    pipelines::{mesh_item, oit_resolve, vitem},
    primitives::{
        mesh_items::MeshItemsBuffer,
        viewport::{ViewportGpuPacket, ViewportUniform},
        vitems::VItemsBuffer,
    },
    resource::{RenderTextureState, RenderTextures},
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
    Clear,
    Compute,
    Depth,
    Color,
    Resolve,
}

#[derive(Resource, Clone, Copy)]
pub(crate) struct RenderDimensions {
    pub width: u32,
    pub height: u32,
}

#[derive(Resource)]
pub(crate) struct FrameTarget {
    pub(crate) render_view: wgpu::TextureView,
    pub(crate) depth_stencil_view: wgpu::TextureView,
    pub(crate) depth_bind_group: wgpu::BindGroup,
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

#[derive(SystemParam)]
pub(crate) struct RenderContext<'w> {
    encoder: ResMut<'w, FrameEncoder>,
}

impl RenderContext<'_> {
    pub(crate) fn encoder(&mut self) -> &mut wgpu::CommandEncoder {
        self.encoder
            .0
            .as_mut()
            .expect("frame encoder was not initialized")
    }
}

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
    view.configure_sets(
        (
            ViewSystems::Clear,
            ViewSystems::Compute,
            ViewSystems::Depth,
            ViewSystems::Color,
            ViewSystems::Resolve,
        )
            .chain(),
    );
    view.add_systems(clear.in_set(ViewSystems::Clear));
    view.add_systems(vitem::compute.in_set(ViewSystems::Compute));
    view.add_systems((vitem::depth, mesh_item::depth).in_set(ViewSystems::Depth));
    view.add_systems((vitem::color, mesh_item::color).in_set(ViewSystems::Color));
    view.add_systems(oit_resolve::resolve.in_set(ViewSystems::Resolve));
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
    graph.add_systems(view_driver.in_set(RenderGraphSystems::Render));
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

fn clear(mut render: RenderContext, target: Res<FrameTarget>) {
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
    render.encoder().begin_render_pass(&pass_desc);
}

fn view_driver(world: &mut World) {
    let mut cameras = world
        .query::<(&SceneOrder, &CameraFrame)>()
        .iter(world)
        .map(|(order, camera)| (order.0, camera.clone()))
        .collect::<Vec<_>>();
    cameras.sort_by_key(|(order, _)| *order);

    let camera = take_single_camera(cameras);
    let dimensions = *world.resource::<RenderDimensions>();
    let uniform = ViewportUniform::from_camera_frame(&camera, dimensions.width, dimensions.height);
    world.resource_scope(|world, mut viewport: Mut<ViewportGpuPacket>| {
        viewport.update(world.resource::<WgpuContext>(), &uniform);
    });
    world.run_schedule(ViewRender);
}

fn take_single_camera(mut cameras: Vec<(usize, CameraFrame)>) -> CameraFrame {
    assert_eq!(
        cameras.len(),
        1,
        "D0002 requires exactly one active CameraFrame per rendered frame"
    );
    cameras.pop().unwrap().1
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
}

fn finish_frame(
    target: Res<FrameTarget>,
    #[cfg(feature = "profiling")] ctx: Res<WgpuContext>,
    #[cfg(feature = "profiling")] mut profiler: ResMut<RenderProfiler>,
) {
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
    target.texture_state.mark_dirty();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_camera_is_accepted() {
        let camera = CameraFrame::default();

        assert_eq!(take_single_camera(vec![(7, camera.clone())]), camera);
    }

    #[test]
    #[should_panic(expected = "D0002 requires exactly one active CameraFrame")]
    fn missing_camera_is_rejected() {
        take_single_camera(Vec::new());
    }

    #[test]
    #[should_panic(expected = "D0002 requires exactly one active CameraFrame")]
    fn multiple_cameras_are_rejected() {
        take_single_camera(vec![
            (0, CameraFrame::default()),
            (1, CameraFrame::default()),
        ]);
    }
}
