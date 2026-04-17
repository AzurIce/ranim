use crate::{
    color::{self, AlphaColor, Srgb},
    core::{
        components::rgba::Rgba,
        core_item::{
            camera_frame::CameraFrame,
            mesh_item::MeshItem as CoreMeshItem,
            vitem::{Basis2d, VItem as CoreVItem},
            CoreItem,
        },
        store::CoreItemStore,
        Extract,
    },
    glam::{DVec3, Mat4, Vec3, Vec4},
    render::{
        resource::{RenderPool, RenderTextures},
        utils::WgpuContext,
        Renderer,
    },
};
use eframe::{egui, App};
use egui::color_picker::{color_edit_button_srgba, Alpha as ColorAlpha};

#[cfg(feature = "items")]
use crate::core::traits::{FillColor, StrokeColor};
#[cfg(feature = "items")]
use crate::items::vitem::geometry::{Circle, RegularPolygon, Square};

fn linear_to_srgb(x: f32) -> f32 {
    if x <= 0.0031308 {
        12.92 * x
    } else {
        (1.055 * x.powf(1.0 / 2.4) - 0.055).clamp(0.0, 1.0)
    }
}

fn srgb_to_linear(x: f32) -> f32 {
    if x <= 0.04045 {
        x / 12.92
    } else {
        ((x + 0.055) / 1.055).powf(2.4)
    }
}

fn rgba_to_color32(rgba: &Rgba) -> egui::Color32 {
    let [r, g, b, a] = rgba.0.to_array();
    egui::Color32::from_rgba_unmultiplied(
        (linear_to_srgb(r) * 255.0) as u8,
        (linear_to_srgb(g) * 255.0) as u8,
        (linear_to_srgb(b) * 255.0) as u8,
        (a.clamp(0.0, 1.0) * 255.0) as u8,
    )
}

fn color32_to_rgba(color: egui::Color32) -> Rgba {
    let r = color.r() as f32 / 255.0;
    let g = color.g() as f32 / 255.0;
    let b = color.b() as f32 / 255.0;
    let a = color.a() as f32 / 255.0;
    Rgba(Vec4::new(
        srgb_to_linear(r),
        srgb_to_linear(g),
        srgb_to_linear(b),
        a,
    ))
}

struct EditorItem {
    name: String,
    visible: bool,
    core_item: CoreItem,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum SelectionTarget {
    Camera,
    Item(usize),
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum AddPreset {
    Circle,
    Square,
    Triangle,
    RegularPolygon,
    MeshTriangle,
}

impl std::fmt::Display for AddPreset {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AddPreset::Circle => write!(f, "Circle"),
            AddPreset::Square => write!(f, "Square"),
            AddPreset::Triangle => write!(f, "Triangle"),
            AddPreset::RegularPolygon => write!(f, "Regular Polygon"),
            AddPreset::MeshTriangle => write!(f, "Mesh Triangle"),
        }
    }
}

enum DragState {
    Orbit { last_pos: egui::Pos2 },
    Pan { last_pos: egui::Pos2 },
}

pub struct EditorApp {
    items: Vec<EditorItem>,
    selection: Option<SelectionTarget>,
    item_counter: usize,

    orbit_phi: f64,
    orbit_theta: f64,
    orbit_distance: f64,
    orbit_target: DVec3,
    perspective_blend: f64,
    frame_height: f64,

    renderer: Option<Renderer>,
    render_textures: Option<RenderTextures>,
    texture_id: Option<egui::TextureId>,
    wgpu_ctx: Option<WgpuContext>,
    pool: RenderPool,
    store: CoreItemStore,
    need_rerender: bool,
    clear_color: wgpu::Color,
    resolution: (u32, u32),

    dragging: Option<DragState>,
}

impl EditorApp {
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            selection: Some(SelectionTarget::Camera),
            item_counter: 0,

            orbit_phi: std::f64::consts::FRAC_PI_4,
            orbit_theta: -std::f64::consts::FRAC_PI_4,
            orbit_distance: 12.0,
            orbit_target: DVec3::ZERO,
            perspective_blend: 0.0,
            frame_height: 8.0,

            renderer: None,
            render_textures: None,
            texture_id: None,
            wgpu_ctx: None,
            pool: RenderPool::new(),
            store: CoreItemStore::default(),
            need_rerender: true,
            clear_color: wgpu::Color {
                r: 0.11,
                g: 0.11,
                b: 0.11,
                a: 1.0,
            },
            resolution: (1280, 720),

            dragging: None,
        }
    }

    fn camera(&self) -> CameraFrame {
        let mut cam =
            CameraFrame::from_spherical(self.orbit_phi, self.orbit_theta, self.orbit_distance);
        cam.set_spherical(
            self.orbit_phi,
            self.orbit_theta,
            self.orbit_distance,
            self.orbit_target,
        );
        cam.perspective_blend = self.perspective_blend;
        cam.frame_height = self.frame_height;
        cam
    }

    fn rebuild_store(&mut self) {
        let camera = self.camera();
        let mut items: Vec<((usize, usize), CoreItem)> = Vec::new();
        items.push(((0, 0), CoreItem::CameraFrame(camera)));

        for (i, item) in self.items.iter().enumerate() {
            if item.visible {
                items.push(((i + 1, 0), item.core_item.clone()));
            }
        }

        self.store.update(items.into_iter());
        self.need_rerender = true;
    }

    fn add_preset(&mut self, preset: AddPreset) {
        self.item_counter += 1;
        let name = format!("{} {}", preset, self.item_counter);
        let core_item = match preset {
            #[cfg(feature = "items")]
            AddPreset::Circle => {
                let mut item = Circle::new(1.0);
                FillColor::set_fill_color(
                    &mut item,
                    color::palettes::manim::BLUE_C.with_alpha(0.8),
                );
                StrokeColor::set_stroke_color(&mut item, AlphaColor::<Srgb>::WHITE);
                let items = Extract::extract(&item);
                items
                    .into_iter()
                    .next()
                    .unwrap_or(CoreItem::VItem(CoreVItem::default()))
            }
            #[cfg(feature = "items")]
            AddPreset::Square => {
                let mut item = Square::new(2.0);
                FillColor::set_fill_color(
                    &mut item,
                    color::palettes::manim::GREEN_C.with_alpha(0.8),
                );
                StrokeColor::set_stroke_color(&mut item, AlphaColor::<Srgb>::WHITE);
                let items = Extract::extract(&item);
                items
                    .into_iter()
                    .next()
                    .unwrap_or(CoreItem::VItem(CoreVItem::default()))
            }
            #[cfg(feature = "items")]
            AddPreset::Triangle => {
                let mut item = RegularPolygon::new(3, 1.0);
                FillColor::set_fill_color(&mut item, color::palettes::manim::RED_C.with_alpha(0.8));
                StrokeColor::set_stroke_color(&mut item, AlphaColor::<Srgb>::WHITE);
                let items = Extract::extract(&item);
                items
                    .into_iter()
                    .next()
                    .unwrap_or(CoreItem::VItem(CoreVItem::default()))
            }
            #[cfg(feature = "items")]
            AddPreset::RegularPolygon => {
                let mut item = RegularPolygon::new(6, 1.0);
                FillColor::set_fill_color(
                    &mut item,
                    color::palettes::manim::YELLOW_C.with_alpha(0.8),
                );
                StrokeColor::set_stroke_color(&mut item, AlphaColor::<Srgb>::WHITE);
                let items = Extract::extract(&item);
                items
                    .into_iter()
                    .next()
                    .unwrap_or(CoreItem::VItem(CoreVItem::default()))
            }
            AddPreset::MeshTriangle => CoreItem::MeshItem(CoreMeshItem {
                points: vec![
                    Vec3::new(-1.0, -1.0, 0.0),
                    Vec3::new(1.0, -1.0, 0.0),
                    Vec3::new(0.0, 1.0, 0.0),
                ],
                triangle_indices: vec![0, 1, 2],
                transform: Mat4::IDENTITY,
                vertex_colors: vec![
                    Rgba(Vec4::new(0.35, 0.77, 0.87, 1.0)),
                    Rgba(Vec4::new(0.35, 0.77, 0.87, 1.0)),
                    Rgba(Vec4::new(0.35, 0.77, 0.87, 1.0)),
                ],
                vertex_normals: vec![Vec3::Z; 3],
            }),
            #[cfg(not(feature = "items"))]
            _ => CoreItem::VItem(CoreVItem::default()),
        };

        let item = EditorItem {
            name,
            visible: true,
            core_item,
        };
        self.items.push(item);
        self.selection = Some(SelectionTarget::Item(self.items.len() - 1));
        self.rebuild_store();
    }

    fn remove_item(&mut self, index: usize) {
        if index < self.items.len() {
            self.items.remove(index);
            self.selection = None;
            self.rebuild_store();
        }
    }

    fn prepare_renderer(&mut self, frame: &eframe::Frame) {
        let needs_init = self.renderer.is_none();
        if !needs_init {
            return;
        }

        let Some(render_state) = frame.wgpu_render_state() else {
            return;
        };

        let ctx = WgpuContext {
            instance: wgpu::Instance::default(),
            adapter: wgpu::Adapter::clone(&render_state.adapter),
            device: wgpu::Device::clone(&render_state.device),
            queue: wgpu::Queue::clone(&render_state.queue),
        };

        let (width, height) = self.resolution;
        let oit_layers = {
            let limits = ctx.device.limits();
            let max_buffer_size = limits.max_storage_buffer_binding_size as usize;
            let pixel_count = (width * height) as usize;
            let max_layers = max_buffer_size / (pixel_count * 8);
            max_layers.clamp(1, 8)
        };

        let renderer = Renderer::new(&ctx, width, height, oit_layers);
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
        self.need_rerender = true;
    }

    fn render_scene(&mut self) {
        if !self.need_rerender {
            return;
        }
        self.need_rerender = false;

        if self.wgpu_ctx.is_none() || self.renderer.is_none() || self.render_textures.is_none() {
            return;
        }

        let camera = self.camera();
        let mut items: Vec<((usize, usize), CoreItem)> = Vec::new();
        items.push(((0, 0), CoreItem::CameraFrame(camera)));
        for (i, item) in self.items.iter().enumerate() {
            if item.visible {
                items.push(((i + 1, 0), item.core_item.clone()));
            }
        }
        self.store.update(items.into_iter());

        // SAFETY: We checked all three are Some above
        let ctx = unsafe { self.wgpu_ctx.as_ref().unwrap_unchecked() };
        let renderer = self.renderer.as_mut().unwrap();
        let render_textures = self.render_textures.as_mut().unwrap();

        renderer.render_store_with_pool(
            ctx,
            render_textures,
            self.clear_color,
            &self.store,
            &mut self.pool,
        );
        self.pool.clean();
    }

    fn handle_viewport_input(&mut self, ui: &mut egui::Ui) {
        let rect = ui.available_rect_before_wrap();
        let response = ui.interact(
            rect,
            ui.id().with("viewport"),
            egui::Sense::click_and_drag(),
        );

        if response.hovered() {
            let scroll_amt = ui.ctx().input(|i| i.smooth_scroll_delta.y as f64);
            if scroll_amt != 0.0 {
                self.frame_height =
                    (self.frame_height + scroll_amt * -0.003 * self.frame_height).clamp(0.5, 100.0);
                self.need_rerender = true;
            }
        }

        match self.dragging {
            Some(DragState::Orbit { last_pos }) => {
                if ui.ctx().input(|i| i.pointer.secondary_down()) {
                    if let Some(current) = response.hover_pos() {
                        let dx = (current.x - last_pos.x) as f64 * 0.01;
                        let dy = (current.y - last_pos.y) as f64 * 0.01;
                        self.orbit_theta += dx;
                        self.orbit_phi =
                            (self.orbit_phi + dy).clamp(0.01, std::f64::consts::PI - 0.01);
                        self.dragging = Some(DragState::Orbit { last_pos: current });
                        self.need_rerender = true;
                    }
                } else {
                    self.dragging = None;
                }
            }
            Some(DragState::Pan { last_pos }) => {
                if let Some(current) = response.hover_pos() {
                    let secondary = ui.ctx().input(|i| i.pointer.secondary_down());
                    let middle = ui.ctx().input(|i| i.pointer.middle_down());
                    if middle || (secondary && ui.ctx().input(|i| i.modifiers.shift)) {
                        let dx = (current.x - last_pos.x) as f64;
                        let dy = (current.y - last_pos.y) as f64;

                        let cam = self.camera();
                        let right = cam.facing.cross(cam.up).normalize();
                        let up = cam.up.normalize();
                        let scale = self.frame_height * 0.003;

                        self.orbit_target -= right * dx * scale;
                        self.orbit_target += up * dy * scale;

                        self.dragging = Some(DragState::Pan { last_pos: current });
                        self.need_rerender = true;
                    } else {
                        self.dragging = None;
                    }
                } else {
                    self.dragging = None;
                }
            }
            None => {}
        }

        if response.hovered() {
            let secondary_pressed = ui.ctx().input(|i| i.pointer.secondary_pressed());
            let middle_pressed = ui.ctx().input(|i| i.pointer.middle_down());
            if secondary_pressed {
                if let Some(pos) = response.hover_pos() {
                    self.dragging = Some(DragState::Orbit { last_pos: pos });
                }
            } else if middle_pressed {
                if let Some(pos) = response.hover_pos() {
                    self.dragging = Some(DragState::Pan { last_pos: pos });
                }
            }
        }
    }

    fn ui_left_panel(&mut self, ui: &mut egui::Ui) {
        ui.vertical(|ui| {
            ui.heading("Scene");
            ui.separator();

            let is_cam_selected = self.selection == Some(SelectionTarget::Camera);
            if ui.selectable_label(is_cam_selected, "📷 Camera").clicked() {
                self.selection = Some(SelectionTarget::Camera);
            }

            ui.separator();

            ui.horizontal(|ui| {
                ui.menu_button("+ Add Item", |ui| {
                    let presets = [
                        AddPreset::Circle,
                        AddPreset::Square,
                        AddPreset::Triangle,
                        AddPreset::RegularPolygon,
                        AddPreset::MeshTriangle,
                    ];
                    for preset in presets {
                        if ui.button(preset.to_string()).clicked() {
                            self.add_preset(preset);
                            ui.close();
                        }
                    }
                });
            });

            ui.separator();

            egui::ScrollArea::vertical().show(ui, |ui| {
                let mut to_remove: Option<usize> = None;
                for (i, item) in self.items.iter_mut().enumerate() {
                    let is_selected = self.selection == Some(SelectionTarget::Item(i));
                    let label = item.name.as_str();

                    ui.horizontal(|ui| {
                        ui.checkbox(&mut item.visible, "");
                        if ui.selectable_label(is_selected, label).clicked() {
                            self.selection = Some(SelectionTarget::Item(i));
                        }
                        if ui.small_button("✕").clicked() {
                            to_remove = Some(i);
                        }
                    });
                }
                if let Some(idx) = to_remove {
                    self.remove_item(idx);
                }
            });
        });
    }

    fn ui_properties_panel(&mut self, ui: &mut egui::Ui) {
        ui.vertical(|ui| {
            ui.heading("Properties");
            ui.separator();

            match self.selection {
                Some(SelectionTarget::Camera) => self.ui_camera_properties(ui),
                Some(SelectionTarget::Item(idx)) if idx < self.items.len() => {
                    let core_item = &mut self.items[idx].core_item;
                    match core_item {
                        CoreItem::CameraFrame(_) => {
                            ui.label("CameraFrame (in scene)");
                        }
                        CoreItem::VItem(vitem) => {
                            let changed = Self::ui_vitem_properties(ui, vitem);
                            if changed {
                                self.need_rerender = true;
                            }
                        }
                        CoreItem::MeshItem(mesh) => {
                            let changed = Self::ui_mesh_item_properties(ui, mesh);
                            if changed {
                                self.need_rerender = true;
                            }
                        }
                    }
                }
                _ => {
                    ui.label("Nothing selected");
                }
            }
        });
    }

    fn ui_camera_properties(&mut self, ui: &mut egui::Ui) {
        ui.heading("Camera");
        ui.separator();

        ui.group(|ui| {
            ui.label("Orbit");
            let mut changed = false;
            ui.horizontal(|ui| {
                ui.label("Phi:");
                changed |= ui
                    .add(egui::DragValue::new(&mut self.orbit_phi).speed(0.01))
                    .changed();
            });
            ui.horizontal(|ui| {
                ui.label("Theta:");
                changed |= ui
                    .add(egui::DragValue::new(&mut self.orbit_theta).speed(0.01))
                    .changed();
            });
            ui.horizontal(|ui| {
                ui.label("Distance:");
                changed |= ui
                    .add(
                        egui::DragValue::new(&mut self.orbit_distance)
                            .speed(0.1)
                            .range(0.5..=200.0),
                    )
                    .changed();
            });
            if changed {
                self.need_rerender = true;
            }
        });

        ui.add_space(4.0);

        ui.group(|ui| {
            ui.label("Target");
            let mut changed = false;
            ui.horizontal(|ui| {
                ui.label("X:");
                changed |= ui
                    .add(egui::DragValue::new(&mut self.orbit_target.x).speed(0.1))
                    .changed();
            });
            ui.horizontal(|ui| {
                ui.label("Y:");
                changed |= ui
                    .add(egui::DragValue::new(&mut self.orbit_target.y).speed(0.1))
                    .changed();
            });
            ui.horizontal(|ui| {
                ui.label("Z:");
                changed |= ui
                    .add(egui::DragValue::new(&mut self.orbit_target.z).speed(0.1))
                    .changed();
            });
            if changed {
                self.need_rerender = true;
            }
        });

        ui.add_space(4.0);

        ui.group(|ui| {
            ui.label("Projection");
            let mut changed = false;
            ui.horizontal(|ui| {
                ui.label("Persp. Blend:");
                changed |= ui
                    .add(
                        egui::DragValue::new(&mut self.perspective_blend)
                            .speed(0.01)
                            .range(0.0..=1.0),
                    )
                    .changed();
            });
            ui.horizontal(|ui| {
                ui.label("Frame Height:");
                changed |= ui
                    .add(
                        egui::DragValue::new(&mut self.frame_height)
                            .speed(0.1)
                            .range(0.5..=100.0),
                    )
                    .changed();
            });
            if changed {
                self.need_rerender = true;
            }
        });

        ui.add_space(4.0);

        ui.group(|ui| {
            ui.label("Computed (read-only)");
            let cam = self.camera();
            ui.horizontal(|ui| {
                ui.label("Pos:");
                ui.label(format!(
                    "({:.2}, {:.2}, {:.2})",
                    cam.pos.x, cam.pos.y, cam.pos.z
                ));
            });
            ui.horizontal(|ui| {
                ui.label("Facing:");
                ui.label(format!(
                    "({:.2}, {:.2}, {:.2})",
                    cam.facing.x, cam.facing.y, cam.facing.z
                ));
            });
        });
    }

    fn ui_vitem_properties(ui: &mut egui::Ui, vitem: &mut CoreVItem) -> bool {
        let mut any_changed = false;
        ui.heading("VItem");
        ui.separator();

        ui.group(|ui| {
            ui.label("Origin");
            ui.horizontal(|ui| {
                ui.label("X:");
                any_changed |= ui
                    .add(egui::DragValue::new(&mut vitem.origin.x).speed(0.1))
                    .changed();
            });
            ui.horizontal(|ui| {
                ui.label("Y:");
                any_changed |= ui
                    .add(egui::DragValue::new(&mut vitem.origin.y).speed(0.1))
                    .changed();
            });
            ui.horizontal(|ui| {
                ui.label("Z:");
                any_changed |= ui
                    .add(egui::DragValue::new(&mut vitem.origin.z).speed(0.1))
                    .changed();
            });
        });

        ui.add_space(4.0);

        ui.group(|ui| {
            ui.label("Basis");
            let u = vitem.basis.u();
            let v = vitem.basis.v();
            ui.horizontal(|ui| {
                ui.label("U:");
                ui.label(format!("({:.2}, {:.2}, {:.2})", u.x, u.y, u.z));
            });
            ui.horizontal(|ui| {
                ui.label("V:");
                ui.label(format!("({:.2}, {:.2}, {:.2})", v.x, v.y, v.z));
            });
            ui.horizontal(|ui| {
                if ui.button("XY Plane").clicked() {
                    vitem.basis = Basis2d::XY;
                    any_changed = true;
                }
                if ui.button("XZ Plane").clicked() {
                    vitem.basis = Basis2d::XZ;
                    any_changed = true;
                }
                if ui.button("YZ Plane").clicked() {
                    vitem.basis = Basis2d::YZ;
                    any_changed = true;
                }
            });
        });

        ui.add_space(4.0);

        ui.group(|ui| {
            ui.label("Colors");

            if !vitem.fill_rgbas.is_empty() {
                let mut egui_color = rgba_to_color32(&vitem.fill_rgbas[0]);
                ui.horizontal(|ui| {
                    ui.label("Fill:");
                    if color_edit_button_srgba(ui, &mut egui_color, ColorAlpha::BlendOrAdditive)
                        .changed()
                    {
                        let new_rgba = color32_to_rgba(egui_color);
                        vitem.fill_rgbas.fill(new_rgba);
                        any_changed = true;
                    }
                });
            }

            if !vitem.stroke_rgbas.is_empty() {
                let mut egui_color = rgba_to_color32(&vitem.stroke_rgbas[0]);
                ui.horizontal(|ui| {
                    ui.label("Stroke:");
                    if color_edit_button_srgba(ui, &mut egui_color, ColorAlpha::BlendOrAdditive)
                        .changed()
                    {
                        let new_rgba = color32_to_rgba(egui_color);
                        vitem.stroke_rgbas.fill(new_rgba);
                        any_changed = true;
                    }
                });
            }

            if !vitem.stroke_widths.is_empty() {
                let mut width = vitem.stroke_widths[0].0;
                ui.horizontal(|ui| {
                    ui.label("Stroke Width:");
                    if ui
                        .add(
                            egui::DragValue::new(&mut width)
                                .speed(0.01)
                                .range(0.0..=1.0),
                        )
                        .changed()
                    {
                        for w in &mut vitem.stroke_widths {
                            w.0 = width;
                        }
                        any_changed = true;
                    }
                });
            }
        });

        ui.add_space(4.0);

        ui.group(|ui| {
            ui.label(format!("Points: {}", vitem.points.len()));
        });

        any_changed
    }

    fn ui_mesh_item_properties(ui: &mut egui::Ui, mesh: &mut CoreMeshItem) -> bool {
        let mut any_changed = false;
        ui.heading("MeshItem");
        ui.separator();

        ui.group(|ui| {
            ui.label("Transform (position)");
            let mut pos = mesh.transform.w_axis.truncate();
            let mut changed = false;
            ui.horizontal(|ui| {
                ui.label("X:");
                changed |= ui
                    .add(egui::DragValue::new(&mut pos.x).speed(0.1))
                    .changed();
            });
            ui.horizontal(|ui| {
                ui.label("Y:");
                changed |= ui
                    .add(egui::DragValue::new(&mut pos.y).speed(0.1))
                    .changed();
            });
            ui.horizontal(|ui| {
                ui.label("Z:");
                changed |= ui
                    .add(egui::DragValue::new(&mut pos.z).speed(0.1))
                    .changed();
            });
            if changed {
                mesh.transform.w_axis = Vec4::new(pos.x, pos.y, pos.z, 1.0);
                any_changed = true;
            }
        });

        ui.add_space(4.0);

        ui.group(|ui| {
            ui.label("Vertex Colors");
            if !mesh.vertex_colors.is_empty() {
                let mut egui_color = rgba_to_color32(&mesh.vertex_colors[0]);
                ui.horizontal(|ui| {
                    ui.label("Color:");
                    if color_edit_button_srgba(ui, &mut egui_color, ColorAlpha::BlendOrAdditive)
                        .changed()
                    {
                        let new_rgba = color32_to_rgba(egui_color);
                        mesh.vertex_colors.fill(new_rgba);
                        any_changed = true;
                    }
                });
            }
        });

        ui.add_space(4.0);

        ui.group(|ui| {
            ui.label(format!("Vertices: {}", mesh.points.len()));
            ui.label(format!("Triangles: {}", mesh.triangle_indices.len() / 3));
        });

        any_changed
    }

    fn ui_viewport(&mut self, ui: &mut egui::Ui) {
        self.handle_viewport_input(ui);

        let texture_id = self.texture_id;
        if let Some(tid) = texture_id {
            let available_size = ui.available_size();
            let aspect_ratio = self.resolution.0 as f32 / self.resolution.1 as f32;
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
    }
}

impl App for EditorApp {
    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        self.prepare_renderer(frame);
        self.render_scene();

        egui::Panel::left("scene_panel")
            .default_size(200.0)
            .show_inside(ui, |ui| {
                self.ui_left_panel(ui);
            });

        egui::Panel::right("properties_panel")
            .default_size(250.0)
            .show_inside(ui, |ui| {
                self.ui_properties_panel(ui);
            });

        self.ui_viewport(ui);

        if self.need_rerender {
            ui.ctx().request_repaint();
        }
    }
}

pub fn run_editor() {
    let title = "Ranim Editor".to_string();

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title(&title)
            .with_inner_size([1440.0, 900.0]),
        renderer: eframe::Renderer::Wgpu,
        ..Default::default()
    };

    let app = EditorApp::new();
    eframe::run_native(&title, native_options, Box::new(|_cc| Ok(Box::new(app)))).unwrap();
}
