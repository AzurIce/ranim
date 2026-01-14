//! Rendering stuff in ranim
// #![warn(missing_docs)]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![allow(rustdoc::private_intra_doc_links)]
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/AzurIce/ranim/refs/heads/main/assets/ranim.svg",
    html_favicon_url = "https://raw.githubusercontent.com/AzurIce/ranim/refs/heads/main/assets/ranim.svg"
)]
/// Render Graph
pub mod graph;
/// The pipelines
pub mod pipelines;
/// The basic renderable structs
pub mod primitives;
pub mod resource;
/// Rendering related utils
pub mod utils;

use glam::{UVec3, uvec3};
use image::{ImageBuffer, Luma, Rgba};

use crate::{
    graph::{AnyGlobalRenderNodeTrait, GlobalRenderGraph, RenderPackets},
    primitives::viewport::ViewportUniform,
    resource::{PipelinesPool, RenderPool, RenderTextures},
    utils::{WgpuBuffer, WgpuVecBuffer},
};
use ranim_core::store::CoreItemStore;
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

#[derive(Clone, Copy)]
pub struct RenderContext<'a> {
    pub render_textures: &'a RenderTextures,
    pub render_pool: &'a RenderPool,
    pub render_packets: &'a RenderPackets,
    pub pipelines: &'a PipelinesPool,
    pub wgpu_ctx: &'a WgpuContext,
    pub resolution_info: &'a ResolutionInfo,
    pub clear_color: wgpu::Color,
}

// MARK: Renderer
pub struct Renderer {
    width: u32,
    height: u32,
    resolution_info: ResolutionInfo,
    pub(crate) pipelines: PipelinesPool,
    packets: RenderPackets,
    render_graph: GlobalRenderGraph,

    pub render_textures: RenderTextures,

    pub(crate) output_texture_dirty: bool,
    pub(crate) depth_texture_dirty: bool,

    #[cfg(feature = "profiling")]
    pub(crate) profiler: wgpu_profiler::GpuProfiler,
}

impl Renderer {
    pub fn new(ctx: &WgpuContext, width: u32, height: u32, oit_layers: usize) -> Self {
        let resolution_info = ResolutionInfo::new(ctx, width, height, oit_layers);
        let render_textures = RenderTextures::new(ctx, width, height);

        // let viewport = ViewportGpuPacket::init(
        //     ctx,
        //     &ViewportUniform::from_camera_frame(&camera, width as u32, height as u32),
        // );

        #[cfg(feature = "profiling")]
        let profiler = wgpu_profiler::GpuProfiler::new(
            &ctx.device,
            wgpu_profiler::GpuProfilerSettings::default(),
        )
        .unwrap();

        let mut render_graph = GlobalRenderGraph::new();
        {
            use graph::*;
            // Global Render Nodes that executes per-frame
            let clear = render_graph.insert_node(ClearNode);
            let view_render = render_graph.insert_node({
                use graph::view::*;
                // View Render Nodes that executes per-viewport in every frame
                let mut render_graph = ViewRenderGraph::new();
                let vitem_compute = render_graph.insert_node(VItemComputeRenderNode);
                let vitem2d_depth = render_graph.insert_node(VItem2dDepthNode);
                let vitem2d_render = render_graph.insert_node(VItem2dColorNode);
                let vitem_render = render_graph.insert_node(VItemRenderNode);
                let oit_resolve = render_graph.insert_node(OITResolveNode);
                render_graph.insert_edge(vitem_compute, vitem_render);
                render_graph.insert_edge(vitem_compute, vitem2d_depth);

                render_graph.insert_edge(vitem2d_depth, vitem2d_render);
                render_graph.insert_edge(vitem2d_render, oit_resolve);
                render_graph
            });
            render_graph.insert_edge(clear, view_render);
        }

        Self {
            width,
            height,
            resolution_info,
            pipelines: PipelinesPool::default(),
            render_textures,
            packets: RenderPackets::default(),
            render_graph,
            // Textures state
            output_texture_dirty: true,
            depth_texture_dirty: true,
            // Profiler
            #[cfg(feature = "profiling")]
            profiler,
        }
    }

    pub fn render_store_with_pool(
        &mut self,
        ctx: &WgpuContext,
        clear_color: wgpu::Color,
        store: &CoreItemStore,
        pool: &mut RenderPool,
    ) {
        let (_id, camera_frame) = &store.camera_frames[0];
        let viewport = ViewportUniform::from_camera_frame(camera_frame, self.width, self.height);

        self.packets.push(pool.alloc_packet(ctx, &viewport));
        self.packets.extend(
            store
                .vitems
                .iter()
                .map(|(_id, data)| pool.alloc_packet(ctx, data)),
        );
        self.packets.extend(
            store
                .vitems2d
                .iter()
                .map(|(_id, data)| pool.alloc_packet(ctx, data)),
        );

        {
            #[cfg(feature = "profiling")]
            profiling::scope!("render");

            let mut encoder = ctx
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

            {
                #[cfg(feature = "profiling")]
                let mut scope = self.profiler.scope("render", &mut encoder);

                let ctx = RenderContext {
                    pipelines: &self.pipelines,
                    render_textures: &self.render_textures,
                    render_packets: &self.packets,
                    render_pool: pool,
                    wgpu_ctx: ctx,
                    resolution_info: &self.resolution_info,
                    clear_color,
                };

                self.render_graph.exec(
                    #[cfg(not(feature = "profiling"))]
                    &mut encoder,
                    #[cfg(feature = "profiling")]
                    &mut scope,
                    ctx,
                );
            }

            #[cfg(not(feature = "profiling"))]
            ctx.queue.submit(Some(encoder.finish()));

            #[cfg(feature = "profiling")]
            {
                self.profiler.resolve_queries(&mut encoder);
                {
                    profiling::scope!("submit");
                    ctx.queue.submit(Some(encoder.finish()));
                }

                // renderable.debug(ctx);

                // Signal to the profiler that the frame is finished.
                self.profiler.end_frame().unwrap();

                // Query for oldest finished frame (this is almost certainly not the one we just submitted!) and display results in the command line.
                ctx.device
                    .poll(wgpu::PollType::wait_indefinitely())
                    .unwrap();
                let latest_profiler_results = self
                    .profiler
                    .process_finished_frame(ctx.queue.get_timestamp_period());
                // profiling_utils::console_output(&latest_profiler_results, ctx.wgpu_ctx.device.features());
                let mut gpu_profiler = PUFFIN_GPU_PROFILER.lock().unwrap();
                wgpu_profiler::puffin::output_frame_to_puffin(
                    &mut gpu_profiler,
                    &latest_profiler_results.unwrap(),
                );
                gpu_profiler.new_frame();
            }

            self.output_texture_dirty = true;
            self.depth_texture_dirty = true;
        }

        self.packets.clear();
        // drop(render_primitives);
    }

    pub fn get_rendered_texture_data(&mut self, ctx: &WgpuContext) -> &[u8] {
        if !self.output_texture_dirty {
            // trace!("[Camera] Updating rendered texture data...");
            return self.render_textures.render_texture.texture_data();
        }
        self.output_texture_dirty = false;
        self.render_textures.render_texture.update_texture_data(ctx)
    }
    pub fn get_rendered_texture_img_buffer(
        &mut self,
        ctx: &WgpuContext,
    ) -> ImageBuffer<Rgba<u8>, &[u8]> {
        ImageBuffer::from_raw(self.width, self.height, self.get_rendered_texture_data(ctx)).unwrap()
    }

    pub fn get_depth_texture_data(&mut self, ctx: &WgpuContext) -> &[f32] {
        if !self.depth_texture_dirty {
            return bytemuck::cast_slice(self.render_textures.depth_stencil_texture.texture_data());
        }
        self.depth_texture_dirty = false;
        bytemuck::cast_slice(
            self.render_textures
                .depth_stencil_texture
                .update_texture_data(ctx),
        )
    }

    pub fn get_depth_texture_img_buffer(
        &mut self,
        ctx: &WgpuContext,
    ) -> ImageBuffer<Luma<u8>, Vec<u8>> {
        let data = self
            .get_depth_texture_data(ctx)
            .iter()
            // Map 0.0-1.0 to 0-255
            .map(|&d| (d.clamp(0.0, 1.0) * 255.0) as u8)
            .collect::<Vec<_>>();
        ImageBuffer::from_raw(self.width, self.height, data).unwrap()
    }
}

#[allow(unused)]
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
