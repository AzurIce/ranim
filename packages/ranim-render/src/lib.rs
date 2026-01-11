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

use glam::{Mat4, Vec2};
use image::{ImageBuffer, Luma, Rgba};

use crate::{
    graph::{AnyRenderNodeTrait, RenderGraph, RenderPackets},
    primitives::RenderResource,
    resource::{PipelinesPool, RenderPool, RenderTextures},
};
use ranim_core::{core_item::camera_frame::CameraFrame, store::CoreItemStore};
use utils::{WgpuBuffer, WgpuContext, WgpuVecBuffer};

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

// MARK: ViewportUniform
pub type CameraUniforms = ViewportUniform;

/// Uniforms for the camera
#[repr(C, align(16))]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ViewportUniform {
    proj_mat: Mat4,
    view_mat: Mat4,
    half_frame_size: Vec2,
    screen_size: [u32; 2],
    oit_layers: u32,
    _padding: [u32; 3],
}

impl ViewportUniform {
    pub fn from_camera_frame(camera_frame: &CameraFrame, width: u32, height: u32) -> Self {
        let ratio = width as f64 / height as f64;
        Self {
            proj_mat: camera_frame.projection_matrix(ratio).as_mat4(),
            view_mat: camera_frame.view_matrix().as_mat4(),
            half_frame_size: Vec2::new(
                (camera_frame.frame_height * ratio) as f32 / 2.0,
                camera_frame.frame_height as f32 / 2.0,
            ),
            screen_size: [width, height],
            oit_layers: camera_frame.oit_layers,
            _padding: [0; 3],
        }
    }
    pub(crate) fn as_bind_group_layout_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
        wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::COMPUTE | wgpu::ShaderStages::VERTEX_FRAGMENT,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }
    }
}

pub type CameraUniformsBindGroup = ViewportBindGroup;

pub struct ViewportBindGroup {
    pub bind_group: wgpu::BindGroup,
}

impl AsRef<wgpu::BindGroup> for ViewportBindGroup {
    fn as_ref(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }
}

impl ViewportBindGroup {
    pub(crate) fn bind_group_layout(ctx: &WgpuContext) -> wgpu::BindGroupLayout {
        ctx.device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Simple Pipeline Uniforms"),
                entries: &[
                    ViewportUniform::as_bind_group_layout_entry(0),
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

    pub(crate) fn new(
        ctx: &WgpuContext,
        uniforms_buffer: &WgpuBuffer<ViewportUniform>,
        pixel_count_buffer: &WgpuVecBuffer<u32>,
        oit_colors_buffer: &WgpuVecBuffer<u32>,
        oit_depths_buffer: &WgpuVecBuffer<f32>,
    ) -> Self {
        let bind_group = ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Camera Uniforms"),
            layout: &Self::bind_group_layout(ctx),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer(
                        uniforms_buffer.as_ref().as_entire_buffer_binding(),
                    ),
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
        Self { bind_group }
    }
}

pub struct RenderContext<'a> {
    pub render_textures: &'a RenderTextures,
    pub render_pool: &'a RenderPool,
    pub pipelines: &'a mut PipelinesPool,
    pub wgpu_ctx: &'a WgpuContext,
}

// MARK: Renderer
pub struct Renderer {
    resolution: (usize, usize),
    pub(crate) pipelines: PipelinesPool,
    packets: RenderPackets,
    render_graph: RenderGraph,

    pub render_textures: RenderTextures,
    pub viewport: ViewportGpuPacket,

    pub(crate) output_texture_dirty: bool,
    pub(crate) depth_texture_dirty: bool,

    #[cfg(feature = "profiling")]
    pub(crate) profiler: wgpu_profiler::GpuProfiler,
}

pub struct ViewportGpuPacket {
    uniforms_buffer: WgpuBuffer<ViewportUniform>,
    pub pixel_count_buffer: WgpuVecBuffer<u32>,
    pub oit_colors_buffer: WgpuVecBuffer<u32>,
    pub oit_depths_buffer: WgpuVecBuffer<f32>,
    uniforms_bind_group: ViewportBindGroup,
}

impl RenderResource for ViewportGpuPacket {
    type Data = ViewportUniform;

    fn init(ctx: &WgpuContext, data: &Self::Data) -> Self {
        let uniforms_buffer = WgpuBuffer::new_init(
            ctx,
            Some("Uniforms Buffer"),
            wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            *data,
        );

        let pixel_count = (data.screen_size[0] * data.screen_size[1]) as usize;
        let layers = data.oit_layers as usize;
        let total_nodes = pixel_count * layers;

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

        let uniforms_bind_group = ViewportBindGroup::new(
            ctx,
            &uniforms_buffer,
            &pixel_count_buffer,
            &oit_colors_buffer,
            &oit_depths_buffer,
        );
        Self {
            uniforms_buffer,
            pixel_count_buffer,
            oit_colors_buffer,
            oit_depths_buffer,
            uniforms_bind_group,
        }
    }

    fn update(&mut self, ctx: &WgpuContext, data: &Self::Data) {
        self.uniforms_buffer.set(ctx, *data);

        let pixel_count = (data.screen_size[0] * data.screen_size[1]) as usize;
        let layers = data.oit_layers as usize;
        let total_nodes = pixel_count * layers;

        let mut bind_group_dirty = false;

        if self.pixel_count_buffer.len() != pixel_count {
            self.pixel_count_buffer.resize(ctx, pixel_count);
            bind_group_dirty = true;
        }

        if self.oit_colors_buffer.len() != total_nodes {
            self.oit_colors_buffer.resize(ctx, total_nodes);
            bind_group_dirty = true;
        }

        if self.oit_depths_buffer.len() != total_nodes {
            self.oit_depths_buffer.resize(ctx, total_nodes);
            bind_group_dirty = true;
        }

        if bind_group_dirty {
            self.uniforms_bind_group = ViewportBindGroup::new(
                ctx,
                &self.uniforms_buffer,
                &self.pixel_count_buffer,
                &self.oit_colors_buffer,
                &self.oit_depths_buffer,
            );
        }
    }
}

impl Renderer {
    pub fn new(ctx: &WgpuContext, width: usize, height: usize) -> Self {
        let camera = CameraFrame::new();

        let render_textures = RenderTextures::new(ctx, width, height);
        let viewport = ViewportGpuPacket::init(
            ctx,
            &ViewportUniform::from_camera_frame(&camera, width as u32, height as u32),
        );
        // trace!("init renderer uniform: {:?}", uniforms);

        // let bg = rgba8(0x33, 0x33, 0x33, 0xff).convert::<LinearSrgb>();
        // let [r, g, b, a] = bg.components.map(|x| x as f64);
        // let clear_color = wgpu::Color { r, g, b, a };

        #[cfg(feature = "profiling")]
        let profiler = wgpu_profiler::GpuProfiler::new(
            &ctx.device,
            wgpu_profiler::GpuProfilerSettings::default(),
        )
        .unwrap();

        let mut render_graph = RenderGraph::new();
        {
            use graph::*;
            let vitem_compute = render_graph.insert_node(VItemComputeRenderNode);
            let vitem2d_depth = render_graph.insert_node(VItem2dDepthNode);
            let vitem2d_render = render_graph.insert_node(VItem2dRenderNode);
            let vitem_render = render_graph.insert_node(VItemRenderNode);
            let oit_resolve = render_graph.insert_node(OITResolveNode);
            render_graph.insert_edge(vitem_compute, vitem_render);
            render_graph.insert_edge(vitem_compute, vitem2d_depth);
            render_graph.insert_edge(vitem2d_depth, vitem2d_render);
            render_graph.insert_edge(vitem2d_render, oit_resolve);
        }

        Self {
            resolution: (width, height),
            pipelines: PipelinesPool::default(),
            render_textures,
            packets: RenderPackets::default(),
            render_graph,
            // Textures state
            output_texture_dirty: true,
            depth_texture_dirty: true,
            // Camera State
            viewport,
            // Profiler
            #[cfg(feature = "profiling")]
            profiler,
        }
    }

    /// Clears the screen with `Renderer::clear_color`
    pub fn clear_screen(&mut self, ctx: &WgpuContext, clear_color: wgpu::Color) {
        #[cfg(feature = "profiling")]
        profiling::scope!("clear_screen");
        // trace!("clear screen {:?}", self.clear_color);
        let mut encoder = ctx
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Encoder"),
            });

        // Clear
        {
            let RenderTextures {
                render_view,
                // multisample_view,
                depth_stencil_view,
                ..
            } = &self.render_textures;
            let _ = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Clear Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    // view: multisample_view,
                    // resolve_target: Some(render_view),
                    depth_slice: None,
                    view: render_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(clear_color),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: depth_stencil_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                // depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });
        }
        ctx.queue.submit(Some(encoder.finish()));
        self.output_texture_dirty = true;
        self.depth_texture_dirty = true;
    }

    pub fn render_store_with_pool(
        &mut self,
        ctx: &WgpuContext,
        clear_color: wgpu::Color,
        store: &CoreItemStore,
        pool: &mut RenderPool,
    ) {
        // println!("camera: {}, vitems: {}", store.camera_frames.len(), store.vitems.len());
        let (_id, camera_frame) = &store.camera_frames[0];

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

        self.viewport.update(
            ctx,
            &ViewportUniform::from_camera_frame(
                camera_frame,
                self.resolution.0 as u32,
                self.resolution.1 as u32,
            ),
        );

        {
            #[cfg(feature = "profiling")]
            profiling::scope!("render");

            // self.render(ctx, clear_color, &render_primitives);
            self.clear_screen(ctx, clear_color);
            let mut encoder = ctx
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

            {
                #[cfg(feature = "profiling")]
                let mut scope = self.profiler.scope("render", &mut encoder);

                let mut ctx = RenderContext {
                    pipelines: &mut self.pipelines,
                    render_textures: &self.render_textures,
                    render_pool: pool,
                    wgpu_ctx: ctx,
                };

                self.render_graph.exec(
                    #[cfg(not(feature = "profiling"))]
                    &mut encoder,
                    #[cfg(feature = "profiling")]
                    &mut scope,
                    &self.packets,
                    &mut ctx,
                    &self.viewport,
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
        let size = self.resolution;
        let data = self.get_rendered_texture_data(ctx);
        ImageBuffer::from_raw(size.0 as u32, size.1 as u32, data).unwrap()
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
        let size = self.resolution;
        let data = self.get_depth_texture_data(ctx);
        // Map 0.0-1.0 to 0-255
        let u8_data: Vec<u8> = data
            .iter()
            .map(|&d| (d.clamp(0.0, 1.0) * 255.0) as u8)
            .collect();
        ImageBuffer::from_raw(size.0 as u32, size.1 as u32, u8_data).unwrap()
    }
}
