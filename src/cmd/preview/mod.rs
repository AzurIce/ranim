mod depth_visual;
mod timeline;

use crate::{
    Scene, SceneConstructor,
    core::{
        SealedRanimScene,
        color::{self, LinearSrgb},
        store::CoreItemStore,
    },
    render::{
        Renderer,
        resource::{RenderPool, RenderTextures},
        utils::WgpuContext,
    },
};
use async_channel::{Receiver, Sender, unbounded};
use depth_visual::DepthVisualPipeline;
use eframe::egui;
use timeline::TimelineState;
use tracing::{error, info};
use web_time::Instant;

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

pub enum RanimPreviewAppCmd {
    ReloadScene(Scene, Sender<()>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    Output,
    Depth,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Resolution {
    pub width: u32,
    pub height: u32,
    pub aspect_ratio: (u32, u32),
}

impl Resolution {
    pub const fn new(width: u32, height: u32, aspect_ratio: (u32, u32)) -> Self {
        Self {
            width,
            height,
            aspect_ratio,
        }
    }

    pub fn ratio(&self) -> f32 {
        self.width as f32 / self.height as f32
    }

    pub fn aspect_ratio_str(&self) -> String {
        format!("{}:{}", self.aspect_ratio.0, self.aspect_ratio.1)
    }
}

// Common resolutions with aspect ratios
impl Resolution {
    // 16:9
    pub const HD: Self = Self::new(1280, 720, (16, 9));
    pub const FHD: Self = Self::new(1920, 1080, (16, 9));
    pub const QHD: Self = Self::new(2560, 1440, (16, 9));
    pub const _4K: Self = Self::new(3840, 2160, (16, 9));
    // 16:10
    pub const WXGA: Self = Self::new(1280, 800, (16, 10));
    pub const WUXGA: Self = Self::new(1920, 1200, (16, 10));
    // 4:3
    pub const SVGA: Self = Self::new(800, 600, (4, 3));
    pub const XGA: Self = Self::new(1024, 768, (4, 3));
    pub const SXGA: Self = Self::new(1280, 960, (4, 3));
    // 1:1
    pub const _1K_SQUARE: Self = Self::new(1080, 1080, (1, 1));
    pub const _2K_SQUARE: Self = Self::new(2160, 2160, (1, 1));
    // 21:9
    pub const UW_QHD: Self = Self::new(3440, 1440, (21, 9));
}

pub struct RanimPreviewApp {
    cmd_rx: Receiver<RanimPreviewAppCmd>,
    pub cmd_tx: Sender<RanimPreviewAppCmd>,
    #[allow(unused)]
    title: String,
    clear_color: wgpu::Color,
    resolution: Resolution,
    timeline: SealedRanimScene,
    need_eval: bool,
    last_sec: f64,
    store: CoreItemStore,
    pool: RenderPool,
    timeline_state: TimelineState,
    play_prev_t: Option<Instant>,

    // Rendering
    renderer: Option<Renderer>,
    render_textures: Option<RenderTextures>,
    texture_id: Option<egui::TextureId>,
    depth_texture_id: Option<egui::TextureId>,
    view_mode: ViewMode,
    wgpu_ctx: Option<WgpuContext>,
    last_render_time: Option<std::time::Duration>,
    last_eval_time: Option<std::time::Duration>,

    // Depth Visual
    depth_visual_pipeline: Option<DepthVisualPipeline>,
    depth_visual_texture: Option<wgpu::Texture>,
    depth_visual_view: Option<wgpu::TextureView>,

    // Resolution changed flag
    resolution_dirty: bool,
}

impl RanimPreviewApp {
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
            resolution: Resolution::QHD,
            timeline_state: TimelineState::new(timeline.total_secs(), timeline_infos),
            timeline,
            need_eval: false,
            last_sec: -1.0,
            store: CoreItemStore::default(),
            pool: RenderPool::new(),
            play_prev_t: None,
            renderer: None,
            render_textures: None,
            texture_id: None,
            depth_texture_id: None,
            view_mode: ViewMode::Output,
            wgpu_ctx: None,
            last_render_time: None,
            last_eval_time: None,
            depth_visual_pipeline: None,
            depth_visual_texture: None,
            depth_visual_view: None,
            resolution_dirty: false,
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

    /// Set preview resolution
    pub fn set_resolution(&mut self, resolution: Resolution) {
        if self.resolution != resolution {
            self.resolution = resolution;
            self.resolution_dirty = true;
        }
    }

    /// Calculate OIT layers based on resolution to stay within GPU buffer limits
    fn calculate_oit_layers(&self, ctx: &WgpuContext, width: u32, height: u32) -> usize {
        const BYTES_PER_PIXEL_PER_LAYER: usize = 8; // 4 bytes color + 4 bytes depth
        const MAX_OIT_LAYERS: usize = 8;

        let limits = ctx.device.limits();
        let max_buffer_size = limits.max_storage_buffer_binding_size as usize;
        let pixel_count = (width * height) as usize;
        let max_layers_by_buffer = max_buffer_size / (pixel_count * BYTES_PER_PIXEL_PER_LAYER);
        let oit_layers = max_layers_by_buffer.clamp(1, MAX_OIT_LAYERS);

        if oit_layers < MAX_OIT_LAYERS {
            tracing::warn!(
                "OIT layers reduced from {} to {} due to GPU buffer size limit ({}MB @ {}x{})",
                MAX_OIT_LAYERS,
                oit_layers,
                max_buffer_size / 1024 / 1024,
                width,
                height
            );
        }

        oit_layers
    }

    fn handle_events(&mut self) {
        if let Ok(cmd) = self.cmd_rx.try_recv() {
            match cmd {
                RanimPreviewAppCmd::ReloadScene(scene, tx) => {
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

                    self.set_clear_color_str(&scene.config.clear_color);

                    if let Err(err) = tx.try_send(()) {
                        error!("Failed to send reloaded signal: {err:?}");
                    }
                }
            }
        }
    }

    fn prepare_renderer(&mut self, frame: &eframe::Frame) {
        // Check if we need to recreate renderer
        let needs_init = self.renderer.is_none();
        let needs_resize = self.resolution_dirty && self.renderer.is_some();

        if !needs_init && !needs_resize {
            return;
        }

        let Some(render_state) = frame.wgpu_render_state() else {
            tracing::info!("frame.wgpu_render_state() is none");
            tracing::info!("{:?}", frame.info());
            return;
        };

        if needs_init {
            tracing::info!("preparing renderer...");
        } else if needs_resize {
            tracing::info!("recreating renderer for resolution change...");
        }

        // Construct WgpuContext using eframe's resources.
        // NOTE: We assume ranim-render doesn't strictly depend on the instance for the operations we do here.
        let ctx = WgpuContext {
            instance: wgpu::Instance::default(), // Dummy instance
            adapter: wgpu::Adapter::clone(&render_state.adapter),
            device: wgpu::Device::clone(&render_state.device),
            queue: wgpu::Queue::clone(&render_state.queue),
        };

        let (width, height) = (self.resolution.width, self.resolution.height);
        let oit_layers = self.calculate_oit_layers(&ctx, width, height);
        let renderer = Renderer::new(&ctx, width, height, oit_layers);
        let render_textures = renderer.new_render_textures(&ctx);

        // Init Depth Visual Pipeline
        if self.depth_visual_pipeline.is_none() {
            self.depth_visual_pipeline = Some(DepthVisualPipeline::new(&ctx));
        }

        // Create Depth Visual Texture
        let depth_visual_texture = ctx.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Depth Visual Texture"),
            size: wgpu::Extent3d {
                width: render_textures.width(),
                height: render_textures.height(),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let depth_visual_view =
            depth_visual_texture.create_view(&wgpu::TextureViewDescriptor::default());

        // Register texture with egui
        let texture_view = &render_textures.linear_render_view;
        let texture_id = render_state.renderer.write().register_native_texture(
            &render_state.device,
            texture_view,
            wgpu::FilterMode::Linear,
        );
        let depth_id = render_state.renderer.write().register_native_texture(
            &render_state.device,
            &depth_visual_view,
            wgpu::FilterMode::Nearest,
        );

        self.texture_id = Some(texture_id);
        self.depth_texture_id = Some(depth_id);
        self.depth_visual_texture = Some(depth_visual_texture);
        self.depth_visual_view = Some(depth_visual_view);
        self.render_textures = Some(render_textures);
        self.renderer = Some(renderer);
        self.wgpu_ctx = Some(ctx);
        self.resolution_dirty = false;
        self.need_eval = true; // Force re-render with new resolution
    }

    fn render_animation(&mut self) {
        if let (Some(ctx), Some(renderer), Some(render_textures)) = (
            self.wgpu_ctx.as_ref(),
            self.renderer.as_mut(),
            self.render_textures.as_mut(),
        ) {
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
            renderer.render_store_with_pool(
                ctx,
                render_textures,
                self.clear_color,
                &self.store,
                &mut self.pool,
            );

            if let (Some(pipeline), Some(view)) = (
                self.depth_visual_pipeline.as_ref(),
                self.depth_visual_view.as_ref(),
            ) {
                let mut encoder =
                    ctx.device
                        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                            label: Some("Depth Visual Encoder"),
                        });

                let bind_group = ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("Depth Visual Bind Group"),
                    layout: &pipeline.bind_group_layout,
                    entries: &[wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(
                            &render_textures.depth_texture_view,
                        ),
                    }],
                });

                {
                    let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("Depth Visual Pass"),
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view,
                            resolve_target: None,
                            depth_slice: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                                store: wgpu::StoreOp::Store,
                            },
                        })],
                        depth_stencil_attachment: None,
                        timestamp_writes: None,
                        occlusion_query_set: None,
                    });
                    rpass.set_pipeline(&pipeline.pipeline);
                    rpass.set_bind_group(0, &bind_group, &[]);
                    rpass.draw(0..3, 0..1);
                }
                ctx.queue.submit(Some(encoder.finish()));
            }

            self.last_render_time = Some(start.elapsed());
            self.pool.clean();
        }
    }
}

impl eframe::App for RanimPreviewApp {
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

                // Resolution selector
                {
                    let resolution = self.resolution;
                    egui::ComboBox::from_label("Resolution")
                        .selected_text(format!(
                            "{}x{} ({})",
                            resolution.width,
                            resolution.height,
                            resolution.aspect_ratio_str()
                        ))
                        .show_ui(ui, |ui| {
                            // 16:9
                            ui.label(egui::RichText::new("16:9").strong());
                            ui.selectable_value(
                                &mut self.resolution,
                                Resolution::HD,
                                "1280x720 (HD)",
                            );
                            ui.selectable_value(
                                &mut self.resolution,
                                Resolution::FHD,
                                "1920x1080 (FHD)",
                            );
                            ui.selectable_value(
                                &mut self.resolution,
                                Resolution::QHD,
                                "2560x1440 (QHD)",
                            );
                            ui.selectable_value(
                                &mut self.resolution,
                                Resolution::_4K,
                                "3840x2160 (4K)",
                            );
                            ui.separator();
                            // 16:10
                            ui.label(egui::RichText::new("16:10").strong());
                            ui.selectable_value(
                                &mut self.resolution,
                                Resolution::WXGA,
                                "1280x800 (WXGA)",
                            );
                            ui.selectable_value(
                                &mut self.resolution,
                                Resolution::WUXGA,
                                "1920x1200 (WUXGA)",
                            );
                            ui.separator();
                            // 4:3
                            ui.label(egui::RichText::new("4:3").strong());
                            ui.selectable_value(
                                &mut self.resolution,
                                Resolution::SVGA,
                                "800x600 (SVGA)",
                            );
                            ui.selectable_value(
                                &mut self.resolution,
                                Resolution::XGA,
                                "1024x768 (XGA)",
                            );
                            ui.selectable_value(
                                &mut self.resolution,
                                Resolution::SXGA,
                                "1280x960 (SXGA)",
                            );
                            ui.separator();
                            // 1:1
                            ui.label(egui::RichText::new("1:1").strong());
                            ui.selectable_value(
                                &mut self.resolution,
                                Resolution::_1K_SQUARE,
                                "1080x1080",
                            );
                            ui.selectable_value(
                                &mut self.resolution,
                                Resolution::_2K_SQUARE,
                                "2160x2160",
                            );
                            ui.separator();
                            // 21:9
                            ui.label(egui::RichText::new("21:9").strong());
                            ui.selectable_value(
                                &mut self.resolution,
                                Resolution::UW_QHD,
                                "3440x1440 (UW-QHD)",
                            );
                        });
                    if self.resolution != resolution {
                        self.resolution_dirty = true;
                    }
                }

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

                    ui.separator();
                    ui.selectable_value(&mut self.view_mode, ViewMode::Output, "Output");
                    ui.selectable_value(&mut self.view_mode, ViewMode::Depth, "Depth");
                    ui.separator();

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
            let texture_id = match self.view_mode {
                ViewMode::Output => self.texture_id,
                ViewMode::Depth => self.depth_texture_id,
            };

            if let Some(tid) = texture_id {
                // Maintain aspect ratio
                // TODO: We could update renderer size here if we want dynamic resolution
                let available_size = ui.available_size();
                let aspect_ratio = self
                    .render_textures
                    .as_ref()
                    .map(|rt| rt.ratio())
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

pub fn run_app(app: RanimPreviewApp, #[cfg(target_arch = "wasm32")] container_id: String) {
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
    let app = RanimPreviewApp::new(scene, name.to_string());
    run_app(
        app,
        #[cfg(target_arch = "wasm32")]
        format!("ranim-app-{name}"),
    );
}

/// Preview a scene
pub fn preview_scene(scene: &Scene) {
    preview_scene_with_name(scene, &scene.name);
}

/// Preview a scene with a custom name
pub fn preview_scene_with_name(scene: &Scene, name: &str) {
    let mut app = RanimPreviewApp::new(scene.constructor, name.to_string());
    app.set_clear_color_str(&scene.config.clear_color);
    run_app(
        app,
        #[cfg(target_arch = "wasm32")]
        format!("ranim-app-{name}"),
    );
}

// WASM support needs refactoring, mostly keeping it commented or adapting basic entry point.
#[cfg(target_arch = "wasm32")]
mod wasm {
    use super::*;

    #[wasm_bindgen(start)]
    pub async fn wasm_start() {
        console_error_panic_hook::set_once();
        wasm_tracing::set_as_global_default();
    }

    /// WASM wrapper: preview a scene (accepts owned [`Scene`] from `find_scene`)
    #[wasm_bindgen]
    pub fn preview_scene(scene: &Scene) {
        super::preview_scene(scene);
    }
}
