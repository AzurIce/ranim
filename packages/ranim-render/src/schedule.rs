use bevy_app::{First, SubApp};
use bevy_ecs::{
    prelude::*,
    schedule::{InternedScheduleLabel, InternedSystemSet, ScheduleLabel},
    system::SystemParam,
};

use crate::{
    pipelines::oit_resolve,
    resource::{RenderTextureState, RenderTextures},
    utils::WgpuContext,
    world::resolve_scene_order,
};

/// Drives one render update: maintenance, extraction, preparation, and submission.
#[derive(ScheduleLabel, Debug, Clone, PartialEq, Eq, Hash)]
struct RenderUpdate;

/// Encodes and submits a frame, running the active view's schedule.
#[derive(ScheduleLabel, Debug, Clone, PartialEq, Eq, Hash)]
struct RenderGraph;

// These values are interned only by the render runtime that creates the handles.
// Bevy's interned identity includes the address, so independently interning the
// same non-zero-sized label in a dylib would create a different schedule or set.
#[derive(ScheduleLabel, Debug, Clone, PartialEq, Eq, Hash)]
enum RenderSchedule {
    Extract,
    Prepare,
    View,
}

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
enum ViewPhase {
    Clear,
    Compute,
    Depth,
    Color,
    Resolve,
}

/// Canonical handles for the ordered phases of [`RenderSchedules::view`].
///
/// Plugins copy these handles from the runtime's [`RenderSchedules`] resource
/// instead of interning their own labels across dynamic library boundaries.
#[derive(Clone, Copy)]
pub struct ViewSystems {
    /// Clear the frame's color and depth targets.
    pub clear: InternedSystemSet,
    /// Run compute work needed by subsequent draw passes.
    pub compute: InternedSystemSet,
    /// Draw all depth prepasses before any color passes.
    pub depth: InternedSystemSet,
    /// Draw color and collect transparent fragments.
    pub color: InternedSystemSet,
    /// Resolve the accumulated transparent fragments.
    pub resolve: InternedSystemSet,
}

/// Shared schedule and phase identities created by the render runtime.
///
/// [`Renderer::new`](crate::Renderer::new) installs this resource before plugins
/// run. Plugins read it to register systems into the same schedules and sets,
/// including when their code comes from a dynamic library. A standalone CPU
/// render world can initialize it with [`Default`]. Shared SDK build conditions
/// still apply; independently interned custom labels do not acquire this identity.
#[derive(Resource, Clone, Copy)]
pub struct RenderSchedules {
    /// Extract the current [`EvaluatedFrame`](ranim_core::frame::EvaluatedFrame).
    /// Systems read complete typed batches and update render components or resources.
    pub extract: InternedScheduleLabel,
    /// Prepare and upload render resources after extraction and scene ordering.
    pub prepare: InternedScheduleLabel,
    /// Run the active view's ordered compute and draw phases.
    pub view: InternedScheduleLabel,
    /// Ordered phases shared by all plugins in [`Self::view`].
    pub view_systems: ViewSystems,
}

impl Default for RenderSchedules {
    fn default() -> Self {
        Self {
            extract: RenderSchedule::Extract.intern(),
            prepare: RenderSchedule::Prepare.intern(),
            view: RenderSchedule::View.intern(),
            view_systems: ViewSystems {
                clear: ViewPhase::Clear.intern(),
                compute: ViewPhase::Compute.intern(),
                depth: ViewPhase::Depth.intern(),
                color: ViewPhase::Color.intern(),
                resolve: ViewPhase::Resolve.intern(),
            },
        }
    }
}

#[derive(Resource, Clone, Copy)]
pub struct RenderDimensions {
    pub width: u32,
    pub height: u32,
}

#[derive(Resource)]
pub struct FrameTarget {
    pub(crate) render_view: wgpu::TextureView,
    pub(crate) depth_stencil_view: wgpu::TextureView,
    pub(crate) depth_bind_group: wgpu::BindGroup,
    clear_color: wgpu::Color,
    texture_state: RenderTextureState,
}

impl FrameTarget {
    /// Color attachment for this frame's draw passes.
    pub fn render_view(&self) -> &wgpu::TextureView {
        &self.render_view
    }

    /// Depth attachment shared by the frame's depth and color passes.
    pub fn depth_stencil_view(&self) -> &wgpu::TextureView {
        &self.depth_stencil_view
    }

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
pub struct RenderContext<'w> {
    encoder: ResMut<'w, FrameEncoder>,
}

impl RenderContext<'_> {
    pub fn encoder(&mut self) -> &mut wgpu::CommandEncoder {
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
pub struct RenderProfiler {
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
    pub fn scope_pass<R, T>(&self, label: &str, pass: &mut R, f: impl FnOnce(&mut R) -> T) -> T
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
    app.init_resource::<RenderSchedules>();
    let schedules = *app.world().resource::<RenderSchedules>();
    let phases = schedules.view_systems;
    app.update_schedule = Some(RenderUpdate.intern());
    app.init_resource::<FrameEncoder>()
        .init_schedule(schedules.extract)
        .init_schedule(schedules.prepare)
        .add_systems(RenderUpdate, render_update);
    app.configure_sets(
        schedules.view,
        (
            phases.clear,
            phases.compute,
            phases.depth,
            phases.color,
            phases.resolve,
        )
            .chain(),
    );
    app.add_systems(schedules.view, clear.in_set(phases.clear));
    app.add_systems(schedules.view, oit_resolve::resolve.in_set(phases.resolve));

    app.add_systems(
        RenderGraph,
        (begin_frame, view_driver, submit, finish_frame).chain(),
    );
}

fn render_update(world: &mut World) {
    let schedules = *world.resource::<RenderSchedules>();
    world.run_schedule(First);
    world.run_schedule(schedules.extract);
    resolve_scene_order(world);
    world.run_schedule(schedules.prepare);
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
    let view = world.resource::<RenderSchedules>().view;
    world.run_schedule(view);
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
    use bevy_ecs::intern::Interner;

    use super::*;

    #[test]
    fn canonical_handles_keep_identity_without_reinterning_in_another_pool() {
        let schedules = RenderSchedules::default();
        let other_schedules = Interner::<dyn ScheduleLabel>::new();
        for (label, handle) in [
            (RenderSchedule::Extract, schedules.extract),
            (RenderSchedule::Prepare, schedules.prepare),
            (RenderSchedule::View, schedules.view),
        ] {
            // A second runtime can have matching TypeIds but its own interner.
            assert_ne!(other_schedules.intern(&label), handle);
            // Registration with the supplied handle bypasses that local pool.
            assert_eq!(handle.intern(), handle);
        }

        let phases = schedules.view_systems;
        let other_sets = Interner::<dyn SystemSet>::new();
        for (label, handle) in [
            (ViewPhase::Clear, phases.clear),
            (ViewPhase::Compute, phases.compute),
            (ViewPhase::Depth, phases.depth),
            (ViewPhase::Color, phases.color),
            (ViewPhase::Resolve, phases.resolve),
        ] {
            assert_ne!(other_sets.intern(&label), handle);
            assert_eq!(handle.intern(), handle);
        }
        assert_eq!(
            [
                phases.clear,
                phases.compute,
                phases.depth,
                phases.color,
                phases.resolve
            ]
            .into_iter()
            .collect::<std::collections::HashSet<_>>()
            .len(),
            5
        );
    }

    #[test]
    fn copied_phase_handles_enforce_the_shared_view_order() {
        #[derive(Resource, Default)]
        struct ExecutionLog(Vec<&'static str>);

        let mut app = SubApp::new();
        app.init_resource::<RenderSchedules>()
            .init_resource::<ExecutionLog>();
        let schedules = *app.world().resource::<RenderSchedules>();
        let phases = schedules.view_systems;
        app.update_schedule = Some(schedules.view);
        app.configure_sets(
            schedules.view,
            (
                phases.clear,
                phases.compute,
                phases.depth,
                phases.color,
                phases.resolve,
            )
                .chain(),
        );
        // Deliberately register in reverse order, using the same copied handles
        // that a separately loaded plugin receives from the runtime resource.
        app.add_systems(
            schedules.view,
            (
                (|mut log: ResMut<ExecutionLog>| log.0.push("resolve")).in_set(phases.resolve),
                (|mut log: ResMut<ExecutionLog>| log.0.push("color")).in_set(phases.color),
                (|mut log: ResMut<ExecutionLog>| log.0.push("depth")).in_set(phases.depth),
                (|mut log: ResMut<ExecutionLog>| log.0.push("compute")).in_set(phases.compute),
                (|mut log: ResMut<ExecutionLog>| log.0.push("clear")).in_set(phases.clear),
            ),
        );
        app.update();
        assert_eq!(
            app.world().resource::<ExecutionLog>().0,
            ["clear", "compute", "depth", "color", "resolve"]
        );
    }

    #[test]
    fn render_update_runs_extraction_before_preparation_and_graph() {
        #[derive(Resource, Default)]
        struct ExecutionLog(Vec<&'static str>);

        let mut app = SubApp::new();
        install_schedules(&mut app);
        let schedules = *app.world().resource::<RenderSchedules>();
        app.init_resource::<ExecutionLog>();
        app.add_systems(First, |mut log: ResMut<ExecutionLog>| {
            log.0.push("first");
        });
        app.add_systems(schedules.extract, |mut log: ResMut<ExecutionLog>| {
            log.0.push("extract");
        });
        app.add_systems(schedules.prepare, |mut log: ResMut<ExecutionLog>| {
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
            ["first", "extract", "prepare", "graph"].repeat(2)
        );
    }
}
