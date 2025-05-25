mod egui_tools;
mod timeline;

use std::sync::Arc;

use egui_wgpu::ScreenDescriptor;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use timeline::TimelineState;
use wgpu::SurfaceError;
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::WindowEvent,
    event_loop::ActiveEventLoop,
    window::{Window, WindowId},
};

use crate::{
    Scene, SceneMeta,
    animation::EvalResult,
    build_timeline,
    context::{RanimContext, WgpuContext},
    render::{
        Renderer,
        pipelines::app::{AppPipeline, Viewport},
        primitives::RenderInstances,
    },
    timeline::{SealedRanimScene, TimelineEvalResult},
};

#[derive(Default, Debug, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
struct OccupiedScreenSpace {
    top: f32,
    bottom: f32,
    left: f32,
    right: f32,
}

struct RenderState {
    pub surface_config: wgpu::SurfaceConfiguration,
    pub surface: wgpu::Surface<'static>,
    pub scale_factor: f32,
    pub egui_renderer: egui_tools::EguiRenderer,
    pub app_pipeline: AppPipeline,

    pub viewport: Viewport,
}

impl RenderState {
    async fn new(
        ctx: &WgpuContext,
        surface: wgpu::Surface<'static>,
        window: &Window,
        width: u32,
        height: u32,
        render_view: &wgpu::TextureView,
    ) -> Self {
        let swapchain_capabilities = surface.get_capabilities(&ctx.adapter);
        let selected_format = wgpu::TextureFormat::Bgra8UnormSrgb;
        let swapchain_format = swapchain_capabilities
            .formats
            .iter()
            .find(|d| **d == selected_format)
            .expect("failed to select proper surface texture format!");

        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: *swapchain_format,
            width,
            height,
            present_mode: wgpu::PresentMode::AutoVsync,
            desired_maximum_frame_latency: 0,
            alpha_mode: swapchain_capabilities.alpha_modes[0],
            view_formats: vec![],
        };

        surface.configure(&ctx.device, &surface_config);

        let egui_renderer =
            egui_tools::EguiRenderer::new(&ctx.device, surface_config.format, None, 1, window);

        let scale_factor = 1.0;

        let sampler = ctx.device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let app_pipeline = AppPipeline::new(
            ctx,
            render_view,
            &sampler,
            swapchain_capabilities.formats[0].into(),
        );

        Self {
            surface,
            surface_config,
            egui_renderer,
            app_pipeline,
            scale_factor,
            viewport: Viewport {
                width: 1.0,
                height: 1.0,
                x: 0.0,
                y: 0.0,
            },
        }
    }

    fn resize_surface(&mut self, ctx: &WgpuContext, width: u32, height: u32) {
        self.surface_config.width = width;
        self.surface_config.height = height;
        self.surface.configure(&ctx.device, &self.surface_config);
    }

    fn update_viewport(&mut self, occupied_screen_space: OccupiedScreenSpace, render_aspect: f32) {
        // Calculate viewport
        let screen_width = self.surface_config.width as f32;
        let screen_height = self.surface_config.height as f32;
        // dbg!(screen_width, screen_height);

        let OccupiedScreenSpace {
            top,
            bottom,
            left,
            right,
        } = occupied_screen_space;
        // dbg!(state.occupied_screen_space);

        // Calculate available space
        let available_width = screen_width - left - right;
        let available_height = screen_height - top - bottom;
        // dbg!(available_width, available_height);

        // Calculate aspect ratio of the render texture
        let viewport_aspect = available_width / available_height;

        let (viewport_width, viewport_height, viewport_x, viewport_y) =
            if render_aspect > viewport_aspect {
                // Render is wider than viewport
                let width = available_width;
                let height = width / render_aspect;
                let x = left - right;
                let y = bottom - top;
                (
                    width / screen_width,
                    height / screen_height,
                    x / screen_width,
                    y / screen_height,
                )
            } else {
                // Render is taller than viewport
                let height = available_height;
                let width = height * render_aspect;
                let x = left - right;
                let y = bottom - top;
                (
                    width / screen_width,
                    height / screen_height,
                    x / screen_width,
                    y / screen_height,
                )
            };
        self.viewport = Viewport {
            width: viewport_width,
            height: viewport_height,
            x: viewport_x,
            y: viewport_y,
        };
    }
}

struct WinitApp {
    state: Option<RenderState>,
    window: Option<Arc<Window>>,

    ctx: RanimContext,
    app_state: AppState,
}

struct TimelineInfo {
    ctx: egui::Context,
    canvas: egui::Rect,
    response: egui::Response,
    painter: egui::Painter,
    text_height: f32,
    font_id: egui::FontId,
}

impl TimelineInfo {
    fn point_from_ms(&self, state: &TimelineState, ms: i64) -> f32 {
        self.canvas.min.x
            + state.offset_points
            + self.canvas.width() * ms as f32 / (state.width_sec * 1000.0) as f32
    }
}

struct AppState {
    meta: SceneMeta,
    timeline: SealedRanimScene,
    // app_options: AppOptions<'a>,
    // timeline: RanimTimeline,
    renderer: Renderer,

    last_sec: f64,
    render_instances: RenderInstances,
    timeline_state: TimelineState,
}

impl AppState {
    fn new(ctx: &RanimContext, scene: impl Scene) -> Self {
        let meta = scene.meta();
        let timeline = build_timeline(scene);
        let timeline_infos = timeline.get_timeline_infos();
        let renderer = Renderer::new(ctx, 8.0, 1920, 1080);
        Self {
            meta,
            renderer,
            last_sec: -1.0,
            render_instances: RenderInstances::default(),
            timeline_state: TimelineState::new(timeline.total_secs(), timeline_infos),
            timeline,
        }
    }
    pub fn prepare(&mut self, ctx: &mut RanimContext, state: &mut RenderState) {
        #[cfg(feature = "profiling")]
        profiling::scope!("frame");

        state
            .app_pipeline
            .bind_group
            .update_viewport(&ctx.wgpu_ctx.queue, state.viewport);

        if self.last_sec == self.timeline_state.current_sec {
            return;
        }
        self.last_sec = self.timeline_state.current_sec;

        let TimelineEvalResult {
            // EvalResult<CameraFrame>, idx
            camera_frame,
            // Vec<(rabject_id, EvalResult<Item>, idx)>
            visual_items,
        } = {
            #[cfg(feature = "profiling")]
            profiling::scope!("eval");

            self.timeline.eval_sec(self.timeline_state.current_sec)
        };

        let extracted = {
            #[cfg(feature = "profiling")]
            profiling::scope!("extract");
            visual_items
                .iter()
                .map(|(id, res, _)| {
                    let renderable = match res {
                        EvalResult::Dynamic(res) => res.extract_renderable(),
                        EvalResult::Static(res) => res.extract_renderable(),
                    };
                    (*id, renderable)
                })
                .collect::<Vec<_>>()
        };

        {
            #[cfg(feature = "profiling")]
            profiling::scope!("prepare");
            extracted.iter().for_each(|(id, renderable)| {
                renderable.prepare_for_id(&ctx.wgpu_ctx, &mut self.render_instances, *id);
            });
            ctx.wgpu_ctx.queue.submit([]);
        }

        let render_primitives = visual_items
            .iter()
            .filter_map(|(id, _, _)| self.render_instances.get_render_instance_dyn(*id))
            .collect::<Vec<_>>();
        let camera_frame = match &camera_frame.0 {
            EvalResult::Dynamic(res) => res,
            EvalResult::Static(res) => res,
        };
        // println!("{:?}", camera_frame);
        // println!("{}", render_primitives.len());
        self.renderer.update_uniforms(&ctx.wgpu_ctx, camera_frame);

        {
            #[cfg(feature = "profiling")]
            profiling::scope!("render");

            self.renderer.render(ctx, &render_primitives);
        }
    }

    #[allow(clippy::field_reassign_with_default)]
    pub fn ui(&mut self, state: &mut RenderState) -> OccupiedScreenSpace {
        let scale_factor = state.scale_factor;
        let mut occupied_screen_space = OccupiedScreenSpace::default();

        occupied_screen_space.bottom = egui::TopBottomPanel::bottom("bottom_panel")
            .resizable(true)
            .max_height(600.0)
            .show(state.egui_renderer.context(), |ui| {
                ui.label("Timeline");

                ui.style_mut().spacing.slider_width = ui.available_width() - 70.0;
                ui.add(
                    egui::Slider::new(
                        &mut self.timeline_state.current_sec,
                        0.0..=self.timeline_state.total_sec,
                    )
                    .text("sec"),
                );

                // self.timeline_state.ui_preview_timeline(ui);
                self.timeline_state.ui_main_timeline(ui);
            })
            .response
            .rect
            .height()
            * scale_factor;

        // dbg!(state.viewport);

        // egui::Window::new(format!("{}", self.meta.name))
        //     .resizable(true)
        //     .vscroll(true)
        //     .default_open(false)
        //     .show(state.egui_renderer.context(), |ui| {
        //         ui.label("Label!");

        //         if ui.button("Button!").clicked() {}

        //         ui.separator();
        //         ui.horizontal(|ui| {
        //             ui.label(format!(
        //                 "Pixels per point: {}",
        //                 state.egui_renderer.context().pixels_per_point()
        //             ));
        //             if ui.button("-").clicked() {
        //                 state.scale_factor = (state.scale_factor - 0.1).max(0.3);
        //             }
        //             if ui.button("+").clicked() {
        //                 state.scale_factor = (state.scale_factor + 0.1).min(3.0);
        //             }
        //         });
        //     });
        occupied_screen_space
    }
}

impl WinitApp {
    fn new(scene: impl Scene) -> Self {
        let ctx = RanimContext::new();
        let app_state = AppState::new(&ctx, scene);

        Self {
            ctx,
            state: None,
            window: None,
            app_state,
        }
    }

    async fn init_window(&mut self, window: Window) {
        let window = Arc::new(window);
        let initial_width = 1360;
        let initial_height = 768;

        let _ = window.request_inner_size(PhysicalSize::new(initial_width, initial_height));

        let surface = self
            .ctx
            .wgpu_ctx
            .instance
            .create_surface(window.clone())
            .expect("Failed to create surface!");

        let state = RenderState::new(
            &self.ctx.wgpu_ctx,
            surface,
            &window,
            initial_width,
            initial_width,
            &self.app_state.renderer.render_textures.render_view,
        )
        .await;

        self.window.get_or_insert(window);
        self.state.get_or_insert(state);
    }

    fn handle_resized(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.state
                .as_mut()
                .unwrap()
                .resize_surface(&self.ctx.wgpu_ctx, width, height);
        }
    }

    fn handle_redraw(&mut self) {
        #[cfg(feature = "profiling")]
        profiling::scope!("redraw");

        // Attempt to handle minimizing window
        if let Some(window) = self.window.as_ref() {
            if let Some(min) = window.is_minimized() {
                if min {
                    println!("Window is minimized");
                    return;
                }
            }
        }

        let state = self.state.as_mut().unwrap();

        let screen_descriptor = ScreenDescriptor {
            size_in_pixels: [state.surface_config.width, state.surface_config.height],
            pixels_per_point: self.window.as_ref().unwrap().scale_factor() as f32
                * state.scale_factor,
        };

        let surface_texture = state.surface.get_current_texture();

        match surface_texture {
            Err(SurfaceError::Outdated) => {
                // Ignoring outdated to allow resizing and minimization
                println!("wgpu surface outdated");
                return;
            }
            Err(_) => {
                surface_texture.expect("Failed to acquire next swap chain texture");
                return;
            }
            Ok(_) => {}
        };

        let surface_texture = surface_texture.unwrap();

        let surface_view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .ctx
            .wgpu_ctx
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

        let window = self.window.as_ref().unwrap();

        // MARK: app render
        {
            #[cfg(feature = "profiling")]
            profiling::scope!("app render");

            self.app_state.prepare(&mut self.ctx, state);
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &surface_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            render_pass.set_pipeline(&state.app_pipeline);
            render_pass.set_bind_group(0, state.app_pipeline.bind_group.as_ref(), &[]);
            render_pass.draw(0..4, 0..1);
        }

        // MARK: egui render
        {
            #[cfg(feature = "profiling")]
            profiling::scope!("egui render");

            state.egui_renderer.begin_frame(window);
            let occupied_screen_space = self.app_state.ui(state);
            state.update_viewport(occupied_screen_space, 16.0 / 9.0); // TODO: get from renderer

            state.egui_renderer.end_frame_and_draw(
                &self.ctx.wgpu_ctx.device,
                &self.ctx.wgpu_ctx.queue,
                &mut encoder,
                window,
                &surface_view,
                screen_descriptor,
            );
        }

        {
            #[cfg(feature = "profiling")]
            profiling::scope!("submit");

            self.ctx.wgpu_ctx.queue.submit(Some(encoder.finish()));
        }
        {
            #[cfg(feature = "profiling")]
            profiling::scope!("present");

            surface_texture.present();
        }

        #[cfg(feature = "profiling")]
        profiling::finish_frame!();
    }
}

impl ApplicationHandler for WinitApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window = event_loop
            .create_window(
                Window::default_attributes()
                    .with_title(format!("Ranim {}", self.app_state.meta.name)),
            )
            .unwrap();
        pollster::block_on(self.init_window(window));
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        self.state
            .as_mut()
            .unwrap()
            .egui_renderer
            .handle_input(self.window.as_ref().unwrap(), &event);
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::RedrawRequested => {
                self.handle_redraw();
                self.window.as_ref().unwrap().request_redraw();
            }
            WindowEvent::Resized(new_size) => {
                self.handle_resized(new_size.width, new_size.height);
            }
            _ => (),
        }
    }
}

#[cfg(feature = "profiling")]
use crate::PUFFIN_GPU_PROFILER;

pub fn run_scene_app(scene: impl Scene) {
    #[cfg(feature = "profiling")]
    let (_cpu_server, _gpu_server) = {
        puffin::set_scopes_on(true);
        // default global profiler
        let cpu_server =
            puffin_http::Server::new(&format!("0.0.0.0:{}", puffin_http::DEFAULT_PORT)).unwrap();
        // custom gpu profiler in `PUFFIN_GPU_PROFILER`
        let gpu_server = puffin_http::Server::new_custom(
            &format!("0.0.0.0:{}", puffin_http::DEFAULT_PORT + 1),
            |sink| PUFFIN_GPU_PROFILER.lock().unwrap().add_sink(sink),
            |id| _ = PUFFIN_GPU_PROFILER.lock().unwrap().remove_sink(id),
        )
        .unwrap();
        (cpu_server, gpu_server)
    };

    let event_loop = winit::event_loop::EventLoop::new().unwrap();
    event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);

    let mut app = WinitApp::new(scene);

    event_loop.run_app(&mut app).expect("Failed to run app");
}
