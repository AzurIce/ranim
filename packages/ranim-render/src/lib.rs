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
/// Rendering related utils
pub mod utils;

use glam::{Mat4, Vec2};
use image::{ImageBuffer, Luma, Rgba};

use crate::{
    graph::{AnyRenderNodeTrait, RenderGraph, RenderPackets},
    primitives::{
        RenderPool, VItem2dDepthNode, VItem2dRenderNode, VItemComputeRenderNode, VItemRenderNode,
    },
    utils::RenderContext,
};
use ranim_core::{core_item::camera_frame::CameraFrame, store::CoreItemStore};
use utils::{PipelinesStorage, WgpuBuffer, WgpuContext};

pub(crate) const OUTPUT_TEXTURE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;
const ALIGNMENT: usize = 256;

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

// MARK: CameraUniforms
/// Uniforms for the camera
#[repr(C, align(16))]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniforms {
    proj_mat: Mat4,
    view_mat: Mat4,
    half_frame_size: Vec2,
    _padding: [f32; 2],
}

impl CameraUniforms {
    pub fn from_camera_frame(camera_frame: &CameraFrame, frame_height: f64, ratio: f64) -> Self {
        Self {
            proj_mat: camera_frame
                .projection_matrix(frame_height, ratio)
                .as_mat4(),
            view_mat: camera_frame.view_matrix().as_mat4(),
            half_frame_size: Vec2::new(
                (frame_height * ratio) as f32 / 2.0,
                frame_height as f32 / 2.0,
            ),
            _padding: [0.0; 2],
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

pub(crate) struct CameraUniformsBindGroup {
    pub(crate) bind_group: wgpu::BindGroup,
}

impl AsRef<wgpu::BindGroup> for CameraUniformsBindGroup {
    fn as_ref(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }
}

impl CameraUniformsBindGroup {
    pub(crate) fn bind_group_layout(ctx: &WgpuContext) -> wgpu::BindGroupLayout {
        ctx.device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Simple Pipeline Uniforms"),
                entries: &[CameraUniforms::as_bind_group_layout_entry(0)],
            })
    }

    pub(crate) fn new(ctx: &WgpuContext, uniforms_buffer: &WgpuBuffer<CameraUniforms>) -> Self {
        let bind_group = ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Camera Uniforms"),
            layout: &Self::bind_group_layout(ctx),
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(
                    uniforms_buffer.as_ref().as_entire_buffer_binding(),
                ),
            }],
        });
        Self { bind_group }
    }
}

// MARK: Renderer

pub struct Renderer {
    size: (usize, usize),
    pub(crate) pipelines: PipelinesStorage,
    packets: RenderPackets,
    render_graph: RenderGraph,

    pub render_textures: RenderTextures,
    pub camera_state: Camera,

    output_staging_buffer: wgpu::Buffer,
    output_texture_data: Option<Vec<u8>>,
    pub(crate) output_texture_updated: bool,

    depth_staging_buffer: wgpu::Buffer,
    depth_texture_data: Option<Vec<f32>>,
    pub(crate) depth_texture_updated: bool,

    #[cfg(feature = "profiling")]
    pub(crate) profiler: wgpu_profiler::GpuProfiler,
}

pub struct Camera {
    frame_height: f64,
    ratio: f64,
    uniforms_buffer: WgpuBuffer<CameraUniforms>,
    uniforms_bind_group: CameraUniformsBindGroup,
}

impl Camera {
    pub fn new(ctx: &WgpuContext, camera: &CameraFrame, frame_height: f64, ratio: f64) -> Self {
        let uniforms = CameraUniforms::from_camera_frame(camera, frame_height, ratio);
        let uniforms_buffer = WgpuBuffer::new_init(
            ctx,
            Some("Uniforms Buffer"),
            wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            uniforms,
        );
        let uniforms_bind_group = CameraUniformsBindGroup::new(ctx, &uniforms_buffer);
        Self {
            frame_height,
            ratio,
            uniforms_buffer,
            uniforms_bind_group,
        }
    }
    pub fn update_uniforms(&mut self, wgpu_ctx: &WgpuContext, camera_frame: &CameraFrame) {
        self.uniforms_buffer.set(
            wgpu_ctx,
            CameraUniforms::from_camera_frame(camera_frame, self.frame_height, self.ratio),
        );
    }
}

impl Renderer {
    pub fn new(ctx: &WgpuContext, frame_height: f64, width: usize, height: usize) -> Self {
        let camera = CameraFrame::new();

        let render_textures = RenderTextures::new(ctx, width, height);
        let camera_state = Camera::new(ctx, &camera, frame_height, width as f64 / height as f64);
        let bytes_per_row = ((width * 4) as f32 / ALIGNMENT as f32).ceil() as usize * ALIGNMENT;
        let output_staging_buffer = ctx.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Output Staging Buffer"),
            size: (bytes_per_row * height) as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let depth_staging_buffer = ctx.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Depth Staging Buffer"),
            size: (bytes_per_row * height) as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

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
        let vitem2d_depth = render_graph.insert_node(VItem2dDepthNode);
        let vitem2d_render = render_graph.insert_node(VItem2dRenderNode);
        let vitem_render = render_graph.insert_node(VItemRenderNode);
        let vitem_compute = render_graph.insert_node(VItemComputeRenderNode);
        render_graph.insert_edge(vitem_compute, vitem_render);
        render_graph.insert_edge(vitem_compute, vitem2d_depth);
        render_graph.insert_edge(vitem2d_depth, vitem2d_render);

        Self {
            size: (width, height),
            pipelines: PipelinesStorage::default(),
            render_textures,
            packets: RenderPackets::default(),
            render_graph,
            // Outputs
            output_staging_buffer,
            output_texture_data: None,
            output_texture_updated: false,
            // Depth
            depth_staging_buffer,
            depth_texture_data: None,
            depth_texture_updated: false,
            // Camera State
            camera_state,
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
        self.output_texture_updated = false;
        self.depth_texture_updated = false;
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

        self.camera_state.update_uniforms(ctx, camera_frame);

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
                    &self.camera_state,
                );

                // // Compute Pass
                // // VItem and VItem2d
                // VItemComputeRenderNode.exec(
                //     #[cfg(not(feature = "profiling"))]
                //     &mut encoder,
                //     #[cfg(feature = "profiling")]
                //     &mut scope,
                //     &self.packets,
                //     &mut ctx,
                //     &self.camera_state,
                // );

                // // Depth Render Pass
                // // VItem2d
                // VItem2dDepthNode.exec(
                //     #[cfg(not(feature = "profiling"))]
                //     &mut encoder,
                //     #[cfg(feature = "profiling")]
                //     &mut scope,
                //     &self.packets,
                //     &mut ctx,
                //     &self.camera_state,
                // );

                // {
                //     #[cfg(feature = "profiling")]
                //     let mut scope = scope.scope("Render Pass");
                //     VItemRenderNode.exec(
                //         #[cfg(not(feature = "profiling"))]
                //         &mut encoder,
                //         #[cfg(feature = "profiling")]
                //         &mut scope,
                //         &self.packets,
                //         &mut ctx,
                //         &self.camera_state,
                //     );
                //     VItem2dRenderNode.exec(
                //         #[cfg(not(feature = "profiling"))]
                //         &mut encoder,
                //         #[cfg(feature = "profiling")]
                //         &mut scope,
                //         &self.packets,
                //         &mut ctx,
                //         &self.camera_state,
                //     );
                // }
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

            self.output_texture_updated = false;
            self.depth_texture_updated = false;
        }

        self.packets.clear();
        // drop(render_primitives);
    }

    fn update_rendered_texture_data(&mut self, ctx: &WgpuContext) {
        let bytes_per_row =
            ((self.size.0 * 4) as f64 / ALIGNMENT as f64).ceil() as usize * ALIGNMENT;
        let mut texture_data =
            self.output_texture_data
                .take()
                .unwrap_or(vec![0; self.size.0 * self.size.1 * 4]);

        let mut encoder = ctx
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        let RenderTextures { render_texture, .. } = &self.render_textures;
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                aspect: wgpu::TextureAspect::All,
                texture: render_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &self.output_staging_buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(bytes_per_row as u32),
                    rows_per_image: Some(self.size.1 as u32),
                },
            },
            render_texture.size(),
        );
        ctx.queue.submit(Some(encoder.finish()));

        {
            let buffer_slice = self.output_staging_buffer.slice(..);

            // NOTE: We have to create the mapping THEN device.poll() before await
            // the future. Otherwise the application will freeze.
            let (tx, rx) = async_channel::bounded(1);
            buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
                pollster::block_on(tx.send(result)).unwrap()
            });
            ctx.device
                .poll(wgpu::PollType::wait_indefinitely())
                .unwrap();
            pollster::block_on(rx.recv()).unwrap().unwrap();

            {
                let view = buffer_slice.get_mapped_range();
                // texture_data.copy_from_slice(&view);
                for y in 0..self.size.1 {
                    let src_row_start = y * bytes_per_row;
                    let dst_row_start = y * self.size.0 * 4;

                    texture_data[dst_row_start..dst_row_start + self.size.0 * 4]
                        .copy_from_slice(&view[src_row_start..src_row_start + self.size.0 * 4]);
                }
            }
        };
        self.output_staging_buffer.unmap();

        self.output_texture_data = Some(texture_data);
        self.output_texture_updated = true;
    }

    // pub(crate) fn get_render_texture(&self) -> &wgpu::Texture {
    //     &self.render_texture
    // }

    pub fn get_rendered_texture_data(&mut self, ctx: &WgpuContext) -> &[u8] {
        if !self.output_texture_updated {
            // trace!("[Camera] Updating rendered texture data...");
            self.update_rendered_texture_data(ctx);
        }
        self.output_texture_data.as_ref().unwrap()
    }
    pub fn get_rendered_texture_img_buffer(
        &mut self,
        ctx: &WgpuContext,
    ) -> ImageBuffer<Rgba<u8>, &[u8]> {
        let size = self.size;
        let data = self.get_rendered_texture_data(ctx);
        ImageBuffer::from_raw(size.0 as u32, size.1 as u32, data).unwrap()
    }

    fn update_depth_texture_data(&mut self, ctx: &WgpuContext) {
        let bytes_per_row =
            ((self.size.0 * 4) as f64 / ALIGNMENT as f64).ceil() as usize * ALIGNMENT;
        let mut texture_data =
            self.depth_texture_data
                .take()
                .unwrap_or(vec![0.0; self.size.0 * self.size.1]);

        let mut encoder = ctx
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Depth Encoder"),
            });

        let RenderTextures {
            depth_stencil_texture,
            ..
        } = &self.render_textures;
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                aspect: wgpu::TextureAspect::DepthOnly,
                texture: depth_stencil_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &self.depth_staging_buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(bytes_per_row as u32),
                    rows_per_image: Some(self.size.1 as u32),
                },
            },
            depth_stencil_texture.size(),
        );
        ctx.queue.submit(Some(encoder.finish()));

        {
            let buffer_slice = self.depth_staging_buffer.slice(..);

            let (tx, rx) = async_channel::bounded(1);
            buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
                pollster::block_on(tx.send(result)).unwrap()
            });
            ctx.device
                .poll(wgpu::PollType::wait_indefinitely())
                .unwrap();
            pollster::block_on(rx.recv()).unwrap().unwrap();

            {
                let view = buffer_slice.get_mapped_range();
                let floats: &[f32] = bytemuck::cast_slice(&view);
                let floats_per_row = bytes_per_row / 4;

                for y in 0..self.size.1 {
                    let src_row_start = y * floats_per_row;
                    let dst_row_start = y * self.size.0;

                    texture_data[dst_row_start..dst_row_start + self.size.0]
                        .copy_from_slice(&floats[src_row_start..src_row_start + self.size.0]);
                }
            }
        };
        self.depth_staging_buffer.unmap();

        self.depth_texture_data = Some(texture_data);
        self.depth_texture_updated = true;
    }

    pub fn get_depth_texture_data(&mut self, ctx: &WgpuContext) -> &[f32] {
        if !self.depth_texture_updated {
            self.update_depth_texture_data(ctx);
        }
        self.depth_texture_data.as_ref().unwrap()
    }

    pub fn get_depth_texture_img_buffer(
        &mut self,
        ctx: &WgpuContext,
    ) -> ImageBuffer<Luma<u8>, Vec<u8>> {
        let size = self.size;
        let data = self.get_depth_texture_data(ctx);
        // Map 0.0-1.0 to 0-255
        let u8_data: Vec<u8> = data
            .iter()
            .map(|&d| (d.clamp(0.0, 1.0) * 255.0) as u8)
            .collect();
        ImageBuffer::from_raw(size.0 as u32, size.1 as u32, u8_data).unwrap()
    }
}

// MARK: RenderTextures
/// Texture resources used for rendering
#[allow(unused)]
pub struct RenderTextures {
    pub render_texture: wgpu::Texture,
    // multisample_texture: wgpu::Texture,
    pub depth_stencil_texture: wgpu::Texture,
    pub render_view: wgpu::TextureView,
    pub linear_render_view: wgpu::TextureView,
    // pub(crate) multisample_view: wgpu::TextureView,
    pub(crate) depth_stencil_view: wgpu::TextureView,
}

impl RenderTextures {
    pub(crate) fn new(ctx: &WgpuContext, width: usize, height: usize) -> Self {
        let format = OUTPUT_TEXTURE_FORMAT;
        let render_texture = ctx.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Target Texture"),
            size: wgpu::Extent3d {
                width: width as u32,
                height: height as u32,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::COPY_SRC
                | wgpu::TextureUsages::COPY_DST
                | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[
                wgpu::TextureFormat::Rgba8UnormSrgb,
                wgpu::TextureFormat::Rgba8Unorm,
            ],
        });
        // let multisample_texture = ctx.device.create_texture(&wgpu::TextureDescriptor {
        //     label: Some("Multisample Texture"),
        //     size: wgpu::Extent3d {
        //         width: width as u32,
        //         height: height as u32,
        //         depth_or_array_layers: 1,
        //     },
        //     mip_level_count: 1,
        //     sample_count: 4,
        //     dimension: wgpu::TextureDimension::D2,
        //     format,
        //     usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        //     view_formats: &[
        //         wgpu::TextureFormat::Rgba8UnormSrgb,
        //         wgpu::TextureFormat::Rgba8Unorm,
        //     ],
        // });
        let depth_stencil_texture = ctx.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Depth Stencil Texture"),
            size: wgpu::Extent3d {
                width: width as u32,
                height: height as u32,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let render_view = render_texture.create_view(&wgpu::TextureViewDescriptor {
            format: Some(format),
            ..Default::default()
        });
        let linear_render_view = render_texture.create_view(&wgpu::TextureViewDescriptor {
            format: Some(wgpu::TextureFormat::Rgba8Unorm),
            ..Default::default()
        });
        // let multisample_view = multisample_texture.create_view(&wgpu::TextureViewDescriptor {
        //     format: Some(format),
        //     ..Default::default()
        // });
        let depth_stencil_view =
            depth_stencil_texture.create_view(&wgpu::TextureViewDescriptor::default());

        Self {
            render_texture,
            // multisample_texture,
            depth_stencil_texture,
            render_view,
            linear_render_view,
            // multisample_view,
            depth_stencil_view,
        }
    }
}

/// A render resource.
pub(crate) trait RenderResource {
    fn new(ctx: &WgpuContext) -> Self
    where
        Self: Sized;
}
