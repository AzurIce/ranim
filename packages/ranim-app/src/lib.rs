mod timeline;

use async_channel::{Receiver, Sender, unbounded};
use eframe::egui;
use ranim_core::color::LinearSrgb;
use ranim_core::store::CoreItemStore;
use ranim_core::{Scene, SceneConstructor, SealedRanimScene, color};
use ranim_render::Renderer;
use ranim_render::resource::RenderPool;
use ranim_render::utils::WgpuContext;
use timeline::TimelineState;
use tracing::{error, info};
use web_time::Instant;
use wgpu;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

// Copied from original lib.rs
pub struct TimelineInfoState {
    pub ctx: egui::Context,
    pub canvas: egui::Rect,
    pub response: egui::Response,
    pub painter: egui::Painter,
    pub text_height: f32,
    pub font_id: egui::FontId,
}

impl TimelineInfoState {
    pub fn point_from_ms(&self, state: &TimelineState, ms: i64) -> f32 {
        let ms = ms as f32;
        let offset = state.offset_points;
        let width_sec = state.width_sec as f32;
        let canvas_width = self.canvas.width();

        let ms_per_pixel = width_sec * 1000.0 / canvas_width;
        let x = ms / ms_per_pixel;
        self.canvas.min.x + x - offset
    }
}

pub enum AppCmd {
    ReloadScene(Scene, Sender<()>),
}

pub struct RanimApp {
    cmd_rx: Receiver<AppCmd>,
    pub cmd_tx: Sender<AppCmd>,
    #[allow(unused)]
    title: String,
    clear_color: wgpu::Color,
    timeline: SealedRanimScene,
    need_eval: bool,
    last_sec: f64,
    store: CoreItemStore,
    pool: RenderPool,
    timeline_state: TimelineState,
    play_prev_t: Option<Instant>,

    // Rendering
    renderer: Option<Renderer>,
    texture_id: Option<egui::TextureId>,
    wgpu_ctx: Option<WgpuContext>,
    last_render_time: Option<std::time::Duration>,
    last_eval_time: Option<std::time::Duration>,
}

impl RanimApp {
    pub fn new(scene_constructor: impl SceneConstructor, title: String) -> Self {
        let t = Instant::now();
        info!("building scene...");
        let timeline = scene_constructor.build_scene();
        info!("Scene built, cost: {:?}", t.elapsed());

        info!("Getting timelines info...");
        let timeline_infos = timeline.get_timeline_infos();
        info!("Total {} timelines", timeline_infos.len());

        let (cmd_tx, cmd_rx) = unbounded();

        Self {
            cmd_rx,
            cmd_tx,
            title,
            clear_color: wgpu::Color::TRANSPARENT,
            timeline_state: TimelineState::new(timeline.total_secs(), timeline_infos),
            timeline,
            need_eval: false,
            last_sec: -1.0,
            store: CoreItemStore::default(),
            pool: RenderPool::new(),
            play_prev_t: None,
            renderer: None,
            texture_id: None,
            wgpu_ctx: None,
            last_render_time: None,
            last_eval_time: None,
        }
    }

    /// Set clear color str
    pub fn set_clear_color_str(&mut self, color: &str) {
        let bg = color::try_color(color)
            .unwrap_or(color::color("#333333ff"))
            .convert::<LinearSrgb>();
        let [r, g, b, a] = bg.components.map(|x| x as f64);
        let clear_color = wgpu::Color { r, g, b, a };
        self.set_clear_color(clear_color);
    }

    /// Set clear color
    pub fn set_clear_color(&mut self, color: wgpu::Color) {
        self.clear_color = color;
    }

    fn handle_events(&mut self) {
        if let Ok(cmd) = self.cmd_rx.try_recv() {
            match cmd {
                AppCmd::ReloadScene(scene, tx) => {
                    let timeline = scene.constructor.build_scene();
                    let timeline_infos = timeline.get_timeline_infos();
                    let old_cur_second = self.timeline_state.current_sec;
                    self.timeline_state = TimelineState::new(timeline.total_secs(), timeline_infos);
                    self.timeline_state.current_sec =
                        old_cur_second.clamp(0.0, self.timeline_state.total_sec);
                    self.timeline = timeline;
                    self.store.update(std::iter::empty());
                    self.pool.clean();
                    self.need_eval = true;

                    self.set_clear_color_str(scene.config.clear_color);

                    if let Err(err) = tx.try_send(()) {
                        error!("Failed to send reloaded signal: {err:?}");
                    }
                }
            }
        }
    }

    fn prepare_renderer(&mut self, frame: &eframe::Frame) {
        if self.renderer.is_some() {
            return;
        }

        tracing::info!("preparing renderer...");
        let Some(render_state) = frame.wgpu_render_state() else {
            tracing::info!("frame.wgpu_render_state() is none");
            tracing::info!("{:?}", frame.info());
            return;
        };

        tracing::info!("constructing renderer...");
        // Construct WgpuContext using eframe's resources.
        // NOTE: We assume ranim-render doesn't strictly depend on the instance for the operations we do here.
        let ctx = WgpuContext {
            instance: wgpu::Instance::default(), // Dummy instance
            adapter: wgpu::Adapter::clone(&render_state.adapter),
            device: wgpu::Device::clone(&render_state.device),
            queue: wgpu::Queue::clone(&render_state.queue),
        };

        let renderer = Renderer::new(&ctx, 2560, 1440, 8); // TODO: dynamic size

        // Register texture with egui
        let texture_view = &renderer.render_textures.linear_render_view;
        let id = render_state.renderer.write().register_native_texture(
            &render_state.device,
            texture_view,
            wgpu::FilterMode::Linear,
        );

        self.texture_id = Some(id);
        self.renderer = Some(renderer);
        self.wgpu_ctx = Some(ctx);
    }

    fn render_animation(&mut self) {
        if let (Some(ctx), Some(renderer)) = (self.wgpu_ctx.as_ref(), self.renderer.as_mut()) {
            if self.last_sec == self.timeline_state.current_sec && !self.need_eval {
                return;
            }
            self.need_eval = false;
            self.last_sec = self.timeline_state.current_sec;

            let start_eval = Instant::now();
            self.store
                .update(self.timeline.eval_at_sec(self.timeline_state.current_sec));
            self.last_eval_time = Some(start_eval.elapsed());

            let start = Instant::now();
            renderer.render_store_with_pool(ctx, self.clear_color, &self.store, &mut self.pool);
            self.last_render_time = Some(start.elapsed());
            self.pool.clean();
        }
    }
}

impl eframe::App for RanimApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        self.prepare_renderer(frame);
        self.handle_events();

        if let Some(play_prev_t) = self.play_prev_t {
            let elapsed = play_prev_t.elapsed().as_secs_f64();
            self.timeline_state.current_sec =
                (self.timeline_state.current_sec + elapsed).min(self.timeline_state.total_sec);
            if self.timeline_state.current_sec == self.timeline_state.total_sec {
                self.play_prev_t = None;
            } else {
                self.play_prev_t = Some(Instant::now());
                ctx.request_repaint(); // Animation loop
            }
        }

        self.render_animation();

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading(&self.title);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let dark_mode = ui.visuals().dark_mode;
                    let button_text = if dark_mode { "☀ Light" } else { "🌙 Dark" };
                    if ui.button(button_text).clicked() {
                        if dark_mode {
                            ctx.set_visuals(egui::Visuals::light());
                        } else {
                            ctx.set_visuals(egui::Visuals::dark());
                        }
                    }

                    if let Some(duration) = self.last_render_time {
                        ui.label(format!("Render: {:.2}ms", duration.as_secs_f64() * 1000.0));
                        ui.separator();
                    }
                    if let Some(duration) = self.last_eval_time {
                        ui.label(format!("Eval: {:.2}ms", duration.as_secs_f64() * 1000.0));
                        ui.separator();
                    }
                });
            });
        });

        egui::TopBottomPanel::bottom("bottom_panel")
            .resizable(true)
            .max_height(600.0)
            .show(ctx, |ui| {
                ui.label("Timeline");

                ui.horizontal(|ui| {
                    if ui.button("<<").clicked() {
                        self.timeline_state.current_sec = 0.0;
                    }
                    #[allow(clippy::collapsible_else_if)]
                    if self.play_prev_t.is_none() {
                        if ui.button("play").clicked() {
                            self.play_prev_t = Some(Instant::now());
                        }
                    } else {
                        if ui.button("pause").clicked() {
                            self.play_prev_t = None;
                        }
                    }
                    if ui.button(">>").clicked() {
                        self.timeline_state.current_sec = self.timeline_state.total_sec;
                    }
                    ui.style_mut().spacing.slider_width = ui.available_width() - 70.0;
                    ui.add(
                        egui::Slider::new(
                            &mut self.timeline_state.current_sec,
                            0.0..=self.timeline_state.total_sec,
                        )
                        .text("sec"),
                    );
                });

                self.timeline_state.ui_main_timeline(ui);
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(tid) = self.texture_id {
                // Maintain aspect ratio
                // TODO: We could update renderer size here if we want dynamic resolution
                let available_size = ui.available_size();
                let aspect_ratio = self
                    .renderer
                    .as_ref()
                    .map(|r| r.ratio())
                    .unwrap_or(1280.0 / 7.0);
                let mut size = available_size;

                if size.x / size.y > aspect_ratio {
                    size.x = size.y * aspect_ratio;
                } else {
                    size.y = size.x / aspect_ratio;
                }

                ui.centered_and_justified(|ui| {
                    ui.image(egui::load::SizedTexture::new(tid, size));
                });
            } else {
                ui.centered_and_justified(|ui| {
                    ui.spinner();
                });
            }
        });
    }
}

pub fn run_app(app: RanimApp, #[cfg(target_arch = "wasm32")] container_id: String) {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let native_options = eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_title(&app.title)
                .with_inner_size([1280.0, 720.0]),
            renderer: eframe::Renderer::Wgpu,
            ..Default::default()
        };

        // We need to clone title because run_native takes String (or &str) and app is moved into closure
        let title = app.title.clone();

        eframe::run_native(
            &title,
            native_options,
            Box::new(|_cc| {
                // If we wanted to access wgpu context on creation, we could do it here from _cc.wgpu_render_state
                Ok(Box::new(app))
            }),
        )
        .unwrap();
    }

    #[cfg(target_arch = "wasm32")]
    {
        use wasm_bindgen::JsCast;
        let web_options = eframe::WebOptions {
            ..Default::default()
        };

        // Handling canvas creation if not found to ensure compatibility
        let document = web_sys::window().unwrap().document().unwrap();
        let canvas = document
            .get_element_by_id(&container_id)
            .and_then(|c| c.dyn_into::<web_sys::HtmlCanvasElement>().ok());

        let canvas = if let Some(canvas) = canvas {
            canvas
        } else {
            let canvas = document.create_element("canvas").unwrap();
            canvas.set_id(&container_id);
            document.body().unwrap().append_child(&canvas).unwrap();
            canvas.dyn_into::<web_sys::HtmlCanvasElement>().unwrap()
        };

        wasm_bindgen_futures::spawn_local(async {
            eframe::WebRunner::new()
                .start(canvas, web_options, Box::new(|_cc| Ok(Box::new(app))))
                .await
                .expect("failed to start eframe");
        });
    }
}

pub fn preview_constructor_with_name(scene: impl SceneConstructor, name: &str) {
    let app = RanimApp::new(scene, name.to_string());
    run_app(
        app,
        #[cfg(target_arch = "wasm32")]
        format!("ranim-app-{name}"),
    );
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub fn preview_scene(scene: &Scene) {
    preview_scene_with_name(scene, scene.name);
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub fn preview_scene_with_name(scene: &Scene, name: &str) {
    let mut app = RanimApp::new(scene.constructor.clone(), name.to_string());
    app.set_clear_color_str(scene.config.clear_color);
    run_app(
        app,
        #[cfg(target_arch = "wasm32")]
        format!("ranim-app-{name}"),
    );
}

// WASM support needs refactoring, mostly keeping it commented or adapting basic entry point.
#[cfg(target_arch = "wasm32")]
mod wasm {
    use wasm_bindgen::prelude::*;

    #[wasm_bindgen(start)]
    pub async fn wasm_start() {
        console_error_panic_hook::set_once();
        wasm_tracing::set_as_global_default();
    }
}
