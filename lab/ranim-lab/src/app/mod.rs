use egui::{TextureId, Vec2};
use ranim::RenderSceneCoreExt;
use ranim_core::{
    CameraFrame, VItem,
    components::{rgba::Rgba, width::Width},
    glam::{dvec3, vec4},
    store::CoreItemStore,
};
use ranim_render::{Renderer, resource::RenderTextures, scene::RenderScene, utils::WgpuContext};

const DEFAULT_WIDTH: u32 = 1280;
const DEFAULT_HEIGHT: u32 = 720;

pub struct RanimLabApp {
    store: CoreItemStore,
    render_scene: RenderScene,
    renderer: Option<Renderer>,
    render_textures: Option<RenderTextures>,
    wgpu_ctx: Option<WgpuContext>,
    texture_id: Option<TextureId>,
    clear_color: wgpu::Color,
    render_dirty: bool,
}

impl RanimLabApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut fonts = egui::FontDefinitions::default();
        egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);
        cc.egui_ctx.set_fonts(fonts);

        Self {
            store: demo_store(),
            render_scene: RenderScene::new(),
            renderer: None,
            render_textures: None,
            wgpu_ctx: None,
            texture_id: None,
            clear_color: wgpu::Color {
                r: 0.08,
                g: 0.08,
                b: 0.08,
                a: 1.0,
            },
            render_dirty: true,
        }
    }

    fn prepare_renderer(&mut self, frame: &eframe::Frame) {
        if self.renderer.is_some() {
            return;
        }

        let Some(render_state) = frame.wgpu_render_state() else {
            tracing::info!("frame.wgpu_render_state() is none");
            return;
        };

        let ctx = WgpuContext::from_device(
            wgpu::Adapter::clone(&render_state.adapter),
            wgpu::Device::clone(&render_state.device),
            wgpu::Queue::clone(&render_state.queue),
        );
        let renderer = Renderer::new(&ctx, DEFAULT_WIDTH, DEFAULT_HEIGHT, 8);
        let render_textures = renderer.new_render_textures(&ctx);
        let texture_id = render_state.renderer.write().register_native_texture(
            &render_state.device,
            &render_textures.linear_render_view,
            wgpu::FilterMode::Linear,
        );

        self.texture_id = Some(texture_id);
        self.render_textures = Some(render_textures);
        self.renderer = Some(renderer);
        self.wgpu_ctx = Some(ctx);
        self.render_dirty = true;
    }

    fn render_if_needed(&mut self) {
        if !self.render_dirty {
            return;
        }

        let (Some(ctx), Some(renderer), Some(render_textures)) = (
            self.wgpu_ctx.as_ref(),
            self.renderer.as_mut(),
            self.render_textures.as_mut(),
        ) else {
            return;
        };

        self.render_scene
            .update_from_core_store(&self.store, DEFAULT_WIDTH, DEFAULT_HEIGHT);
        renderer.render_scene(ctx, render_textures, self.clear_color, &self.render_scene);
        self.render_dirty = false;
    }

    fn viewport_size(ui: &egui::Ui) -> Vec2 {
        let available = ui.available_size_before_wrap();
        let aspect = DEFAULT_WIDTH as f32 / DEFAULT_HEIGHT as f32;
        let width = available.x.max(1.0);
        let height = (width / aspect).min(available.y.max(1.0));

        if height * aspect <= available.x {
            Vec2::new(height * aspect, height)
        } else {
            Vec2::new(width, width / aspect)
        }
    }
}

impl eframe::App for RanimLabApp {
    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        self.prepare_renderer(frame);
        self.render_if_needed();

        egui::Panel::top("lab_toolbar").show_inside(ui, |ui| {
            ui.horizontal(|ui| {
                ui.heading("Ranim Lab");
                if ui.button("Render").clicked() {
                    self.render_dirty = true;
                }
            });
        });

        egui::CentralPanel::default().show_inside(ui, |ui| {
            if let Some(texture_id) = self.texture_id {
                let size = Self::viewport_size(ui);
                ui.image((texture_id, size));
            } else {
                ui.centered_and_justified(|ui| {
                    ui.label("Waiting for WGPU renderer");
                });
            }
        });
    }
}

fn demo_store() -> CoreItemStore {
    let mut triangle = VItem::from_vpoints(vec![
        dvec3(-1.6, -0.9, 0.0),
        dvec3(0.0, 1.2, 0.0),
        dvec3(1.6, -0.9, 0.0),
    ]);
    triangle.close();
    triangle.fill_rgbas = vec![Rgba(vec4(0.08, 0.35, 0.95, 0.9))].into();
    triangle.stroke_rgbas = vec![Rgba(vec4(0.95, 0.95, 0.95, 1.0))].into();
    triangle.stroke_widths = vec![Width(0.04)].into();

    let mut store = CoreItemStore::new();
    store.camera_frames.push(CameraFrame::default());
    store.vitems.push(triangle);
    store
}
