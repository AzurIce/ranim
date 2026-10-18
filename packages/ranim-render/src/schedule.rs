use bevy_app::{First, SubApp};
use bevy_ecs::{prelude::*, schedule::ScheduleLabel, system::SystemParam};
use ranim_core::core_item::camera_frame::CameraFrame;

use crate::{
    pipelines::oit_resolve,
    primitives::viewport::{DEPTH_ORDER_SPAN, ViewportGpuPacket, ViewportUniform},
    resource::{RenderTextureState, RenderTextures},
    utils::WgpuContext,
    world::{CoreItemEntities, SceneOrder},
};

/// Drives one render update: frame maintenance, preparation, and command submission.
#[derive(ScheduleLabel, Debug, Clone, PartialEq, Eq, Hash)]
struct RenderUpdate;

/// Prepares render resources before encoding a frame.
#[derive(ScheduleLabel, Debug, Clone, PartialEq, Eq, Hash)]
pub struct RenderPrepare;

/// Encodes and submits a frame, running [`ViewRender`] for its active view.
#[derive(ScheduleLabel, Debug, Clone, PartialEq, Eq, Hash)]
struct RenderGraph;

/// Runs the ordered compute and draw phases for the active view.
#[derive(ScheduleLabel, Debug, Clone, PartialEq, Eq, Hash)]
pub struct ViewRender;

/// Ordered phases in [`ViewRender`], shared by all render plugins.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum ViewSystems {
    /// Clear the frame's color and depth targets.
    Clear,
    /// Run compute work needed by subsequent draw passes.
    Compute,
    /// Draw all depth prepasses before any color passes.
    Depth,
    /// Draw color and collect transparent fragments.
    Color,
    /// Resolve the accumulated transparent fragments.
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

pub(crate) fn install_schedules(app: &mut SubApp) {
    app.update_schedule = Some(RenderUpdate.intern());
    app.init_resource::<FrameEncoder>()
        .init_schedule(RenderPrepare)
        .add_systems(RenderUpdate, render_update);
    app.configure_sets(
        ViewRender,
        (
            ViewSystems::Clear,
            ViewSystems::Compute,
            ViewSystems::Depth,
            ViewSystems::Color,
            ViewSystems::Resolve,
        )
            .chain(),
    );
    app.add_systems(ViewRender, clear.in_set(ViewSystems::Clear));
    app.add_systems(
        ViewRender,
        oit_resolve::resolve.in_set(ViewSystems::Resolve),
    );

    app.add_systems(
        RenderGraph,
        (begin_frame, view_driver, submit, finish_frame).chain(),
    );
}

fn render_update(world: &mut World) {
    world.run_schedule(First);
    world.run_schedule(RenderPrepare);
    let _span = crate::cpu_probe::span("render_graph");
    world.run_schedule(RenderGraph);
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
    let order_span = world.resource::<CoreItemEntities>().scene_order_span();
    // Omitted render plugins can leave gaps in scene orders. Use the full
    // input range so the total depth offset stays bounded in that case too.
    let bias_epsilon = DEPTH_ORDER_SPAN / order_span.max(1) as f32;
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
    fn render_update_runs_maintenance_and_prepare_before_graph() {
        #[derive(Resource, Default)]
        struct ExecutionLog(Vec<&'static str>);

        let mut app = SubApp::new();
        install_schedules(&mut app);
        app.init_resource::<ExecutionLog>();
        app.add_systems(First, |mut log: ResMut<ExecutionLog>| {
            log.0.push("first");
        });
        app.add_systems(RenderPrepare, |mut log: ResMut<ExecutionLog>| {
            log.0.push("prepare");
        });
        // Replace only GPU execution; retain the installed update and prepare
        // schedules so this exercises their ordering without creating a device.
        app.add_schedule(Schedule::new(RenderGraph));
        app.add_systems(RenderGraph, |mut log: ResMut<ExecutionLog>| {
            log.0.push("graph")
        });

        app.update();
        app.update();
        assert_eq!(
            app.world().resource::<ExecutionLog>().0,
            ["first", "prepare", "graph"].repeat(2)
        );
    }

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
