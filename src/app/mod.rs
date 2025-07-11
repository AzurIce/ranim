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
    event_loop::{ActiveEventLoop, EventLoop, EventLoopProxy},
    window::{Window, WindowId},
};

use crate::{
    Scene, SceneMeta,
    animation::EvalResult,
    app::egui_tools::EguiRenderer,
    build_timeline,
    render::{
        Renderer,
        pipelines::app::{AppPipeline, Viewport},
        primitives::RenderInstances,
    },
    timeline::{SealedRanimScene, TimelineEvalResult},
    utils::wgpu::WgpuContext,
};

#[derive(Default, Debug, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
struct OccupiedScreenSpace {
    top: f32,
    bottom: f32,
    left: f32,
    right: f32,
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

#[allow(unused)]
struct AppState {
    meta: SceneMeta,
    timeline: SealedRanimScene,
    // app_options: AppOptions<'a>,
    // timeline: RanimTimeline,
    // renderer: Renderer,
    last_sec: f64,
    render_instances: RenderInstances,
    timeline_state: TimelineState,
}

impl AppState {
    fn new(scene: impl Scene) -> Self {
        let meta = scene.meta();
        let timeline = build_timeline(scene);
        let timeline_infos = timeline.get_timeline_infos();
        // let renderer = Renderer::new(ctx, 8.0, 1920, 1080);
        Self {
            meta,
            // renderer,
            last_sec: -1.0,
            render_instances: RenderInstances::default(),
            timeline_state: TimelineState::new(timeline.total_secs(), timeline_infos),
            timeline,
        }
    }
    pub fn prepare(&mut self, ctx: &WgpuContext, app_renderer: &mut AppRenderer) {
        #[cfg(feature = "profiling")]
        profiling::scope!("frame");

        // app_renderer
        //     .app_pipeline
        //     .bind_group
        //     .update_viewport(&ctx.queue, app_renderer.viewport);

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
                .map(|(id, res, _, _)| {
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
                renderable.prepare_for_id(ctx, &mut self.render_instances, *id);
            });
            ctx.queue.submit([]);
        }

        let render_primitives = visual_items
            .iter()
            .filter_map(|(id, _, _, _)| self.render_instances.get_render_instance_dyn(*id))
            .collect::<Vec<_>>();
        let camera_frame = match &camera_frame.0 {
            EvalResult::Dynamic(res) => res,
            EvalResult::Static(res) => res,
        };
        // println!("{:?}", camera_frame);
        // println!("{}", render_primitives.len());
        app_renderer
            .ranim_renderer
            .update_uniforms(ctx, camera_frame);

        {
            #[cfg(feature = "profiling")]
            profiling::scope!("render");

            app_renderer.ranim_renderer.render(ctx, &render_primitives);
        }
    }

    #[allow(clippy::field_reassign_with_default)]
    pub fn ui(&mut self, app_renderer: &mut AppRenderer) -> OccupiedScreenSpace {
        // let scale_factor = app_renderer.scale_factor;
        let mut occupied_screen_space = OccupiedScreenSpace::default();

        occupied_screen_space.bottom = egui::TopBottomPanel::bottom("bottom_panel")
            .resizable(true)
            .max_height(600.0)
            .show(app_renderer.egui_renderer.context(), |ui| {
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
            * app_renderer.egui_renderer.context().pixels_per_point();

        occupied_screen_space
    }
}

// in resume: create the window, and launch the async task to init WgpuContext
// the WgpuContext will be sent through event loop proxy as user event
struct WinitApp {
    event_loop_proxy: Option<EventLoopProxy<WgpuContext>>,
    app_state: AppState,

    size: (u32, u32),
    window: Option<Arc<Window>>,
    app_renderer: Option<AppRenderer>,
    wgpu_ctx: Option<WgpuContext>,
}

impl WinitApp {
    fn new(scene: impl Scene, event_loop: &EventLoop<WgpuContext>) -> Self {
        Self {
            event_loop_proxy: Some(event_loop.create_proxy()),
            app_state: AppState::new(scene),

            size: (0, 0),
            window: None,
            app_renderer: None,
            wgpu_ctx: None,
        }
    }
}

// MARK: Redraw
fn redraw(
    wgpu_ctx: &WgpuContext,
    app_state: &mut AppState,
    window: &Window,
    app_renderer: &mut AppRenderer,
) {
    #[cfg(feature = "profiling")]
    profiling::scope!("redraw");

    // Attempt to handle minimizing window
    if let Some(min) = window.is_minimized() {
        if min {
            println!("Window is minimized");
            return;
        }
    }

    let screen_descriptor = ScreenDescriptor {
        size_in_pixels: [
            app_renderer.surface_config.width,
            app_renderer.surface_config.height,
        ],
        pixels_per_point: window.scale_factor() as f32,
    };

    let surface_texture = app_renderer.surface.get_current_texture();

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

    let mut encoder = wgpu_ctx
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

    // MARK: app render
    {
        #[cfg(feature = "profiling")]
        profiling::scope!("app render");

        app_state.prepare(wgpu_ctx, app_renderer);
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

        render_pass.set_pipeline(&app_renderer.app_pipeline);
        render_pass.set_bind_group(0, app_renderer.app_pipeline.bind_group.as_ref(), &[]);
        render_pass.draw(0..4, 0..1);
    }

    // MARK: egui render
    {
        #[cfg(feature = "profiling")]
        profiling::scope!("egui render");

        app_renderer.egui_renderer.begin_frame(window);
        let occupied_screen_space = app_state.ui(app_renderer);
        let viewport = app_renderer.calc_viewport(occupied_screen_space, 16.0 / 9.0); // TODO: get from renderer
        app_renderer
            .app_pipeline
            .bind_group
            .update_viewport(&wgpu_ctx.queue, viewport);

        app_renderer.egui_renderer.end_frame_and_draw(
            &wgpu_ctx.device,
            &wgpu_ctx.queue,
            &mut encoder,
            window,
            &surface_view,
            screen_descriptor,
        );
    }

    {
        #[cfg(feature = "profiling")]
        profiling::scope!("submit");

        wgpu_ctx.queue.submit(Some(encoder.finish()));
    }
    {
        #[cfg(feature = "profiling")]
        profiling::scope!("present");

        surface_texture.present();
    }

    #[cfg(feature = "profiling")]
    profiling::finish_frame!();
}

// MARK: Resize
fn resize(ctx: &WgpuContext, app_renderer: &mut AppRenderer, size: PhysicalSize<u32>) {
    if size.width == 0 || size.height == 0 {
        log::warn!("[resize]: ignored resize to value <= 0: {size:?}");
        return;
    }
    log::info!("[resize]: {size:?}");
    {
        app_renderer.surface_config.width = size.width;
        app_renderer.surface_config.height = size.height;
        app_renderer
            .surface
            .configure(&ctx.device, &app_renderer.surface_config);
    }
}

impl ApplicationHandler<WgpuContext> for WinitApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        #[allow(unused)]
        let mut window_attrs = Window::default_attributes();
        #[allow(unused)]
        let (mut width, mut height) = (0, 0);

        #[cfg(target_arch = "wasm32")]
        {
            use wasm_bindgen::JsCast;
            use winit::platform::web::WindowAttributesExtWebSys;

            let window = web_sys::window().unwrap();
            let document = window.document().unwrap();
            let canvas = document
                .get_element_by_id(&format!("app-{}", self.app_state.meta.name))
                .unwrap();
            let canvas = canvas.dyn_into::<web_sys::HtmlCanvasElement>().ok();

            log::info!("found canvas: {}", canvas.is_some());
            if let Some(canvas) = canvas.as_ref() {
                self.size = (canvas.width(), canvas.height());
                log::info!("canvas size: {:?}", self.size);
            }
            window_attrs = window_attrs.with_canvas(canvas);

            // window_attrs =
            //     window_attrs.with_prevent_default(window.prevent_default_event_handling);
            // window_attrs = winit_window_attrs.with_append(true);
        }

        log::info!("creating window...");
        let Ok(window) = event_loop.create_window(window_attrs) else {
            log::error!("failed to create window");
            return;
        };
        log::info!("window size: {:?}", window.inner_size());
        #[cfg(not(target_arch = "wasm32"))]
        {
            let size = window.inner_size();
            self.size = (size.width, size.height);
        }
        let window = Arc::new(window);
        self.window.replace(window.clone());

        // Init context
        let Some(event_loop_proxy) = self.event_loop_proxy.take() else {
            return;
        };
        log::info!("initializing wgpu ctx...");
        let init_wgpu_ctx = async move {
            let renderer = WgpuContext::new().await;
            assert!(event_loop_proxy.send_event(renderer).is_ok());
        };
        #[cfg(not(target_arch = "wasm32"))]
        {
            pollster::block_on(init_wgpu_ctx);
        }
        #[cfg(target_arch = "wasm32")]
        {
            wasm_bindgen_futures::spawn_local(init_wgpu_ctx);
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        if let WindowEvent::CloseRequested = event {
            event_loop.exit();
            return;
        }

        let (Some(window), Some(wgpu_ctx), Some(app_renderer)) = (
            self.window.as_ref(),
            self.wgpu_ctx.as_ref(),
            self.app_renderer.as_mut(),
        ) else {
            return;
        };
        let app_state = &mut self.app_state;

        if app_renderer
            .egui_renderer
            .handle_input(window, &event)
            .consumed
        {
            return;
        }

        match event {
            WindowEvent::RedrawRequested => redraw(wgpu_ctx, app_state, window, app_renderer),
            WindowEvent::Resized(size) => resize(wgpu_ctx, app_renderer, size),
            _ => (),
        }
        self.window.as_ref().unwrap().request_redraw();
    }
    fn user_event(&mut self, _event_loop: &ActiveEventLoop, wgpu_ctx: WgpuContext) {
        let mut app_renderer = AppRenderer::new(&wgpu_ctx, self.window.clone().unwrap(), self.size);
        log::info!("app_renderer initialized");
        if let Some(window) = self.window.as_ref() {
            let size = window.inner_size();
            log::info!("window size: {size:?}");
            resize(&wgpu_ctx, &mut app_renderer, size)
        }
        self.app_renderer.replace(app_renderer);
        self.wgpu_ctx.replace(wgpu_ctx);
    }
}

#[cfg(feature = "profiling")]
use crate::PUFFIN_GPU_PROFILER;

/// Runs a scene preview app on `scene`
pub fn run_scene_app(scene: impl Scene) {
    #[cfg(target_arch = "wasm32")]
    {
        std::panic::set_hook(Box::new(console_error_panic_hook::hook));
        console_log::init().expect("Failed to initialize console_log");
    }

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

    let event_loop = winit::event_loop::EventLoop::<WgpuContext>::with_user_event()
        .build()
        .unwrap();
    event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);

    #[allow(unused_mut)]
    let mut app = WinitApp::new(scene, &event_loop);

    #[cfg(target_arch = "wasm32")]
    {
        use winit::platform::web::EventLoopExtWebSys;
        event_loop.spawn_app(app);
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        event_loop.run_app(&mut app).unwrap();
    }
}

#[allow(unused)]
pub(crate) struct AppRenderer {
    surface: wgpu::Surface<'static>,
    surface_config: wgpu::SurfaceConfiguration,
    egui_renderer: EguiRenderer,
    ranim_renderer: Renderer,
    sampler: wgpu::Sampler,
    app_pipeline: AppPipeline,
    // viewport: Viewport,
}

impl AppRenderer {
    fn new(ctx: &WgpuContext, window: Arc<Window>, size: (u32, u32)) -> Self {
        let surface = ctx.instance.create_surface(window.clone()).unwrap();

        // let swapchain_capabilities = surface.get_capabilities(&ctx.adapter);
        // let selected_format = wgpu::TextureFormat::Bgra8UnormSrgb;
        // let swapchain_format = swapchain_capabilities
        //     .formats
        //     .iter()
        //     .find(|d| **d == selected_format)
        //     .expect("failed to select proper surface texture format!");

        let surface_config = surface
            .get_default_config(&ctx.adapter, size.0, size.1)
            .expect("failed to get surface config");
        // let surface_config = wgpu::SurfaceConfiguration {
        //     usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        //     format: *swapchain_format,
        //     width: size.0,
        //     height: size.1,
        //     present_mode: wgpu::PresentMode::AutoVsync,
        //     desired_maximum_frame_latency: 0,
        //     alpha_mode: swapchain_capabilities.alpha_modes[0],
        //     view_formats: vec![],
        // };

        surface.configure(&ctx.device, &surface_config);

        let egui_renderer =
            egui_tools::EguiRenderer::new(&ctx.device, surface_config.format, None, 1, &window);
        let ranim_renderer = Renderer::new(ctx, 8.0, 1280, 720);

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
            #[cfg(target_arch = "wasm32")]
            &ranim_renderer.render_textures.linear_render_view,
            #[cfg(not(target_arch = "wasm32"))]
            &ranim_renderer.render_textures.render_view,
            &sampler,
            surface_config.format.into(),
        );
        Self {
            surface,
            surface_config,
            egui_renderer,
            ranim_renderer,
            sampler,
            app_pipeline,
            // viewport: Viewport {
            //     width: 1.0,
            //     height: 1.0,
            //     x: 0.0,
            //     y: 0.0
            // }
        }
    }
    fn calc_viewport(
        &self,
        occupied_screen_space: OccupiedScreenSpace,
        render_aspect: f32,
    ) -> Viewport {
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
        Viewport {
            width: viewport_width,
            height: viewport_height,
            x: viewport_x,
            y: viewport_y,
        }
    }
}
