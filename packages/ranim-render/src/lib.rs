//! Rendering stuff in ranim
// #![warn(missing_docs)]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![allow(rustdoc::private_intra_doc_links)]
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/AzurIce/ranim/refs/heads/main/assets/ranim.svg",
    html_favicon_url = "https://raw.githubusercontent.com/AzurIce/ranim/refs/heads/main/assets/ranim.svg"
)]
/// The pipelines
pub mod pipelines;
/// The basic renderable structs
pub mod primitives;
pub mod resource;
mod schedule;
/// Rendering related utils
pub mod utils;
pub mod world;

use bevy_ecs::prelude::*;
use glam::{UVec3, uvec3};

use crate::{
    primitives::{mesh_items::MeshItemsBuffer, viewport::ViewportUniform, vitems::VItemsBuffer},
    resource::{PipelinesPool, RenderTextures},
    schedule::{FrameTarget, RenderDimensions, RenderGraph, RenderPrepare, install_schedules},
    utils::{WgpuBuffer, WgpuVecBuffer},
    world::{CoreItemEntities, RenderFrame, reconcile},
};
use utils::WgpuContext;

#[cfg(feature = "profiling")]
// Since the timing information we get from WGPU may be several frames behind the CPU, we can't report these frames to
// the singleton returned by `puffin::GlobalProfiler::lock`. Instead, we need our own `puffin::GlobalProfiler` that we
// can be several frames behind puffin's main global profiler singleton.
pub static PUFFIN_GPU_PROFILER: std::sync::LazyLock<std::sync::Mutex<puffin::GlobalProfiler>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(puffin::GlobalProfiler::default()));

#[allow(unused)]
#[cfg(feature = "profiling")]
mod profiling_utils {
    use wgpu_profiler::GpuTimerQueryResult;

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

    pub fn console_output(
        results: &Option<Vec<GpuTimerQueryResult>>,
        enabled_features: wgpu::Features,
    ) {
        puffin::profile_scope!("console_output");
        print!("\x1B[2J\x1B[1;1H"); // Clear terminal and put cursor to first row first column
        println!("Welcome to wgpu_profiler demo!");
        println!();
        println!(
            "Press space to write out a trace file that can be viewed in chrome's chrome://tracing"
        );
        println!();
        match results {
            Some(results) => {
                scopes_to_console_recursive(results, 0);
            }
            None => println!("No profiling results available yet!"),
        }
    }
}

// MARK: Renderer
pub struct Renderer {
    width: u32,
    height: u32,
    world: World,
}

impl Renderer {
    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn ratio(&self) -> f32 {
        self.width as f32 / self.height as f32
    }

    pub fn new(ctx: &WgpuContext, width: u32, height: u32, oit_layers: usize) -> Self {
        let mut world = World::new();
        world.insert_resource(ctx.clone());
        world.insert_resource(RenderDimensions { width, height });
        world.insert_resource(ResolutionInfo::new(ctx, width, height, oit_layers));
        world.init_resource::<PipelinesPool>();
        world.insert_resource(VItemsBuffer::new(ctx));
        world.insert_resource(MeshItemsBuffer::new(ctx));
        world.insert_resource(primitives::viewport::ViewportGpuPacket::new(
            ctx,
            &ViewportUniform::from_camera_frame(&Default::default(), width, height),
        ));
        world.init_resource::<CoreItemEntities>();

        #[cfg(feature = "profiling")]
        world.insert_resource(schedule::RenderProfiler(
            wgpu_profiler::GpuProfiler::new(
                &ctx.device,
                wgpu_profiler::GpuProfilerSettings::default(),
            )
            .unwrap(),
        ));
        install_schedules(&mut world);

        Self {
            width,
            height,
            world,
        }
    }

    pub fn new_render_textures(&self, ctx: &WgpuContext) -> RenderTextures {
        RenderTextures::new(ctx, self.width, self.height)
    }

    /// Reconcile and render one evaluated frame.
    pub fn render_frame(
        &mut self,
        render_textures: &mut RenderTextures,
        clear_color: wgpu::Color,
        frame: &RenderFrame,
    ) {
        reconcile(&mut self.world, frame);
        self.world
            .insert_resource(FrameTarget::new(render_textures, clear_color));
        self.world.run_schedule(RenderPrepare);
        self.world.run_schedule(RenderGraph);
    }

    /// Reconcile directly from a `LogicWorld` and render one frame (M2 direct
    /// extraction): same render schedules as [`render_frame`](Self::render_frame),
    /// but the extracted items move out of the logic world instead of being
    /// cloned through a transport frame.
    pub fn render_logic_frame(
        &mut self,
        render_textures: &mut RenderTextures,
        clear_color: wgpu::Color,
        logic: &mut bevy_ecs::world::World,
    ) {
        crate::world::reconcile_logic(&mut self.world, logic);
        self.world
            .insert_resource(FrameTarget::new(render_textures, clear_color));
        self.world.run_schedule(RenderPrepare);
        self.world.run_schedule(RenderGraph);
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
