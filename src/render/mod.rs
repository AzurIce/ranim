pub mod pipelines;
pub mod primitives;

use color::LinearSrgb;
use glam::{Mat4, Vec2};
use image::{ImageBuffer, Rgba};
use pipelines::{Map3dTo2dPipeline, VItemPipeline};
use primitives::Renderable;

use crate::{
    color::rgba8,
    context::{RanimContext, WgpuContext},
    items::camera_frame::CameraFrame,
    utils::{PipelinesStorage, wgpu::WgpuBuffer},
};

pub const OUTPUT_TEXTURE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;
const ALIGNMENT: usize = 256;

#[cfg(feature = "profiling")]
use crate::PUFFIN_GPU_PROFILER;

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

#[repr(C, align(16))]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
/// Uniforms for the camera
pub struct CameraUniforms {
    proj_mat: Mat4,
    view_mat: Mat4,
    half_frame_size: Vec2,
    _padding: [f32; 2],
}

impl CameraUniforms {
    pub fn as_bind_group_layout_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
        wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::all(),
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }
    }
}

pub struct CameraUniformsBindGroup {
    pub bind_group: wgpu::BindGroup,
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
    frame_height: f64,
    size: (usize, usize),
    pub pipelines: PipelinesStorage,

    clear_color: wgpu::Color,
    pub(crate) render_textures: RenderTextures,

    uniforms_buffer: WgpuBuffer<CameraUniforms>,
    pub(crate) uniforms_bind_group: CameraUniformsBindGroup,

    output_staging_buffer: wgpu::Buffer,
    output_texture_data: Option<Vec<u8>>,
    pub(crate) output_texture_updated: bool,

    #[cfg(feature = "profiling")]
    pub(crate) profiler: wgpu_profiler::GpuProfiler,
}

impl Renderer {
    pub(crate) fn new(ctx: &RanimContext, frame_height: f64, width: usize, height: usize) -> Self {
        let camera = CameraFrame::new();

        let ctx = &ctx.wgpu_ctx;
        let render_textures = RenderTextures::new(ctx, width, height);
        let bytes_per_row = ((width * 4) as f32 / ALIGNMENT as f32).ceil() as usize * ALIGNMENT;
        let output_staging_buffer = ctx.device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: (bytes_per_row * height) as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        let uniforms = CameraUniforms {
            proj_mat: camera
                .projection_matrix(width as f64, height as f64)
                .as_mat4(),
            view_mat: camera.view_matrix().as_mat4(),
            half_frame_size: Vec2::new(width as f32 / 2.0, height as f32 / 2.0),
            _padding: [0.0; 2],
        };
        // trace!("init renderer uniform: {:?}", uniforms);
        let uniforms_buffer = WgpuBuffer::new_init(
            ctx,
            Some("Uniforms Buffer"),
            wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            uniforms,
        );
        let uniforms_bind_group = CameraUniformsBindGroup::new(ctx, &uniforms_buffer);

        let bg = rgba8(0x33, 0x33, 0x33, 0xff).convert::<LinearSrgb>();
        let [r, g, b, a] = bg.components.map(|x| x as f64);
        let clear_color = wgpu::Color { r, g, b, a };

        #[cfg(feature = "profiling")]
        let profiler = wgpu_profiler::GpuProfiler::new(
            &ctx.device,
            wgpu_profiler::GpuProfilerSettings::default(),
        )
        .unwrap();

        Self {
            frame_height,
            size: (width, height),
            clear_color,
            pipelines: PipelinesStorage::default(),
            render_textures,
            // Outputs
            output_staging_buffer,
            output_texture_data: None,
            output_texture_updated: false,
            // Uniforms
            uniforms_buffer,
            uniforms_bind_group,
            // Profiler
            #[cfg(feature = "profiling")]
            profiler,
        }
    }

    /// Clears the screen with `Renderer::clear_color`
    pub fn clear_screen(&mut self, wgpu_ctx: &WgpuContext) {
        #[cfg(feature = "profiling")]
        profiling::scope!("clear_screen");
        // trace!("clear screen {:?}", self.clear_color);
        let mut encoder = wgpu_ctx
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Encoder"),
            });

        // Clear
        {
            let RenderTextures {
                render_view,
                // multisample_view,
                // depth_stencil_view,
                ..
            } = &self.render_textures;
            let _ = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("VMobject Clear Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    // view: multisample_view,
                    // resolve_target: Some(render_view),
                    view: render_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(self.clear_color),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                // depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                //     view: depth_stencil_view,
                //     depth_ops: Some(wgpu::Operations {
                //         load: wgpu::LoadOp::Clear(1.0),
                //         store: wgpu::StoreOp::Store,
                //     }),
                //     stencil_ops: Some(wgpu::Operations {
                //         load: wgpu::LoadOp::Clear(0),
                //         store: wgpu::StoreOp::Store,
                //     }),
                // }),
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });
        }
        wgpu_ctx.queue.submit(Some(encoder.finish()));
        self.output_texture_updated = false;
    }

    pub fn render(&mut self, ctx: &mut RanimContext, renderable: &impl Renderable) {
        self.clear_screen(&ctx.wgpu_ctx);
        let mut encoder = ctx
            .wgpu_ctx
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

        // renderable.update_clip_info(&ctx.wgpu_ctx, &self.camera);
        {
            #[cfg(feature = "profiling")]
            let mut scope = self.profiler.scope("compute pass", &mut encoder);
            #[cfg(feature = "profiling")]
            let mut cpass = scope.scoped_compute_pass("VItem Map Points Compute Pass");
            #[cfg(not(feature = "profiling"))]
            let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("VItem Map Points Compute Pass"),
                timestamp_writes: None,
            });
            cpass.set_pipeline(
                ctx.pipelines
                    .get_or_init::<Map3dTo2dPipeline>(&ctx.wgpu_ctx),
            );
            cpass.set_bind_group(0, &self.uniforms_bind_group.bind_group, &[]);

            renderable.encode_compute_pass_command(
                &ctx.wgpu_ctx,
                &mut ctx.pipelines,
                &mut cpass,
                &self.uniforms_bind_group.bind_group,
                &self.render_textures,
                #[cfg(feature = "profiling")]
                &self.profiler,
            );
        }
        {
            #[cfg(feature = "profiling")]
            let mut scope = self.profiler.scope("render pass", &mut encoder);
            let RenderTextures {
                // multisample_view,
                render_view,
                ..
            } = &mut self.render_textures;
            let rpass_desc = wgpu::RenderPassDescriptor {
                label: Some("VItem Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    // view: multisample_view,
                    // resolve_target: Some(render_view),
                    view: render_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            };
            #[cfg(feature = "profiling")]
            let mut rpass = scope.scoped_render_pass("VItem Render Pass", rpass_desc);
            #[cfg(not(feature = "profiling"))]
            let mut rpass = encoder.begin_render_pass(&rpass_desc);
            rpass.set_pipeline(ctx.pipelines.get_or_init::<VItemPipeline>(&ctx.wgpu_ctx));
            rpass.set_bind_group(0, &self.uniforms_bind_group.bind_group, &[]);

            renderable.encode_render_pass_command(
                &ctx.wgpu_ctx,
                &mut ctx.pipelines,
                &mut rpass,
                &self.uniforms_bind_group.bind_group,
                &self.render_textures,
                #[cfg(feature = "profiling")]
                &self.profiler,
            );
        }
        // renderable.encode_render_command(
        //     &ctx.wgpu_ctx,
        //     &mut ctx.pipelines,
        //     &mut encoder,
        //     &self.uniforms_bind_group.bind_group,
        //     &self.render_textures,
        //     #[cfg(feature = "profiling")]
        //     &mut self.profiler,
        // );

        #[cfg(not(feature = "profiling"))]
        ctx.wgpu_ctx.queue.submit(Some(encoder.finish()));

        #[cfg(feature = "profiling")]
        {
            self.profiler.resolve_queries(&mut encoder);
            {
                profiling::scope!("submit");
                ctx.wgpu_ctx.queue.submit(Some(encoder.finish()));
            }

            renderable.debug(&ctx.wgpu_ctx);

            // Signal to the profiler that the frame is finished.
            self.profiler.end_frame().unwrap();

            // Query for oldest finished frame (this is almost certainly not the one we just submitted!) and display results in the command line.
            ctx.wgpu_ctx.device.poll(wgpu::Maintain::Wait);
            let latest_profiler_results = self
                .profiler
                .process_finished_frame(ctx.wgpu_ctx.queue.get_timestamp_period());
            // profiling_utils::console_output(&latest_profiler_results, ctx.wgpu_ctx.device.features());
            let mut gpu_profiler = PUFFIN_GPU_PROFILER.lock().unwrap();
            wgpu_profiler::puffin::output_frame_to_puffin(
                &mut gpu_profiler,
                &latest_profiler_results.unwrap(),
            );
            gpu_profiler.new_frame();
        }

        self.output_texture_updated = false;
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

        pollster::block_on(async {
            let buffer_slice = self.output_staging_buffer.slice(..);

            // NOTE: We have to create the mapping THEN device.poll() before await
            // the future. Otherwise the application will freeze.
            let (tx, rx) = async_channel::bounded(1);
            buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
                tx.send_blocking(result).unwrap()
            });
            ctx.device.poll(wgpu::Maintain::Wait).panic_on_timeout();
            rx.recv().await.unwrap().unwrap();

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
        });
        self.output_staging_buffer.unmap();

        self.output_texture_data = Some(texture_data);
        self.output_texture_updated = true;
    }

    // pub(crate) fn get_render_texture(&self) -> &wgpu::Texture {
    //     &self.render_texture
    // }

    pub(crate) fn get_rendered_texture_data(&mut self, ctx: &WgpuContext) -> &[u8] {
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

    pub fn update_uniforms(&mut self, wgpu_ctx: &WgpuContext, camera_frame: &CameraFrame) {
        let uniforms = CameraUniforms {
            view_mat: camera_frame.view_matrix().as_mat4(),
            proj_mat: camera_frame
                .projection_matrix(self.frame_height, self.size.0 as f64 / self.size.1 as f64)
                .as_mat4(),
            // center of the screen
            half_frame_size: Vec2::new(self.size.0 as f32 / 2.0, self.size.1 as f32 / 2.0),
            _padding: [0.0; 2],
        };
        // trace!("Uniforms: {:?}", self.uniforms);
        // trace!("[Camera] uploading camera uniforms to buffer...");
        self.uniforms_buffer.set(wgpu_ctx, uniforms);
    }
}

// MARK: RenderTextures

pub struct RenderTextures {
    pub(crate) render_texture: wgpu::Texture,
    // multisample_texture: wgpu::Texture,
    // depth_stencil_texture: wgpu::Texture,
    pub(crate) render_view: wgpu::TextureView,
    // pub(crate) multisample_view: wgpu::TextureView,
    // pub(crate) depth_stencil_view: wgpu::TextureView,
}

impl RenderTextures {
    pub fn new(ctx: &WgpuContext, width: usize, height: usize) -> Self {
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
        // let depth_stencil_texture = ctx.device.create_texture(&wgpu::TextureDescriptor {
        //     label: Some("Depth Stencil Texture"),
        //     size: wgpu::Extent3d {
        //         width: width as u32,
        //         height: height as u32,
        //         depth_or_array_layers: 1,
        //     },
        //     mip_level_count: 1,
        //     sample_count: 1,
        //     dimension: wgpu::TextureDimension::D2,
        //     format: wgpu::TextureFormat::Depth24PlusStencil8,
        //     usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        //     view_formats: &[],
        // });
        let render_view = render_texture.create_view(&wgpu::TextureViewDescriptor {
            format: Some(format),
            ..Default::default()
        });
        // let multisample_view = multisample_texture.create_view(&wgpu::TextureViewDescriptor {
        //     format: Some(format),
        //     ..Default::default()
        // });
        // let depth_stencil_view =
        //     depth_stencil_texture.create_view(&wgpu::TextureViewDescriptor::default());

        Self {
            render_texture,
            // multisample_texture,
            // depth_stencil_texture,
            render_view,
            // multisample_view,
            // depth_stencil_view,
        }
    }
}

/// A render resource.
pub trait RenderResource {
    fn new(ctx: &WgpuContext) -> Self
    where
        Self: Sized;
}
