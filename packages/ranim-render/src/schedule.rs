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

/// Always-present render profiler resource. GPU timer scopes are
/// **runtime-toggled** (`RANIM_PROFILE_GPU=1` at startup or
/// [`RenderProfiler::set_enabled`]): when off (the default) scope wrapping
/// is a pass-through and no queries are resolved or polled, so profiling
/// costs nothing; when on, each processed frame's scope tree is stored in
/// `last_frame_scopes`. `inner` is `None` when the device lacks timestamp
/// query support.
#[derive(Resource)]
pub(crate) struct RenderProfiler {
    pub(crate) inner: Option<wgpu_profiler::GpuProfiler>,
    enabled: std::sync::atomic::AtomicBool,
    /// GPU timer scopes of the most recent processed frame.
    pub(crate) last_frame_scopes: Option<Vec<wgpu_profiler::GpuTimerQueryResult>>,
}

impl RenderProfiler {
    pub(crate) fn new(_ctx: &WgpuContext) -> Self {
        Self {
            inner: wgpu_profiler::GpuProfiler::new(
                &_ctx.device,
                wgpu_profiler::GpuProfilerSettings::default(),
            )
            .ok(),
            enabled: std::sync::atomic::AtomicBool::new(
                std::env::var("RANIM_PROFILE_GPU").is_ok_and(|v| v != "0" && !v.is_empty()),
            ),
            last_frame_scopes: None,
        }
    }

    pub(crate) fn is_enabled(&self) -> bool {
        self.enabled.load(std::sync::atomic::Ordering::Relaxed)
    }

    pub(crate) fn set_enabled(&self, on: bool) {
        self.enabled.store(on, std::sync::atomic::Ordering::Relaxed);
    }

    /// Run `f` with `pass` wrapped in a GPU timer scope labeled `label`.
    /// Pass-through while GPU timers are disabled or unsupported.
    pub(crate) fn scope_pass<R, T>(
        &self,
        label: &str,
        pass: &mut R,
        f: impl FnOnce(&mut R) -> T,
    ) -> T
    where
        R: wgpu_profiler::ProfilerCommandRecorder,
    {
        if !self.is_enabled() {
            return f(pass);
        }
        let Some(profiler) = self.inner.as_ref() else {
            return f(pass);
        };
        let mut scope = profiler.scope(label.to_string(), pass);
        f(&mut *scope)
    }
}

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
    view.add_systems(
        (vitem::depth, mesh_item::depth)
            .chain()
            .in_set(ViewSystems::Depth),
    );
    view.add_systems(
        (vitem::color, mesh_item::color)
            .chain()
            .in_set(ViewSystems::Color),
    );
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
    let _span = crate::cpu_probe::span("prepare_vitems");
    let mut items = items.iter().collect::<Vec<_>>();
    items.sort_by_key(|(order, _)| order.0);
    buffer.update(
        &ctx,
        items.iter().map(|&(order, item)| (order.0 as f32, item)),
    );
}

fn prepare_mesh_items(
    ctx: Res<WgpuContext>,
    mut buffer: ResMut<MeshItemsBuffer>,
    items: Query<(&SceneOrder, &MeshItem)>,
) {
    let _span = crate::cpu_probe::span("prepare_mesh_items");
    let mut items = items.iter().collect::<Vec<_>>();
    items.sort_by_key(|(order, _)| order.0);
    buffer.update(
        &ctx,
        items.iter().map(|&(order, item)| (order.0 as f32, item)),
    );
}

fn begin_frame(ctx: Res<WgpuContext>, mut encoder: ResMut<FrameEncoder>) {
    encoder.0 = Some(
        ctx.device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default()),
    );
}

fn clear(mut render: RenderContext, target: Res<FrameTarget>, profiler: Res<RenderProfiler>) {
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
    let encoder = render.encoder();
    let mut pass = encoder.begin_render_pass(&pass_desc);
    profiler.scope_pass("clear", &mut pass, |_| ());
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
    // The per-order depth bias epsilon: the fixed span is split evenly across
    // all items of the frame so the total offset stays bounded.
    let item_count = world.resource::<VItemsBuffer>().item_count()
        + world.resource::<MeshItemsBuffer>().item_count();
    let bias_epsilon = crate::primitives::viewport::DEPTH_ORDER_SPAN / item_count.max(1) as f32;
    let uniform = ViewportUniform::from_camera_frame(
        &camera,
        dimensions.width,
        dimensions.height,
        bias_epsilon,
    );
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
    mut profiler: ResMut<RenderProfiler>,
) {
    let mut encoder = encoder.0.take().expect("frame encoder was not initialized");
    if profiler.is_enabled()
        && let Some(inner) = profiler.inner.as_mut()
    {
        inner.resolve_queries(&mut encoder);
    }
    ctx.queue.submit(Some(encoder.finish()));
}

fn finish_frame(
    target: Res<FrameTarget>,
    ctx: Res<WgpuContext>,
    mut profiler: ResMut<RenderProfiler>,
) {
    // Processing timer queries forces a device poll (a sync point), so it
    // only happens while GPU timers are explicitly enabled.
    if profiler.is_enabled()
        && let Some(inner) = profiler.inner.as_mut()
        && inner.end_frame().is_ok()
    {
        ctx.device
            .poll(wgpu::PollType::wait_indefinitely())
            .unwrap();
        if let Some(results) = inner.process_finished_frame(ctx.queue.get_timestamp_period()) {
            profiler.last_frame_scopes = Some(results);
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
