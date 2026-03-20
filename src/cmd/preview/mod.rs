mod depth_visual;
mod timeline;

use crate::{
    Output, Scene, SceneConfig, SceneConstructor,
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
#[cfg(all(not(target_family = "wasm"), feature = "render"))]
use crate::{OutputFormat, cmd::render::file_writer::OutputFormatExt};
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

#[cfg(all(not(target_family = "wasm"), feature = "render"))]
enum ExportProgress {
    /// (current_frame, total_frames)
    Progress(u64, u64),
    Done,
    Error(String),
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
}

impl Resolution {
    pub const fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }

    pub fn ratio(&self) -> f32 {
        self.width as f32 / self.height as f32
    }

    /// Calculate and return the simplified aspect ratio (e.g., (16, 9) for 1920x1080)
    pub fn aspect_ratio(&self) -> (u32, u32) {
        fn gcd(a: u32, b: u32) -> u32 {
            if b == 0 { a } else { gcd(b, a % b) }
        }
        let g = gcd(self.width, self.height);
        (self.width / g, self.height / g)
    }

    pub fn aspect_ratio_str(&self) -> String {
        let (w, h) = self.aspect_ratio();
        format!("{w}:{h}")
    }
}

// Common resolutions
impl Resolution {
    // 16:9
    pub const HD: Self = Self::new(1280, 720);
    pub const FHD: Self = Self::new(1920, 1080);
    pub const QHD: Self = Self::new(2560, 1440);
    pub const UHD: Self = Self::new(3840, 2160);
    // 16:10
    pub const WXGA: Self = Self::new(1280, 800);
    pub const WUXGA: Self = Self::new(1920, 1200);
    // 4:3
    pub const SVGA: Self = Self::new(800, 600);
    pub const XGA: Self = Self::new(1024, 768);
    pub const SXGA: Self = Self::new(1280, 960);
    // 1:1
    pub const _1K_SQUARE: Self = Self::new(1080, 1080);
    pub const _2K_SQUARE: Self = Self::new(2160, 2160);
    // 21:9
    pub const UW_QHD: Self = Self::new(3440, 1440);
}

pub struct RanimPreviewApp {
    cmd_rx: Receiver<RanimPreviewAppCmd>,
    pub cmd_tx: Sender<RanimPreviewAppCmd>,
    #[allow(unused)]
    title: String,
    clear_color: wgpu::Color,
    scene_constructor: fn(&mut crate::core::RanimScene),
    scene_config: SceneConfig,
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

    // Export
    #[cfg(all(not(target_family = "wasm"), feature = "render"))]
    export_dialog_open: bool,
    export_config: Output,
    #[cfg(all(not(target_family = "wasm"), feature = "render"))]
    export_progress_rx: Option<Receiver<ExportProgress>>,
    #[cfg(all(not(target_family = "wasm"), feature = "render"))]
    export_current_frame: u64,
    #[cfg(all(not(target_family = "wasm"), feature = "render"))]
    export_total_frames: u64,

    // Playback
    playback_speed: f64,
    looping: bool,
}

impl RanimPreviewApp {
    pub fn new(
        scene_constructor: fn(&mut crate::core::RanimScene),
        title: String,
        scene_config: SceneConfig,
    ) -> Self {
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
            scene_constructor,
            scene_config,
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
            #[cfg(all(not(target_family = "wasm"), feature = "render"))]
            export_dialog_open: false,
            export_config: Output::default(),
            #[cfg(all(not(target_family = "wasm"), feature = "render"))]
            export_progress_rx: None,
            #[cfg(all(not(target_family = "wasm"), feature = "render"))]
            export_current_frame: 0,
            #[cfg(all(not(target_family = "wasm"), feature = "render"))]
            export_total_frames: 0,
            playback_speed: 1.0,
            looping: false,
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

    #[cfg(all(not(target_family = "wasm"), feature = "render"))]
    fn start_export(&mut self, ctx: egui::Context) {
        let (progress_tx, progress_rx) = unbounded();
        self.export_progress_rx = Some(progress_rx);

        let constructor = self.scene_constructor;
        let scene_config = self.scene_config.clone();
        let output = self.export_config.clone();
        let name = self.title.clone();

        std::thread::spawn(move || {
            let progress_tx_cb = progress_tx.clone();
            let ctx_cb = ctx.clone();
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                crate::cmd::render::render_scene_output_with_progress(
                    constructor,
                    name,
                    &scene_config,
                    &output,
                    2,
                    Some(Box::new(move |current, total| {
                        let _ =
                            progress_tx_cb.send_blocking(ExportProgress::Progress(current, total));
                        ctx_cb.request_repaint();
                    })),
                );

                let _ = progress_tx.send_blocking(ExportProgress::Done);
                ctx.request_repaint();
            }));

            if let Err(e) = result {
                let msg = if let Some(s) = e.downcast_ref::<&str>() {
                    s.to_string()
                } else if let Some(s) = e.downcast_ref::<String>() {
                    s.clone()
                } else {
                    "Unknown export error".to_string()
                };
                let _ = progress_tx.send_blocking(ExportProgress::Error(msg));
                ctx.request_repaint();
            }
        });
    }
}

impl eframe::App for RanimPreviewApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        self.prepare_renderer(frame);
        self.handle_events();

        // Space bar toggles play/pause
        if ctx.input(|i| i.key_pressed(egui::Key::Space)) {
            if self.play_prev_t.is_some() {
                self.play_prev_t = None;
            } else {
                if self.timeline_state.current_sec >= self.timeline_state.total_sec {
                    self.timeline_state.current_sec = 0.0;
                }
                self.play_prev_t = Some(Instant::now());
            }
        }

        // Arrow keys step forward/back one frame
        {
            let frame_dur = 1.0 / self.export_config.fps as f64;
            if ctx.input(|i| i.key_pressed(egui::Key::ArrowLeft)) {
                self.play_prev_t = None;
                self.timeline_state.current_sec =
                    (self.timeline_state.current_sec - frame_dur).max(0.0);
            }
            if ctx.input(|i| i.key_pressed(egui::Key::ArrowRight)) {
                self.play_prev_t = None;
                self.timeline_state.current_sec = (self.timeline_state.current_sec + frame_dur)
                    .min(self.timeline_state.total_sec);
            }
        }

        if let Some(play_prev_t) = self.play_prev_t {
            let elapsed = play_prev_t.elapsed().as_secs_f64() * self.playback_speed;
            self.timeline_state.current_sec =
                (self.timeline_state.current_sec + elapsed).min(self.timeline_state.total_sec);
            if self.timeline_state.current_sec >= self.timeline_state.total_sec {
                if self.looping {
                    self.timeline_state.current_sec = 0.0;
                    self.play_prev_t = Some(Instant::now());
                    ctx.request_repaint();
                } else {
                    self.play_prev_t = None;
                }
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
                                Resolution::UHD,
                                "3840x2160 (UHD)",
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
                    let button_text = if dark_mode {
                        format!("{} Light", egui_phosphor::regular::SUN)
                    } else {
                        format!("{} Dark", egui_phosphor::regular::MOON)
                    };
                    if ui.button(button_text).clicked() {
                        if dark_mode {
                            ctx.set_visuals(egui::Visuals::light());
                        } else {
                            ctx.set_visuals(egui::Visuals::dark());
                        }
                    }

                    ui.separator();
                    #[cfg(all(not(target_family = "wasm"), feature = "render"))]
                    {
                        let exporting = self.export_progress_rx.is_some();
                        if ui
                            .add_enabled(!exporting, egui::Button::new("Export"))
                            .clicked()
                        {
                            self.export_dialog_open = true;
                        }
                        ui.separator();
                    }
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
                    let fps = self.export_config.fps as f64;
                    let frame_dur = 1.0 / fps;

                    // |< Jump to start
                    if ui
                        .button(egui_phosphor::regular::SKIP_BACK)
                        .on_hover_text("Jump to start")
                        .clicked()
                    {
                        self.timeline_state.current_sec = 0.0;
                        self.play_prev_t = None;
                    }

                    // < Step back one frame
                    if ui
                        .button(egui_phosphor::regular::CARET_LEFT)
                        .on_hover_text("Step back one frame")
                        .clicked()
                    {
                        self.play_prev_t = None;
                        self.timeline_state.current_sec =
                            (self.timeline_state.current_sec - frame_dur).max(0.0);
                    }

                    // Play / Pause
                    let is_playing = self.play_prev_t.is_some();
                    let play_label = if is_playing {
                        egui_phosphor::regular::PAUSE
                    } else {
                        egui_phosphor::regular::PLAY
                    };
                    let play_tooltip = if is_playing { "Pause" } else { "Play" };
                    if ui.button(play_label).on_hover_text(play_tooltip).clicked() {
                        if is_playing {
                            self.play_prev_t = None;
                        } else {
                            if self.timeline_state.current_sec >= self.timeline_state.total_sec {
                                self.timeline_state.current_sec = 0.0;
                            }
                            self.play_prev_t = Some(Instant::now());
                        }
                    }

                    // > Step forward one frame
                    if ui
                        .button(egui_phosphor::regular::CARET_RIGHT)
                        .on_hover_text("Step forward one frame")
                        .clicked()
                    {
                        self.play_prev_t = None;
                        self.timeline_state.current_sec = (self.timeline_state.current_sec
                            + frame_dur)
                            .min(self.timeline_state.total_sec);
                    }

                    // >| Jump to end
                    if ui
                        .button(egui_phosphor::regular::SKIP_FORWARD)
                        .on_hover_text("Jump to end")
                        .clicked()
                    {
                        self.timeline_state.current_sec = self.timeline_state.total_sec;
                        self.play_prev_t = None;
                    }

                    ui.separator();

                    // Loop toggle
                    let mut loop_btn = egui::Button::new(egui_phosphor::regular::ARROWS_CLOCKWISE);
                    if self.looping {
                        loop_btn = loop_btn.fill(ui.visuals().selection.bg_fill);
                    }
                    if ui
                        .add(loop_btn)
                        .on_hover_text(if self.looping {
                            "Looping: ON"
                        } else {
                            "Looping: OFF"
                        })
                        .clicked()
                    {
                        self.looping = !self.looping;
                    }

                    ui.separator();

                    // Speed control
                    let drag_speed = (self.playback_speed * 0.02).max(0.01);
                    ui.add(
                        egui::DragValue::new(&mut self.playback_speed)
                            .speed(drag_speed)
                            .range(0.1..=10.0)
                            .suffix("x"),
                    )
                    .on_hover_text("Playback speed");

                    ui.separator();

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

        // Export (native only)
        #[cfg(all(not(target_family = "wasm"), feature = "render"))]
        {
            // Poll export progress
            if let Some(rx) = &self.export_progress_rx {
                let mut done = false;
                let mut error_msg = None;

                while let Ok(msg) = rx.try_recv() {
                    match msg {
                        ExportProgress::Progress(current, total) => {
                            self.export_current_frame = current;
                            self.export_total_frames = total;
                        }
                        ExportProgress::Done => {
                            done = true;
                        }
                        ExportProgress::Error(err) => {
                            error_msg = Some(err);
                            done = true;
                        }
                    }
                }

                if done {
                    self.export_progress_rx = None;
                    self.export_current_frame = 0;
                    self.export_total_frames = 0;
                    if let Some(err) = error_msg {
                        error!("Export failed: {err}");
                    } else {
                        info!("Export completed");
                    }
                } else {
                    ctx.request_repaint();
                }
            }

            // Export configuration dialog
            let exporting = self.export_progress_rx.is_some();
            if self.export_dialog_open || exporting {
                let mut open = self.export_dialog_open;
                egui::Window::new("Export")
                    .open(&mut open)
                    .resizable(false)
                    .show(ctx, |ui| {
                        ui.add_enabled_ui(!exporting, |ui| {
                            egui::Grid::new("export_grid")
                                .num_columns(2)
                                .show(ui, |ui| {
                                    ui.label("Width:");
                                    ui.add(
                                        egui::DragValue::new(&mut self.export_config.width)
                                            .range(1..=7680),
                                    );
                                    ui.end_row();

                                    ui.label("Height:");
                                    ui.add(
                                        egui::DragValue::new(&mut self.export_config.height)
                                            .range(1..=4320),
                                    );
                                    ui.end_row();

                                    ui.label("FPS:");
                                    ui.add(
                                        egui::DragValue::new(&mut self.export_config.fps)
                                            .range(1..=240),
                                    );
                                    ui.end_row();

                                    ui.label("Format:");
                                    egui::ComboBox::from_id_salt("export_format")
                                        .selected_text(format!("{}", self.export_config.format))
                                        .show_ui(ui, |ui| {
                                            ui.selectable_value(
                                                &mut self.export_config.format,
                                                OutputFormat::Mp4,
                                                "mp4",
                                            );
                                            ui.selectable_value(
                                                &mut self.export_config.format,
                                                OutputFormat::Webm,
                                                "webm",
                                            );
                                            ui.selectable_value(
                                                &mut self.export_config.format,
                                                OutputFormat::Mov,
                                                "mov",
                                            );
                                            ui.selectable_value(
                                                &mut self.export_config.format,
                                                OutputFormat::Gif,
                                                "gif",
                                            );
                                        });
                                    ui.end_row();

                                    ui.label("Output dir:");
                                    ui.text_edit_singleline(&mut self.export_config.dir);
                                    ui.end_row();

                                    // Show resolved output path preview right below the dir input
                                    ui.label("");
                                    {
                                        let mut output_dir =
                                            std::path::PathBuf::from(&self.export_config.dir);
                                        if !output_dir.is_absolute() {
                                            output_dir = std::env::current_dir()
                                                .unwrap_or_default()
                                                .join(&output_dir);
                                        }
                                        let (_, _, ext) =
                                            self.export_config.format.encoding_params();
                                        let name = self
                                            .export_config
                                            .name
                                            .as_deref()
                                            .unwrap_or(&self.title);
                                        let file_path = output_dir.join(format!(
                                            "{}_{}x{}_{}.{ext}",
                                            name,
                                            self.export_config.width,
                                            self.export_config.height,
                                            self.export_config.fps,
                                        ));
                                        ui.label(
                                            egui::RichText::new(format!(
                                                "-> {}",
                                                file_path.display()
                                            ))
                                            .small()
                                            .color(ui.visuals().weak_text_color()),
                                        );
                                    }
                                    ui.end_row();

                                    ui.label("Save frames:");
                                    ui.checkbox(&mut self.export_config.save_frames, "");
                                    ui.end_row();
                                });
                        }); // end add_enabled_ui

                        ui.add_space(8.0);

                        // Show progress bar inline when exporting
                        if exporting {
                            let current = self.export_current_frame;
                            let total = self.export_total_frames;
                            if total > 0 {
                                let progress = current as f32 / total as f32;
                                ui.add(egui::ProgressBar::new(progress).text(format!(
                                    "{current}/{total} frames ({:.0}%)",
                                    progress * 100.0
                                )));
                            } else {
                                ui.horizontal(|ui| {
                                    ui.spinner();
                                    ui.label("Preparing...");
                                });
                            }
                        } else if ui.button("Start Export").clicked() {
                            self.start_export(ctx.clone());
                        }
                    });
                // Don't allow closing the window while exporting
                if !exporting {
                    self.export_dialog_open = open;
                }
            }
        }
    }
}

pub fn run_app(app: RanimPreviewApp, #[cfg(target_arch = "wasm32")] container_id: String) {
    #[cfg(not(target_family = "wasm"))]
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
            Box::new(|cc| {
                let mut fonts = egui::FontDefinitions::default();
                egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);
                cc.egui_ctx.set_fonts(fonts);
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

pub fn preview_constructor_with_name(
    scene: fn(&mut crate::core::RanimScene),
    name: &str,
    scene_config: &SceneConfig,
) {
    let app = RanimPreviewApp::new(scene, name.to_string(), scene_config.clone());
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
    let mut app = RanimPreviewApp::new(scene.constructor, name.to_string(), scene.config.clone());
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
