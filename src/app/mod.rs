mod egui_tools;

use std::sync::Arc;

use egui_wgpu::ScreenDescriptor;
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
    prelude::RanimTimeline,
    render::{
        Renderer,
        pipelines::app::{AppPipeline, Viewport},
        primitives::RenderInstances,
    },
    timeline::TimelineEvalResult,
};

#[derive(Default, Debug, Clone, Copy)]
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
    pub occupied_screen_space: OccupiedScreenSpace,

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
            &ctx,
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
            occupied_screen_space: OccupiedScreenSpace::default(),
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
}

struct WinitApp {
    state: Option<RenderState>,
    window: Option<Arc<Window>>,

    ctx: RanimContext,
    app_state: AppState,
}

struct AppState {
    meta: SceneMeta,
    timeline: RanimTimeline,
    // app_options: AppOptions<'a>,
    // timeline: RanimTimeline,
    renderer: Renderer,

    last_sec: f64,
    current_sec: f64,
    render_instances: RenderInstances,
}

impl AppState {
    pub fn prepare(&mut self, ctx: &mut RanimContext, state: &mut RenderState) {
        #[cfg(feature = "profiling")]
        profiling::scope!("frame");

        state.app_pipeline.bind_group.update_viewport(
            &ctx.wgpu_ctx.queue,
            state.viewport,
        );

        if self.last_sec == self.current_sec {
            return;
        }
        self.last_sec = self.current_sec;

        let TimelineEvalResult {
            // EvalResult<CameraFrame>, idx
            camera_frame,
            // Vec<(rabject_id, EvalResult<Item>, idx)>
            items,
        } = {
            #[cfg(feature = "profiling")]
            profiling::scope!("eval");

            self.timeline.eval_sec(self.current_sec)
        };

        {
            #[cfg(feature = "profiling")]
            profiling::scope!("prepare");
            items.iter().for_each(|(id, res, _idx)| {
                // let last_idx = last_idx.entry(*id).or_insert(-1);
                // let prev_last_idx = *last_idx;
                // *last_idx = *idx as i32;
                match res {
                    EvalResult::Dynamic(res) => res.prepare_render_instance_for_entity(
                        &ctx.wgpu_ctx,
                        &mut self.render_instances,
                        *id,
                    ),
                    EvalResult::Static(res) => {
                        // if prev_last_idx != *idx as i32 {
                        res.prepare_render_instance_for_entity(
                            &ctx.wgpu_ctx,
                            &mut self.render_instances,
                            *id,
                        )
                        // }
                    }
                }
            });
            ctx.wgpu_ctx.queue.submit([]);
        }

        let render_primitives = items
            .iter()
            .filter_map(|(id, res, _)| match res {
                EvalResult::Dynamic(res) => {
                    res.get_render_instance_for_entity(&self.render_instances, *id)
                }
                EvalResult::Static(res) => {
                    res.get_render_instance_for_entity(&self.render_instances, *id)
                }
            })
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

    pub fn ui(&mut self, state: &mut RenderState) {
        let bottom = egui::TopBottomPanel::bottom("bottom_panel").show(state.egui_renderer.context(), |ui| {
            ui.label("Bottom Panel");
            ui.add(
                egui::Slider::new(&mut self.current_sec, 0.0..=self.timeline.duration_secs())
                    .text("sec"),
            );
        }).response.rect.height();

        state.occupied_screen_space.bottom = bottom * state.scale_factor;

        // Calculate viewport
        let screen_width = state.surface_config.width as f32;
        let screen_height = state.surface_config.height as f32;
        // dbg!(screen_width, screen_height);

        let OccupiedScreenSpace {
            top,
            bottom,
            left,
            right,
        } = state.occupied_screen_space;
        // dbg!(state.occupied_screen_space);

        // Calculate available space
        let available_width = screen_width - left - right;
        let available_height = screen_height - top - bottom;
        // dbg!(available_width, available_height);

        // Calculate aspect ratio of the render texture
        let render_aspect = 16.0 / 9.0; // TODO: get from renderer
        let viewport_aspect = available_width / available_height;

        let (viewport_width, viewport_height, viewport_x, viewport_y) =
            if render_aspect > viewport_aspect {
                // Render is wider than viewport
                let width = available_width;
                let height = width / render_aspect;
                let x = left - right;
                let y = bottom - top;
                (width / screen_width, height / screen_height, x / screen_width, y / screen_height)
            } else {
                // Render is taller than viewport
                let height = available_height;
                let width = height * render_aspect;
                let x = left - right;
                let y = bottom - top;
                (width / screen_width, height / screen_height, x / screen_width, y / screen_height)
            };
        state.viewport = Viewport {
            width: viewport_width,
            height: viewport_height,
            x: viewport_x,
            y: viewport_y,
        };
        // dbg!(state.viewport);

        egui::Window::new(format!("{}", self.meta.name))
            .resizable(true)
            .vscroll(true)
            .default_open(false)
            .show(state.egui_renderer.context(), |ui| {
                ui.label("Label!");

                if ui.button("Button!").clicked() {}

                ui.separator();
                ui.horizontal(|ui| {
                    ui.label(format!(
                        "Pixels per point: {}",
                        state.egui_renderer.context().pixels_per_point()
                    ));
                    if ui.button("-").clicked() {
                        state.scale_factor = (state.scale_factor - 0.1).max(0.3);
                    }
                    if ui.button("+").clicked() {
                        state.scale_factor = (state.scale_factor + 0.1).min(3.0);
                    }
                });
            });
    }
}

impl AppState {
    fn new(ctx: &RanimContext, scene: impl Scene) -> Self {
        let meta = scene.meta();
        let timeline = build_timeline(scene);
        let renderer = Renderer::new(ctx, 8.0, 1920, 1080);
        Self {
            meta,
            timeline,
            renderer,
            current_sec: 0.0,
            last_sec: -1.0,
            render_instances: RenderInstances::default(),
        }
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

    async fn set_window(&mut self, window: Window) {
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
            state.egui_renderer.begin_frame(window);
            self.app_state.ui(state);
            state.egui_renderer.end_frame_and_draw(
                &self.ctx.wgpu_ctx.device,
                &self.ctx.wgpu_ctx.queue,
                &mut encoder,
                window,
                &surface_view,
                screen_descriptor,
            );
        }

        self.ctx.wgpu_ctx.queue.submit(Some(encoder.finish()));
        surface_texture.present();
    }
}

impl ApplicationHandler for WinitApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window = event_loop
            .create_window(Window::default_attributes())
            .unwrap();
        pollster::block_on(self.set_window(window));
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

pub fn run_scene_app(scene: impl Scene) {
    let event_loop = winit::event_loop::EventLoop::new().unwrap();
    event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);

    let mut app = WinitApp::new(scene);

    event_loop.run_app(&mut app).expect("Failed to run app");
}
