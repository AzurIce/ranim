//! Rendering stuff in ranim
#![recursion_limit = "256"]
// #![warn(missing_docs)]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![allow(rustdoc::private_intra_doc_links)]
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/AzurIce/ranim/refs/heads/main/assets/ranim.svg",
    html_favicon_url = "https://raw.githubusercontent.com/AzurIce/ranim/refs/heads/main/assets/ranim.svg"
)]
/// Lightweight CPU timing spans, drained per frame by the preview
/// profiler panel.
pub mod cpu_probe;
pub mod extract;
/// The pipelines
pub mod pipelines;
/// The basic renderable structs
pub mod primitives;
pub mod resource;
/// Render schedules and ordered extension points.
pub mod schedule;
mod setup;
/// Upload instrumentation and experimental upload strategies
/// (enabled via `RANIM_PROFILE_UPLOAD`).
pub mod upload_probe;
/// Rendering related utils
pub mod utils;
pub mod world;

use bevy_app::{App, PluginsState, SubApp};
use bevy_derive::AppLabel;
use bevy_ecs::prelude::*;
use glam::{UVec3, uvec3};
use ranim_core::frame::EvaluatedFrame;

pub use setup::RenderSetup;

use crate::{
    primitives::viewport::{ViewportGpuPacket, ViewportUniform},
    resource::{PipelinesPool, RenderTextures},
    schedule::{FrameTarget, RenderDimensions, RenderProfiler, install_schedules},
    utils::{WgpuBuffer, WgpuVecBuffer},
};
use utils::WgpuContext;

pub mod profiling_utils {
    use wgpu_profiler::GpuTimerQueryResult;

    /// Print a `GpuTimerQueryResult` tree (label + duration) to stdout.
    pub fn scopes_to_console_recursive(results: &[GpuTimerQueryResult], indentation: u32) {
        for scope in results {
            if indentation > 0 {
                print!("{:<width$}", "|", width = 4);
            }

            if let Some(time) = &scope.time {
                println!(
                    "{:.3}μs - {}",
                    (time.end - time.start) * 1000.0 * 1000.0,
                    scope.label
                );
            } else {
                println!("n/a - {}", scope.label);
            }

            if !scope.nested_queries.is_empty() {
                scopes_to_console_recursive(&scope.nested_queries, indentation + 1);
            }
        }
    }
}

/// The [`App`] label of a ranim render sub-app.
///
/// Installed by [`install_render_app`]; read it back with
/// `app.get_sub_app(RanimRenderApp)`.
#[derive(AppLabel, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RanimRenderApp;

/// Create an independent render runtime with the shared frame, viewport, and
/// OIT infrastructure installed.
///
/// Item plugins install their own extraction and GPU systems on top.
/// Extraction systems can read the current [`EvaluatedFrame`] resource during
/// [`RenderSchedule::Extract`](crate::schedule::RenderSchedule::Extract).
/// Plugins configure the render world directly; no logic world is created.
pub fn build_render_app(ctx: &WgpuContext, width: u32, height: u32, oit_layers: usize) -> SubApp {
    let mut render_app = SubApp::new();
    render_app
        .insert_resource(ctx.clone())
        .insert_resource(RenderDimensions { width, height })
        .insert_resource(ResolutionInfo::new(ctx, width, height, oit_layers))
        .insert_resource(ViewportGpuPacket::new(ctx, &ViewportUniform::default()))
        .insert_resource(RenderProfiler::new(ctx))
        .init_resource::<PipelinesPool>()
        .init_resource::<EvaluatedFrame>();
    install_schedules(&mut render_app);
    render_app
}

/// Build and install the render sub-app for `app`, with the project's
/// [`RenderSetup`] applied.
///
/// On every [`App::update`], right after the main schedule ran, the sub-app's
/// extract hook swaps the main world's current [`EvaluatedFrame`] into the
/// render world (the render world's previous batch storage moves back, so
/// both sides keep reusing their allocations).
///
/// # Panics
///
/// Panics if a plugin is not ready after installation. This supports
/// synchronous setup using the already initialized GPU context.
pub fn install_render_app(
    app: &mut App,
    ctx: &WgpuContext,
    width: u32,
    height: u32,
    oit_layers: usize,
    setup: RenderSetup,
) {
    let mut render_app = build_render_app(ctx, width, height, oit_layers);
    render_app.add_plugins(setup);
    assert_eq!(
        render_app.plugins_state(),
        PluginsState::Ready,
        "render plugins must be ready after synchronous installation"
    );
    render_app.finish();
    render_app.cleanup();
    render_app.set_extract(|main, render| {
        let Some(mut frame) = main.get_resource_mut::<EvaluatedFrame>() else {
            return;
        };
        std::mem::swap(&mut *frame, &mut *render.resource_mut::<EvaluatedFrame>());
    });
    app.insert_sub_app(RanimRenderApp, render_app);
}

/// Manual driving of a standalone render sub-app built by [`build_render_app`].
pub trait RenderAppExt {
    /// Point the render world at the frame's output target.
    ///
    /// Cheap: the frame target clones the textures' views. Inserted per frame
    /// when the output rotates over multiple readback buffers.
    fn set_frame_target(&mut self, render_textures: &RenderTextures, clear_color: wgpu::Color);

    /// Extract and render one evaluated frame into `render_textures`.
    ///
    /// The frame is temporarily swapped into the render world, then returned
    /// with its input data intact so its batches can be reused or rendered
    /// again. Extraction systems should only read the frame.
    fn render_frame(
        &mut self,
        render_textures: &mut RenderTextures,
        clear_color: wgpu::Color,
        frame: &mut EvaluatedFrame,
    );

    /// Take the GPU timer scopes recorded for the most recent processed
    /// frame (empty unless GPU timers are enabled and the device supports
    /// timestamp queries).
    fn take_last_gpu_scopes(&mut self) -> Option<Vec<wgpu_profiler::GpuTimerQueryResult>>;

    /// Whether the device supports GPU timestamp queries.
    fn gpu_timers_supported(&self) -> bool;

    /// Whether GPU timer scopes are currently being recorded.
    fn gpu_timers_enabled(&self) -> bool;

    /// Enable/disable GPU timer recording at runtime. While enabled, each
    /// frame pays a device poll to read the timers back.
    fn set_gpu_timers_enabled(&self, on: bool);
}

impl RenderAppExt for SubApp {
    fn set_frame_target(&mut self, render_textures: &RenderTextures, clear_color: wgpu::Color) {
        self.insert_resource(FrameTarget::new(render_textures, clear_color));
    }

    fn render_frame(
        &mut self,
        render_textures: &mut RenderTextures,
        clear_color: wgpu::Color,
        frame: &mut EvaluatedFrame,
    ) {
        std::mem::swap(
            frame,
            &mut self.world_mut().resource_mut::<EvaluatedFrame>(),
        );
        self.insert_resource(FrameTarget::new(render_textures, clear_color));
        self.update();
        std::mem::swap(
            frame,
            &mut self.world_mut().resource_mut::<EvaluatedFrame>(),
        );
    }

    fn take_last_gpu_scopes(&mut self) -> Option<Vec<wgpu_profiler::GpuTimerQueryResult>> {
        self.world_mut()
            .resource_mut::<RenderProfiler>()
            .last_frame_scopes
            .take()
    }

    fn gpu_timers_supported(&self) -> bool {
        self.world().resource::<RenderProfiler>().inner.is_some()
    }

    fn gpu_timers_enabled(&self) -> bool {
        self.world().resource::<RenderProfiler>().is_enabled()
    }

    fn set_gpu_timers_enabled(&self, on: bool) {
        self.world().resource::<RenderProfiler>().set_enabled(on);
    }
}

#[allow(unused)]
#[derive(Resource)]
pub struct ResolutionInfo {
    buffer: WgpuBuffer<UVec3>,
    pub(crate) pixel_count_buffer: WgpuVecBuffer<u32>,
    oit_colors_buffer: WgpuVecBuffer<u32>,
    oit_depths_buffer: WgpuVecBuffer<f32>,
    bind_group: wgpu::BindGroup,
}

impl ResolutionInfo {
    /// Shared per-pixel OIT storage bound by item rendering pipelines.
    pub fn bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }

    pub fn new(ctx: &WgpuContext, width: u32, height: u32, oit_layers: usize) -> Self {
        let buffer = WgpuBuffer::new_init(
            ctx,
            Some("ResolutionInfo Buffer"),
            wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            uvec3(width, height, oit_layers as u32),
        );

        let pixel_count = (width * height) as usize;
        let total_nodes = pixel_count * oit_layers;

        let pixel_count_buffer = WgpuVecBuffer::new(
            ctx,
            Some("OIT Pixel Count Buffer"),
            wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC,
            pixel_count,
        );
        let oit_colors_buffer = WgpuVecBuffer::new(
            ctx,
            Some("OIT Colors Buffer"),
            wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_SRC
                | wgpu::BufferUsages::COPY_DST,
            total_nodes,
        );
        let oit_depths_buffer = WgpuVecBuffer::new(
            ctx,
            Some("OIT Depths Buffer"),
            wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_SRC
                | wgpu::BufferUsages::COPY_DST,
            total_nodes,
        );

        let bind_group = ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("ResolutionInfo BindGroup"),
            layout: &Self::create_bind_group_layout(ctx),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: buffer.as_ref().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Buffer(
                        pixel_count_buffer.buffer.as_entire_buffer_binding(),
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Buffer(
                        oit_colors_buffer.buffer.as_entire_buffer_binding(),
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Buffer(
                        oit_depths_buffer.buffer.as_entire_buffer_binding(),
                    ),
                },
            ],
        });

        Self {
            buffer,
            bind_group,
            oit_colors_buffer,
            oit_depths_buffer,
            pixel_count_buffer,
        }
    }
    // This may never be used?
    // pub fn update(&mut self, ctx: &WgpuContext, resolution: UVec2) {
    //     self.buffer.set(ctx, resolution);

    //     let pixel_count = (data.screen_size[0] * data.screen_size[1]) as usize;
    //     let layers = data.oit_layers as usize;
    //     let total_nodes = pixel_count * layers;

    //     let mut bind_group_dirty = false;

    //     if self.pixel_count_buffer.len() != pixel_count {
    //         self.pixel_count_buffer.resize(ctx, pixel_count);
    //         bind_group_dirty = true;
    //     }

    //     if self.oit_colors_buffer.len() != total_nodes {
    //         self.oit_colors_buffer.resize(ctx, total_nodes);
    //         bind_group_dirty = true;
    //     }

    //     if self.oit_depths_buffer.len() != total_nodes {
    //         self.oit_depths_buffer.resize(ctx, total_nodes);
    //         bind_group_dirty = true;
    //     }

    //     if bind_group_dirty {
    //         self.uniforms_bind_group = ViewportBindGroup::new(
    //             ctx,
    //             &self.uniforms_buffer,
    //             &self.pixel_count_buffer,
    //             &self.oit_colors_buffer,
    //             &self.oit_depths_buffer,
    //         );
    //     }
    // }
    pub fn create_bind_group_layout(ctx: &WgpuContext) -> wgpu::BindGroupLayout {
        ctx.device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("ResolutionInfo BindGroupLayout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX_FRAGMENT
                            | wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            })
    }
}
