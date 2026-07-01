use std::{collections::BTreeMap, time::Instant};

use egui::{
    Align2, CentralPanel, Color32, Context, CornerRadius, CursorIcon, DragValue, FontId, Id, Key,
    LayerId, Margin, Order, Panel, PointerButton, Pos2, Rect, Sense, Stroke, StrokeKind, TextureId,
    Ui, Vec2, pos2, vec2,
};
use ranim_core::{
    core_item::CoreItem,
    glam::{DVec3, dvec3},
    prelude::CameraFrame,
    store::CoreItemStore,
};
use ranim_render::{
    Renderer,
    resource::{RenderPool, RenderTextures},
    utils::WgpuContext,
};
use tracing::{debug, info, trace};

use crate::{
    model::{Deck, Element, MIN_OBJECT_SIZE, SlideFrame},
    object::{
        InspectorCtx, PaintCtx, RECTANGLE_DESCRIPTOR, RenderCtx, SlideObjectDescriptor,
        SlideObjectRegistry, TEXT_DESCRIPTOR,
    },
};

const LEFT_PANEL_WIDTH: f32 = 188.0;
const RIGHT_PANEL_WIDTH: f32 = 280.0;
const TOOLBAR_HEIGHT: f32 = 44.0;
const BOTTOM_BAR_HEIGHT: f32 = 36.0;
const CANVAS_PADDING: f32 = 48.0;
const MIN_CANVAS_ZOOM: f32 = 0.1;
const MAX_CANVAS_ZOOM: f32 = 8.0;
const TOOLBAR_ZOOM_STEP: f32 = 1.2;
const RENDER_WIDTH: u32 = 1280;
const RENDER_HEIGHT: u32 = 720;
const RENDER_OIT_LAYERS: usize = 8;
const MIN_RENDER_SIZE: u32 = 16;
const WORLD_RENDER_BUFFER_SAFETY: f64 = 0.90;
const WORLD_CAMERA_ROTATE_SPEED: f64 = 0.006;
const WORLD_CAMERA_MOVE_SPEED: f64 = 4.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum CenterTab {
    World,
    Output,
}

impl CenterTab {
    fn allows_selection(self) -> bool {
        matches!(self, Self::World)
    }
}

#[derive(Debug, Clone, Copy)]
enum ObjectTreeAction {
    Duplicate(u64),
    Delete(u64),
}

pub struct RanimSlideApp {
    deck: Deck,
    object_registry: SlideObjectRegistry,
    store: CoreItemStore,
    renderer: Option<Renderer>,
    render_textures: Option<RenderTextures>,
    render_pool: RenderPool,
    wgpu_ctx: Option<WgpuContext>,
    texture_id: Option<TextureId>,
    render_dirty: bool,
    world_camera: CameraFrame,
    canvas_zoom: f32,
    canvas_pan: Vec2,
    center_tab: CenterTab,
    last_canvas_available: Option<Vec2>,
    last_canvas_rect: Option<Rect>,
    last_viewport_inner_rect: Option<Rect>,
    last_content_rect: Option<Rect>,
}

impl RanimSlideApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        info!("creating ranim-slide app");
        let mut fonts = egui::FontDefinitions::default();
        egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);
        cc.egui_ctx.set_fonts(fonts);

        let object_registry = SlideObjectRegistry::builtin();
        let mut deck = Deck::default();
        deck.add_object_to_current_page(&RECTANGLE_DESCRIPTOR);
        deck.add_object_to_current_page(&TEXT_DESCRIPTOR);
        let frame = deck.frame;

        Self {
            deck,
            object_registry,
            store: CoreItemStore::new(),
            renderer: None,
            render_textures: None,
            render_pool: RenderPool::new(),
            wgpu_ctx: None,
            texture_id: None,
            render_dirty: true,
            world_camera: default_world_camera(frame),
            canvas_zoom: 1.0,
            canvas_pan: Vec2::ZERO,
            center_tab: CenterTab::World,
            last_canvas_available: None,
            last_canvas_rect: None,
            last_viewport_inner_rect: None,
            last_content_rect: None,
        }
    }

    fn ensure_renderer_size(&mut self, frame: &eframe::Frame, width: u32, height: u32) -> bool {
        let Some(render_state) = frame.wgpu_render_state() else {
            trace!("wgpu render state not ready");
            return false;
        };

        if self.wgpu_ctx.is_none() {
            self.wgpu_ctx = Some(WgpuContext {
                instance: wgpu::Instance::default(),
                adapter: wgpu::Adapter::clone(&render_state.adapter),
                device: wgpu::Device::clone(&render_state.device),
                queue: wgpu::Queue::clone(&render_state.queue),
            });
        }

        let width = width.max(MIN_RENDER_SIZE);
        let height = height.max(MIN_RENDER_SIZE);
        let needs_rebuild = self
            .renderer
            .as_ref()
            .is_none_or(|renderer| renderer.width() != width || renderer.height() != height);
        if !needs_rebuild {
            return true;
        }

        let start = Instant::now();
        let ctx = self.wgpu_ctx.as_ref().expect("wgpu context initialized");
        let renderer = Renderer::new(ctx, width, height, RENDER_OIT_LAYERS);
        let render_textures = renderer.new_render_textures(ctx);
        if let Some(texture_id) = self.texture_id {
            render_state
                .renderer
                .write()
                .update_egui_texture_from_wgpu_texture(
                    &render_state.device,
                    &render_textures.linear_render_view,
                    wgpu::FilterMode::Linear,
                    texture_id,
                );
        } else {
            let texture_id = render_state.renderer.write().register_native_texture(
                &render_state.device,
                &render_textures.linear_render_view,
                wgpu::FilterMode::Linear,
            );
            self.texture_id = Some(texture_id);
        }

        self.renderer = Some(renderer);
        self.render_textures = Some(render_textures);
        self.render_dirty = true;
        info!(
            width,
            height,
            elapsed_ms = start.elapsed().as_secs_f64() * 1000.0,
            "prepared ranim renderer target"
        );
        true
    }

    fn rebuild_store(&mut self, camera: CameraFrame) {
        let start = Instant::now();
        let mut items = Vec::new();
        items.push(((0, 0), CoreItem::CameraFrame(camera)));

        let render_ctx = RenderCtx;
        let mut ordered_elements = self
            .deck
            .current_page()
            .elements
            .iter()
            .enumerate()
            .filter(|(_, element)| element.visible)
            .collect::<Vec<_>>();
        ordered_elements.sort_by_key(|(_, element)| (element.z_index, element.id));
        let visible_elements = ordered_elements.len();

        for (idx, element) in ordered_elements {
            let start_len = items.len();
            let mut core_items = Vec::new();
            element
                .object
                .extract_core_items(&render_ctx, &mut core_items);
            items.extend(
                core_items
                    .into_iter()
                    .enumerate()
                    .map(|(item_idx, item)| ((idx + 1, item_idx), item)),
            );

            if items.len() == start_len {
                continue;
            }
        }

        let item_count = items.len();
        self.store.update(items.into_iter());
        debug!(
            elements = self.deck.current_page().elements.len(),
            visible_elements,
            core_items = item_count,
            vitems = self.store.vitems.len(),
            meshes = self.store.mesh_items.len(),
            elapsed_ms = start.elapsed().as_secs_f64() * 1000.0,
            "rebuilt core item store"
        );
    }

    fn render_if_needed(&mut self, camera: CameraFrame) {
        if !self.render_dirty {
            return;
        }

        self.rebuild_store(camera);

        let (Some(ctx), Some(renderer), Some(render_textures)) = (
            self.wgpu_ctx.as_ref(),
            self.renderer.as_mut(),
            self.render_textures.as_mut(),
        ) else {
            return;
        };

        let start = Instant::now();
        renderer.render_store_with_pool(
            ctx,
            render_textures,
            wgpu::Color {
                r: 0.98,
                g: 0.985,
                b: 0.995,
                a: 1.0,
            },
            &self.store,
            &mut self.render_pool,
        );
        self.render_dirty = false;
        info!(
            vitems = self.store.vitems.len(),
            meshes = self.store.mesh_items.len(),
            elapsed_ms = start.elapsed().as_secs_f64() * 1000.0,
            "rendered slide preview"
        );
    }

    fn ui_toolbar(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.heading("Ranim Slide");
            ui.separator();

            if ui.button("Add Page").clicked() {
                self.deck.push_page();
                self.render_dirty = true;
            }

            let mut pending_insert = None;
            ui.menu_button("Insert", |ui| {
                descriptor_menu_ui(
                    ui,
                    self.object_registry.descriptors(),
                    0,
                    &mut pending_insert,
                );
            });
            if let Some(descriptor) = pending_insert {
                self.deck.add_object_to_current_page(descriptor);
                self.render_dirty = true;
            }

            let can_delete_element = self.deck.current_page().selected_element.is_some();
            if ui
                .add_enabled(can_delete_element, egui::Button::new("Delete"))
                .clicked()
            {
                self.deck.current_page_mut().delete_selected();
                self.render_dirty = true;
            }
        });
    }

    fn ui_bottom_bar(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            if ui.button("-").clicked() {
                self.canvas_zoom = clamp_canvas_zoom(self.canvas_zoom / TOOLBAR_ZOOM_STEP);
            }
            if ui.button("Fit").clicked() {
                self.reset_canvas_view();
            }
            if ui.button("+").clicked() {
                self.canvas_zoom = clamp_canvas_zoom(self.canvas_zoom * TOOLBAR_ZOOM_STEP);
            }

            let mut zoom_percent = self.canvas_zoom * 100.0;
            if ui
                .add(
                    egui::Slider::new(
                        &mut zoom_percent,
                        MIN_CANVAS_ZOOM * 100.0..=MAX_CANVAS_ZOOM * 100.0,
                    )
                    .suffix("%"),
                )
                .changed()
            {
                self.canvas_zoom = clamp_canvas_zoom(zoom_percent / 100.0);
            }
            ui.label(format!("{:.0}%", self.canvas_zoom * 100.0));
        });
    }

    fn ui_pages(&mut self, ui: &mut Ui) {
        ui.heading("Pages");
        ui.add_space(8.0);

        for page_idx in 0..self.deck.pages.len() {
            let selected = self.deck.selected_page == page_idx;
            let page = &self.deck.pages[page_idx];
            let label = format!("{}  {}  #{}", page_idx + 1, page.name, page.id);

            let response = ui.selectable_label(selected, label);
            if response.clicked() {
                self.deck.selected_page = page_idx;
                self.render_dirty = true;
            }
        }

        ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
            if ui
                .add_enabled(self.deck.pages.len() > 1, egui::Button::new("Remove Page"))
                .clicked()
            {
                self.deck.remove_current_page();
                self.render_dirty = true;
            }
        });
    }

    fn ui_right_panel(&mut self, ui: &mut Ui) {
        self.ui_object_tree(ui);
        ui.add_space(10.0);
        ui.separator();
        ui.add_space(10.0);
        self.ui_inspector(ui);
    }

    fn ui_object_tree(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.heading("Objects");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(format!("{} items", self.deck.current_page().elements.len()));
            });
        });
        ui.add_space(6.0);

        let mut select_after = None;
        let mut post_action = None;
        let mut render_changed = false;

        egui::ScrollArea::vertical()
            .id_salt("object_tree")
            .max_height(190.0)
            .auto_shrink([false, true])
            .show(ui, |ui| {
                let default_output_camera = self.deck.frame.camera_frame();
                let page = self.deck.current_page_mut();
                let camera_response = ui
                    .horizontal(|ui| {
                        ui.label(egui_phosphor::regular::CAMERA);
                        if ui
                            .selectable_label(page.selected_camera, "Output Camera")
                            .on_hover_text("Camera used by the Output tab")
                            .clicked()
                        {
                            page.select_output_camera();
                        }
                    })
                    .response;
                camera_response.context_menu(|ui| {
                    if ui.button("Reset").clicked() {
                        page.output_camera = default_output_camera;
                        page.select_output_camera();
                        render_changed = true;
                        ui.close();
                    }
                });
                ui.separator();

                if page.elements.is_empty() {
                    ui.label("No objects");
                    return;
                }

                let mut order = (0..page.elements.len()).collect::<Vec<_>>();
                order.sort_by_key(|idx| {
                    let element = &page.elements[*idx];
                    (element.z_index, element.id)
                });
                order.reverse();

                for idx in order {
                    let element = &mut page.elements[idx];
                    let id = element.id;
                    let mut row_action = None;
                    let row_response = ui
                        .horizontal(|ui| {
                            let visibility_icon = if element.visible {
                                egui_phosphor::regular::EYE
                            } else {
                                egui_phosphor::regular::EYE_SLASH
                            };
                            let visibility_hint = if element.visible { "Hide" } else { "Show" };
                            if ui
                                .small_button(visibility_icon)
                                .on_hover_text(visibility_hint)
                                .clicked()
                            {
                                element.visible = !element.visible;
                                render_changed = true;
                            }

                            let lock_icon = if element.locked {
                                egui_phosphor::regular::LOCK
                            } else {
                                egui_phosphor::regular::LOCK_OPEN
                            };
                            let lock_hint = if element.locked { "Unlock" } else { "Lock" };
                            if ui
                                .small_button(lock_icon)
                                .on_hover_text(lock_hint)
                                .clicked()
                            {
                                element.locked = !element.locked;
                            }

                            let label = egui::RichText::new(element.name.as_str());
                            let label = if element.visible {
                                label
                            } else {
                                label.color(Color32::from_rgb(125, 132, 143))
                            };
                            let label_response = ui
                                .selectable_label(element.selected, label)
                                .on_hover_text(element.object.descriptor().type_id);
                            if label_response.clicked() {
                                select_after = Some(id);
                            }
                        })
                        .response;

                    if row_response.clicked() {
                        select_after = Some(id);
                    }

                    row_response.context_menu(|ui| {
                        ui.label("Rename");
                        ui.text_edit_singleline(&mut element.name);
                        ui.separator();
                        if ui.button("Copy").clicked() {
                            row_action = Some(ObjectTreeAction::Duplicate(id));
                            ui.close();
                        }
                        if ui.button("Delete").clicked() {
                            row_action = Some(ObjectTreeAction::Delete(id));
                            ui.close();
                        }
                    });

                    if row_action.is_some() {
                        post_action = row_action;
                    }
                }
            });

        if let Some(id) = select_after {
            self.deck.current_page_mut().select_element(Some(id));
        }

        match post_action {
            Some(ObjectTreeAction::Duplicate(id)) => {
                if self.deck.duplicate_element_on_current_page(id).is_some() {
                    self.render_dirty = true;
                }
            }
            Some(ObjectTreeAction::Delete(id)) => {
                if self.deck.current_page_mut().delete_element(id) {
                    self.render_dirty = true;
                }
            }
            None => {}
        }

        if render_changed {
            self.render_dirty = true;
        }
    }

    fn ui_center(&mut self, ui: &mut Ui, frame: &eframe::Frame) {
        ui.horizontal(|ui| {
            ui.add_space(8.0);
            let world_changed = ui
                .selectable_value(&mut self.center_tab, CenterTab::World, "World")
                .changed();
            let output_changed = ui
                .selectable_value(&mut self.center_tab, CenterTab::Output, "Output")
                .changed();
            if world_changed || output_changed {
                self.render_dirty = true;
            }
        });
        ui.separator();
        self.ui_canvas(ui, frame, self.center_tab);
    }

    fn ui_canvas(&mut self, ui: &mut Ui, frame: &eframe::Frame, tab: CenterTab) {
        let available_rect = ui.available_rect_before_wrap();
        if tab == CenterTab::World {
            self.ui_world_canvas(ui, frame, available_rect);
        } else {
            self.ui_output_canvas(ui, frame, available_rect);
        }
    }

    fn ui_world_canvas(&mut self, ui: &mut Ui, frame: &eframe::Frame, available_rect: Rect) {
        let (render_width, render_height) =
            world_render_size_for_rect(ui.ctx(), frame, available_rect);
        if self.handle_world_camera_navigation(ui, available_rect) {
            self.render_dirty = true;
        }
        self.ensure_renderer_size(frame, render_width, render_height);
        self.render_if_needed(self.world_camera.clone());

        self.log_canvas_resize(available_rect.size(), available_rect, 1.0);
        let canvas_response = ui.interact(
            available_rect,
            Id::new(("slide_canvas", CenterTab::World)),
            Sense::click_and_drag(),
        );
        if canvas_response.hovered() {
            ui.ctx().set_cursor_icon(CursorIcon::Crosshair);
        }

        paint_render_texture(
            ui,
            self.texture_id,
            available_rect,
            "Waiting for WGPU renderer",
        );
    }

    fn ui_output_canvas(&mut self, ui: &mut Ui, frame: &eframe::Frame, available_rect: Rect) {
        let available = available_rect.size();
        let slide_frame = self.deck.frame;
        let frame_size = slide_frame.size();
        let fit_scale = fit_canvas_scale(available, frame_size);
        let is_panning = self.handle_canvas_navigation(ui, available_rect, frame_size, fit_scale);
        let scale = fit_scale * self.canvas_zoom;
        let canvas_rect = canvas_rect_for_view(available_rect, frame_size, scale, self.canvas_pan);
        self.log_canvas_resize(available, canvas_rect, scale);
        self.ensure_renderer_size(frame, RENDER_WIDTH, RENDER_HEIGHT);
        self.render_if_needed(self.deck.current_page().output_camera.clone());

        let canvas_response = ui.interact(
            canvas_rect,
            Id::new(("slide_canvas", CenterTab::Output)),
            Sense::click_and_drag(),
        );

        let painter = ui.painter();
        painter.rect_filled(
            canvas_rect,
            CornerRadius::same(3),
            Color32::from_rgb(250, 251, 253),
        );
        painter.rect_stroke(
            canvas_rect,
            CornerRadius::same(3),
            Stroke::new(1.0_f32, Color32::from_rgb(180, 188, 200)),
            StrokeKind::Outside,
        );

        let pointer_delta = ui.input(|input| input.pointer.delta());
        let dragged_delta = if CenterTab::Output.allows_selection()
            && !is_panning
            && canvas_response.dragged_by(PointerButton::Primary)
        {
            let delta = pointer_delta / scale;
            vec2(delta.x, -delta.y)
        } else {
            Vec2::ZERO
        };

        if CenterTab::Output.allows_selection()
            && !is_panning
            && canvas_response.clicked_by(PointerButton::Primary)
        {
            let selected = canvas_response
                .interact_pointer_pos()
                .and_then(|pointer_pos| {
                    screen_to_scene(pointer_pos, canvas_rect, scale, slide_frame)
                })
                .and_then(|scene_pos| self.deck.current_page().element_at(scene_pos));
            self.deck.current_page_mut().select_element(selected);
        }

        if dragged_delta != Vec2::ZERO {
            if let Some(element) = self.deck.current_page_mut().selected_element_mut() {
                if element.translate(dragged_delta) {
                    self.render_dirty = true;
                    ui.ctx().set_cursor_icon(CursorIcon::Grabbing);
                }
            }
        }

        let paint_ctx = PaintCtx {
            canvas_rect,
            scale,
            frame_min: slide_frame.min(),
            frame_max: slide_frame.max(),
        };
        paint_render_texture(
            ui,
            self.texture_id,
            canvas_rect,
            "Waiting for WGPU renderer",
        );

        if CenterTab::Output.allows_selection() {
            for element in &self.deck.current_page().elements {
                paint_selection_overlay(ui, element, &paint_ctx);
            }
        }
    }

    fn handle_canvas_navigation(
        &mut self,
        ui: &mut Ui,
        available_rect: Rect,
        frame_size: Vec2,
        fit_scale: f32,
    ) -> bool {
        let (
            pointer_pos,
            press_origin,
            pointer_delta,
            zoom_delta,
            scroll_delta,
            primary_down,
            secondary_down,
            middle_down,
            space_down,
        ) = ui.input(|input| {
            (
                input.pointer.hover_pos(),
                input.pointer.press_origin(),
                input.pointer.delta(),
                input.zoom_delta(),
                input.smooth_scroll_delta(),
                input.pointer.button_down(PointerButton::Primary),
                input.pointer.button_down(PointerButton::Secondary),
                input.pointer.button_down(PointerButton::Middle),
                input.key_down(Key::Space),
            )
        });

        let pointer_over_view = pointer_pos.is_some_and(|pos| available_rect.contains(pos));
        let drag_started_in_view = press_origin.is_some_and(|pos| available_rect.contains(pos));
        let can_navigate = pointer_over_view || drag_started_in_view;
        if !can_navigate {
            return false;
        }

        let mut changed = false;
        if (zoom_delta - 1.0).abs() > 0.001 {
            let anchor = pointer_pos.unwrap_or_else(|| available_rect.center());
            changed |=
                self.zoom_canvas_at(anchor, available_rect, frame_size, fit_scale, zoom_delta);
        } else if scroll_delta != Vec2::ZERO && pointer_over_view {
            self.canvas_pan += scroll_delta;
            changed = true;
        }

        let is_pan_drag = drag_started_in_view
            && (middle_down || secondary_down || (space_down && primary_down))
            && pointer_delta != Vec2::ZERO;
        if is_pan_drag {
            self.canvas_pan += pointer_delta;
            ui.ctx().set_cursor_icon(CursorIcon::Grabbing);
            changed = true;
        }

        if changed {
            ui.ctx().request_repaint();
        } else if drag_started_in_view && (middle_down || secondary_down || space_down) {
            ui.ctx().set_cursor_icon(CursorIcon::Grab);
        }

        drag_started_in_view && (middle_down || secondary_down || (space_down && primary_down))
    }

    fn handle_world_camera_navigation(&mut self, ui: &mut Ui, available_rect: Rect) -> bool {
        let wants_keyboard = ui.ctx().egui_wants_keyboard_input();
        let (
            pointer_pos,
            press_origin,
            pointer_delta,
            secondary_down,
            w_down,
            a_down,
            s_down,
            d_down,
            q_down,
            e_down,
            stable_dt,
        ) = ui.input(|input| {
            (
                input.pointer.hover_pos(),
                input.pointer.press_origin(),
                input.pointer.delta(),
                input.pointer.button_down(PointerButton::Secondary),
                input.key_down(Key::W),
                input.key_down(Key::A),
                input.key_down(Key::S),
                input.key_down(Key::D),
                input.key_down(Key::Q),
                input.key_down(Key::E),
                input.stable_dt,
            )
        });

        let pointer_over_view = pointer_pos.is_some_and(|pos| available_rect.contains(pos));
        let drag_started_in_view = press_origin.is_some_and(|pos| available_rect.contains(pos));
        let mut changed = false;

        if secondary_down && drag_started_in_view {
            ui.ctx().set_cursor_icon(CursorIcon::Grabbing);
            if pointer_delta != Vec2::ZERO {
                rotate_world_camera(&mut self.world_camera, pointer_delta);
                changed = true;
            }
        } else if pointer_over_view {
            ui.ctx().set_cursor_icon(CursorIcon::Crosshair);
        }

        let can_move_camera =
            !wants_keyboard && secondary_down && (pointer_over_view || drag_started_in_view);
        if can_move_camera {
            let movement = world_camera_movement(
                &self.world_camera,
                w_down,
                a_down,
                s_down,
                d_down,
                q_down,
                e_down,
            );
            if movement != DVec3::ZERO {
                let dt = stable_dt.max(1.0 / 120.0) as f64;
                self.world_camera.pos += movement * WORLD_CAMERA_MOVE_SPEED * dt;
                changed = true;
            }
        }

        if changed {
            ui.ctx().request_repaint();
        }

        changed
    }

    fn zoom_canvas_at(
        &mut self,
        anchor: Pos2,
        available_rect: Rect,
        frame_size: Vec2,
        fit_scale: f32,
        zoom_delta: f32,
    ) -> bool {
        let Some((zoom, pan)) = zoomed_canvas_view_at(
            anchor,
            available_rect,
            frame_size,
            fit_scale,
            self.canvas_zoom,
            self.canvas_pan,
            zoom_delta,
        ) else {
            return false;
        };

        self.canvas_zoom = zoom;
        self.canvas_pan = pan;
        true
    }

    fn reset_canvas_view(&mut self) {
        self.canvas_zoom = 1.0;
        self.canvas_pan = Vec2::ZERO;
    }

    fn ui_inspector(&mut self, ui: &mut Ui) {
        ui.heading("Inspector");
        ui.add_space(8.0);

        let page = self.deck.current_page_mut();
        if page.selected_camera {
            ui.label("Name");
            ui.label("Output Camera");
            ui.label("Type: CameraFrame");
            ui.add_space(10.0);
            if camera_inspector_ui(ui, &mut page.output_camera) {
                self.render_dirty = true;
            }
            return;
        }

        let Some(element) = page.selected_element_mut() else {
            ui.label("No element selected");
            return;
        };

        ui.label("Name");
        ui.text_edit_singleline(&mut element.name);
        ui.label(format!(
            "Type: {}",
            element.object.descriptor().display_name
        ));
        ui.add_space(10.0);

        if ui.checkbox(&mut element.visible, "Visible").changed() {
            self.render_dirty = true;
        }
        ui.checkbox(&mut element.locked, "Locked");
        if ui
            .add(
                DragValue::new(&mut element.z_index)
                    .speed(1.0)
                    .prefix("Layer "),
            )
            .changed()
        {
            self.render_dirty = true;
        }
        ui.add_space(10.0);

        ui.label("Position");
        ui.horizontal(|ui| {
            let position = element.object.position();
            let mut x = position.x;
            let mut y = position.y;
            let mut z = position.z;
            let x_changed = ui
                .add(DragValue::new(&mut x).speed(0.01).prefix("X "))
                .changed();
            let y_changed = ui
                .add(DragValue::new(&mut y).speed(0.01).prefix("Y "))
                .changed();
            let z_changed = ui
                .add(DragValue::new(&mut z).speed(0.01).prefix("Z "))
                .changed();
            if x_changed || y_changed || z_changed {
                element.set_pos(x, y, z);
                self.render_dirty = true;
            }
        });

        ui.label("Size");
        ui.checkbox(&mut element.lock_aspect, "Lock aspect ratio");
        ui.horizontal(|ui| {
            let bounds = element.object.bounds();
            let old_size = vec2(
                bounds.width().max(MIN_OBJECT_SIZE),
                bounds.height().max(MIN_OBJECT_SIZE),
            );
            let mut width = old_size.x;
            let mut height = old_size.y;
            let width_changed = ui
                .add(DragValue::new(&mut width).speed(0.01).prefix("W "))
                .changed();
            let height_changed = ui
                .add(DragValue::new(&mut height).speed(0.01).prefix("H "))
                .changed();
            if width_changed || height_changed {
                let size = inspector_resize_size(
                    old_size,
                    vec2(width, height),
                    width_changed,
                    height_changed,
                    element.lock_aspect,
                );
                element.set_size(size.x, size.y);
                self.render_dirty = true;
            }
        });

        ui.add_space(10.0);
        ui.separator();
        ui.add_space(8.0);

        let mut ctx = InspectorCtx;
        let response = element.object.inspector_ui(ui, &mut ctx);
        if response.request_repaint {
            ui.ctx().request_repaint();
        }
        if response.changed {
            self.render_dirty = true;
        }
    }

    fn log_canvas_resize(&mut self, available: Vec2, canvas_rect: Rect, scale: f32) {
        let available_changed = self
            .last_canvas_available
            .is_none_or(|last| (last - available).length_sq() > 0.25);
        let canvas_changed = self
            .last_canvas_rect
            .is_none_or(|last| rect_changed(last, canvas_rect));

        if available_changed || canvas_changed {
            info!(
                available_width = available.x,
                available_height = available.y,
                canvas_width = canvas_rect.width(),
                canvas_height = canvas_rect.height(),
                scale,
                available_changed,
                canvas_changed,
                "canvas layout changed"
            );
        }
        self.last_canvas_available = Some(available);
        self.last_canvas_rect = Some(canvas_rect);
    }

    fn log_viewport_resize(&mut self, ctx: &Context) {
        let (inner_rect, outer_rect, content_rect, pixels_per_point) = ctx.input(|input| {
            let viewport = input.viewport();
            (
                viewport.inner_rect,
                viewport.outer_rect,
                input.content_rect(),
                input.pixels_per_point,
            )
        });
        let inner_changed = match (self.last_viewport_inner_rect, inner_rect) {
            (Some(last), Some(current)) => rect_changed(last, current),
            (None, None) => false,
            _ => true,
        };
        let content_changed = self
            .last_content_rect
            .is_none_or(|last| rect_changed(last, content_rect));

        if inner_changed || content_changed {
            info!(
                inner_rect = ?inner_rect,
                outer_rect = ?outer_rect,
                content_rect = ?content_rect,
                pixels_per_point,
                inner_changed,
                content_changed,
                "viewport layout changed"
            );
        }
        self.last_viewport_inner_rect = inner_rect;
        self.last_content_rect = Some(content_rect);
    }
}

impl eframe::App for RanimSlideApp {
    fn ui(&mut self, ui: &mut Ui, frame: &mut eframe::Frame) {
        let start = Instant::now();
        let ctx = ui.ctx();
        self.log_viewport_resize(ctx);

        if !ctx.egui_wants_keyboard_input()
            && ctx
                .input(|input| input.key_pressed(Key::Delete) || input.key_pressed(Key::Backspace))
        {
            self.deck.current_page_mut().delete_selected();
            self.render_dirty = true;
        }

        Panel::top("toolbar")
            .exact_size(TOOLBAR_HEIGHT)
            .show_inside(ui, |ui| {
                ui.add_space(5.0);
                self.ui_toolbar(ui);
            });

        Panel::bottom("bottom_bar")
            .exact_size(BOTTOM_BAR_HEIGHT)
            .frame(panel_frame())
            .show_inside(ui, |ui| {
                self.ui_bottom_bar(ui);
            });

        Panel::left("pages")
            .resizable(false)
            .exact_size(LEFT_PANEL_WIDTH)
            .frame(panel_frame())
            .show_inside(ui, |ui| {
                self.ui_pages(ui);
            });

        Panel::right("inspector")
            .resizable(false)
            .exact_size(RIGHT_PANEL_WIDTH)
            .frame(panel_frame())
            .show_inside(ui, |ui| {
                self.ui_right_panel(ui);
            });

        CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(Color32::from_rgb(229, 233, 239))
                    .inner_margin(Margin::same(0)),
            )
            .show_inside(ui, |ui| {
                self.ui_center(ui, frame);
            });
        trace!(
            elapsed_ms = start.elapsed().as_secs_f64() * 1000.0,
            render_dirty = self.render_dirty,
            "app ui frame"
        );
    }
}

fn descriptor_menu_ui(
    ui: &mut Ui,
    descriptors: &[&'static SlideObjectDescriptor],
    depth: usize,
    pending_insert: &mut Option<&'static SlideObjectDescriptor>,
) {
    let mut groups = BTreeMap::<&str, Vec<&'static SlideObjectDescriptor>>::new();
    let mut leaves = Vec::new();

    for descriptor in descriptors {
        let segments = descriptor.type_id.split("::").collect::<Vec<_>>();
        if depth + 1 >= segments.len() {
            leaves.push(*descriptor);
        } else {
            groups.entry(segments[depth]).or_default().push(*descriptor);
        }
    }

    for (segment, group) in groups {
        ui.menu_button(segment, |ui| {
            descriptor_menu_ui(ui, &group, depth + 1, pending_insert);
        });
    }

    for descriptor in leaves {
        if ui.button(descriptor.display_name).clicked() {
            *pending_insert = Some(descriptor);
            ui.close();
        }
    }
}

fn rect_changed(last: Rect, current: Rect) -> bool {
    (last.min - current.min).length_sq() > 0.25 || (last.size() - current.size()).length_sq() > 0.25
}

fn fit_canvas_scale(available: Vec2, frame_size: Vec2) -> f32 {
    ((available.x - CANVAS_PADDING * 2.0) / frame_size.x)
        .min((available.y - CANVAS_PADDING * 2.0) / frame_size.y)
        .clamp(8.0, 160.0)
}

fn canvas_rect_for_view(
    available_rect: Rect,
    frame_size: Vec2,
    scale: f32,
    canvas_pan: Vec2,
) -> Rect {
    Rect::from_center_size(available_rect.center() + canvas_pan, frame_size * scale)
}

fn zoomed_canvas_view_at(
    anchor: Pos2,
    available_rect: Rect,
    frame_size: Vec2,
    fit_scale: f32,
    old_zoom: f32,
    old_pan: Vec2,
    zoom_delta: f32,
) -> Option<(f32, Vec2)> {
    let new_zoom = clamp_canvas_zoom(old_zoom * zoom_delta);
    if (new_zoom - old_zoom).abs() <= f32::EPSILON {
        return None;
    }

    let old_scale = fit_scale * old_zoom;
    let new_scale = fit_scale * new_zoom;
    let old_rect = canvas_rect_for_view(available_rect, frame_size, old_scale, old_pan);
    let local = (anchor - old_rect.min) / old_scale;
    let new_min = anchor - local * new_scale;
    let new_center = new_min + frame_size * new_scale / 2.0;

    Some((new_zoom, new_center - available_rect.center()))
}

fn clamp_canvas_zoom(zoom: f32) -> f32 {
    zoom.clamp(MIN_CANVAS_ZOOM, MAX_CANVAS_ZOOM)
}

fn world_render_size_for_rect(ctx: &Context, frame: &eframe::Frame, rect: Rect) -> (u32, u32) {
    let pixels_per_point = ctx.input(|input| input.pixels_per_point);
    let requested = (
        (rect.width() * pixels_per_point)
            .round()
            .max(MIN_RENDER_SIZE as f32) as u32,
        (rect.height() * pixels_per_point)
            .round()
            .max(MIN_RENDER_SIZE as f32) as u32,
    );

    let Some(render_state) = frame.wgpu_render_state() else {
        return requested;
    };
    let limits = render_state.device.limits();
    cap_render_size_for_wgpu_limits(
        requested,
        limits.max_storage_buffer_binding_size,
        limits.max_texture_dimension_2d,
    )
}

fn cap_render_size_for_wgpu_limits(
    requested: (u32, u32),
    max_storage_buffer_binding_size: u64,
    max_texture_dimension_2d: u32,
) -> (u32, u32) {
    let max_texture_dimension_2d = max_texture_dimension_2d.max(MIN_RENDER_SIZE);
    let (width, height) = (
        requested.0.min(max_texture_dimension_2d),
        requested.1.min(max_texture_dimension_2d),
    );
    let pixel_count = width as u64 * height as u64;
    let bytes_per_pixel = (RENDER_OIT_LAYERS * std::mem::size_of::<u32>()).max(1) as u64;
    let max_pixels = ((max_storage_buffer_binding_size as f64 * WORLD_RENDER_BUFFER_SAFETY)
        / bytes_per_pixel as f64)
        .floor()
        .max(1.0) as u64;

    if pixel_count <= max_pixels {
        return (width, height);
    }

    let scale = (max_pixels as f64 / pixel_count as f64).sqrt();
    (
        ((width as f64 * scale).floor() as u32).max(MIN_RENDER_SIZE),
        ((height as f64 * scale).floor() as u32).max(MIN_RENDER_SIZE),
    )
}

fn default_world_camera(frame: SlideFrame) -> CameraFrame {
    let mut camera = CameraFrame::from_spherical(1.0, -std::f64::consts::FRAC_PI_2, 12.0);
    camera.frame_height = frame.frame_height as f64;
    camera.fovy = std::f64::consts::FRAC_PI_3;
    camera.near = 0.1;
    camera.far = 1000.0;
    camera
}

fn rotate_world_camera(camera: &mut CameraFrame, pointer_delta: Vec2) {
    let mut yaw = camera.facing.y.atan2(camera.facing.x);
    let mut pitch = camera.facing.z.asin();
    yaw -= pointer_delta.x as f64 * WORLD_CAMERA_ROTATE_SPEED;
    pitch = (pitch - pointer_delta.y as f64 * WORLD_CAMERA_ROTATE_SPEED).clamp(-1.45, 1.45);

    let pitch_cos = pitch.cos();
    camera.facing = dvec3(pitch_cos * yaw.cos(), pitch_cos * yaw.sin(), pitch.sin()).normalize();
    let right = camera.facing.cross(DVec3::Z);
    if right.length_squared() > 1.0e-8 {
        camera.up = right.normalize().cross(camera.facing).normalize();
    }
}

fn world_camera_movement(
    camera: &CameraFrame,
    w_down: bool,
    a_down: bool,
    s_down: bool,
    d_down: bool,
    q_down: bool,
    e_down: bool,
) -> DVec3 {
    let forward = camera.facing.normalize();
    let up = camera.up.normalize();
    let right = forward.cross(up).normalize();
    let mut movement = DVec3::ZERO;

    if w_down {
        movement += forward;
    }
    if s_down {
        movement -= forward;
    }
    if d_down {
        movement += right;
    }
    if a_down {
        movement -= right;
    }
    if e_down {
        movement += up;
    }
    if q_down {
        movement -= up;
    }

    if movement.length_squared() > 1.0 {
        movement.normalize()
    } else {
        movement
    }
}

fn paint_render_texture(ui: &Ui, texture_id: Option<TextureId>, rect: Rect, waiting_text: &str) {
    if let Some(texture_id) = texture_id {
        ui.painter().image(
            texture_id,
            rect,
            Rect::from_min_max(Pos2::ZERO, pos2(1.0, 1.0)),
            Color32::WHITE,
        );
    } else {
        ui.painter().rect_filled(
            rect,
            CornerRadius::same(3),
            Color32::from_rgb(250, 251, 253),
        );
        ui.painter().text(
            rect.center(),
            Align2::CENTER_CENTER,
            waiting_text,
            FontId::proportional(14.0),
            Color32::from_rgb(110, 118, 130),
        );
    }
}

fn camera_inspector_ui(ui: &mut Ui, camera: &mut CameraFrame) -> bool {
    let mut changed = false;

    ui.label("Position");
    changed |= camera_vec3_ui(ui, "Pos", &mut camera.pos, 0.05);

    ui.label("Orientation");
    changed |= camera_vec3_ui(ui, "Facing", &mut camera.facing, 0.01);
    changed |= camera_vec3_ui(ui, "Up", &mut camera.up, 0.01);
    if changed {
        if camera.facing.length_squared() > 1.0e-8 {
            camera.facing = camera.facing.normalize();
        }
        if camera.up.length_squared() > 1.0e-8 {
            camera.up = camera.up.normalize();
        }
    }

    ui.add_space(8.0);
    ui.label("Projection");
    changed |= ui
        .add(
            DragValue::new(&mut camera.frame_height)
                .speed(0.05)
                .prefix("Frame H "),
        )
        .changed();
    changed |= ui
        .add(
            DragValue::new(&mut camera.scale)
                .speed(0.01)
                .prefix("Scale "),
        )
        .changed();
    changed |= ui
        .add(egui::Slider::new(&mut camera.perspective_blend, 0.0..=1.0).text("Perspective"))
        .changed();

    let mut fovy_degrees = camera.fovy.to_degrees();
    if ui
        .add(
            DragValue::new(&mut fovy_degrees)
                .speed(0.2)
                .prefix("FOV ")
                .suffix(" deg"),
        )
        .changed()
    {
        camera.fovy = fovy_degrees.clamp(1.0, 170.0).to_radians();
        changed = true;
    }

    ui.add_space(8.0);
    ui.label("Clip");
    ui.horizontal(|ui| {
        changed |= ui
            .add(DragValue::new(&mut camera.near).speed(0.1).prefix("Near "))
            .changed();
        changed |= ui
            .add(DragValue::new(&mut camera.far).speed(0.1).prefix("Far "))
            .changed();
    });

    camera.frame_height = camera.frame_height.max(0.01);
    camera.scale = camera.scale.max(0.01);
    camera.perspective_blend = camera.perspective_blend.clamp(0.0, 1.0);
    camera.fovy = camera
        .fovy
        .clamp(1.0_f64.to_radians(), 170.0_f64.to_radians());
    if camera.far <= camera.near {
        camera.far = camera.near + 0.01;
    }

    changed
}

fn camera_vec3_ui(ui: &mut Ui, label: &str, value: &mut DVec3, speed: f64) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        changed |= ui
            .add(
                DragValue::new(&mut value.x)
                    .speed(speed)
                    .prefix(format!("{label} X ")),
            )
            .changed();
        changed |= ui
            .add(DragValue::new(&mut value.y).speed(speed).prefix("Y "))
            .changed();
        changed |= ui
            .add(DragValue::new(&mut value.z).speed(speed).prefix("Z "))
            .changed();
    });
    changed
}

fn inspector_resize_size(
    old_size: Vec2,
    edited_size: Vec2,
    width_changed: bool,
    height_changed: bool,
    lock_aspect: bool,
) -> Vec2 {
    let old_width = old_size.x.max(MIN_OBJECT_SIZE);
    let old_height = old_size.y.max(MIN_OBJECT_SIZE);
    let mut size = vec2(
        edited_size.x.max(MIN_OBJECT_SIZE),
        edited_size.y.max(MIN_OBJECT_SIZE),
    );

    if lock_aspect {
        let ratio = old_width / old_height;
        if width_changed {
            size.y = (size.x / ratio).max(MIN_OBJECT_SIZE);
        } else if height_changed {
            size.x = (size.y * ratio).max(MIN_OBJECT_SIZE);
        }
    }

    size
}

fn panel_frame() -> egui::Frame {
    egui::Frame::new()
        .fill(Color32::from_rgb(244, 246, 249))
        .inner_margin(Margin::same(12))
        .stroke(Stroke::new(1.0_f32, Color32::from_rgb(210, 216, 224)))
}

fn paint_selection_overlay(ui: &Ui, element: &Element, ctx: &PaintCtx) {
    if !element.visible {
        return;
    }

    let rect = ctx.scene_rect_to_screen(element.object.bounds());
    let painter = ui.painter();

    if element.selected {
        let selection_color = Color32::from_rgb(20, 105, 240);
        painter.rect_stroke(
            rect.expand(3.0),
            CornerRadius::same(2),
            Stroke::new(1.5_f32, selection_color),
            StrokeKind::Outside,
        );

        for point in resize_handles(rect.expand(3.0)) {
            painter.rect_filled(
                Rect::from_center_size(point, vec2(7.0, 7.0)),
                CornerRadius::same(1),
                selection_color,
            );
        }
    }

    let overlay_painter = ui.ctx().layer_painter(LayerId::new(
        Order::Foreground,
        Id::new("slide_selection_overlay"),
    ));
    if element.selected {
        overlay_painter.text(
            rect.left_top() + vec2(0.0, -20.0),
            Align2::LEFT_BOTTOM,
            &element.name,
            FontId::proportional(12.0),
            Color32::from_rgb(20, 105, 240),
        );
    }
}

#[cfg(test)]
fn slide_camera(frame: SlideFrame) -> CameraFrame {
    frame.camera_frame()
}

fn screen_to_scene(pos: Pos2, canvas_rect: Rect, scale: f32, frame: SlideFrame) -> Option<Pos2> {
    const HIT_EPSILON: f32 = 0.001;

    if pos.x < canvas_rect.left()
        || pos.x > canvas_rect.right()
        || pos.y < canvas_rect.top()
        || pos.y > canvas_rect.bottom()
    {
        return None;
    }

    let local = (pos - canvas_rect.min) / scale;
    let frame_min = frame.min();
    let frame_max = frame.max();
    let frame_width = frame.width();
    if local.x < -HIT_EPSILON
        || local.x > frame_width + HIT_EPSILON
        || local.y < -HIT_EPSILON
        || local.y > frame.frame_height + HIT_EPSILON
    {
        return None;
    }

    Some(pos2(
        frame_min.x + local.x.clamp(0.0, frame_width),
        frame_max.y - local.y.clamp(0.0, frame.frame_height),
    ))
}

fn resize_handles(rect: Rect) -> [Pos2; 8] {
    [
        rect.left_top(),
        pos2(rect.center().x, rect.top()),
        rect.right_top(),
        pos2(rect.right(), rect.center().y),
        rect.right_bottom(),
        pos2(rect.center().x, rect.bottom()),
        rect.left_bottom(),
        pos2(rect.left(), rect.center().y),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_screen_positions_to_scene_units() {
        let frame = SlideFrame::default();
        let canvas = Rect::from_min_size(pos2(10.0, 20.0), frame.size() * 100.0);
        let frame_min = frame.min();
        let frame_max = frame.max();

        assert_pos_near(
            screen_to_scene(canvas.left_top(), canvas, 100.0, frame).unwrap(),
            pos2(frame_min.x, frame_max.y),
        );
        assert_pos_near(
            screen_to_scene(canvas.right_bottom(), canvas, 100.0, frame).unwrap(),
            pos2(frame_max.x, frame_min.y),
        );
        assert_pos_near(
            screen_to_scene(canvas.center(), canvas, 100.0, frame).unwrap(),
            pos2(0.0, 0.0),
        );
    }

    #[test]
    fn uses_frame_height_for_camera() {
        assert_eq!(
            slide_camera(SlideFrame::default()).frame_height,
            crate::model::SLIDE_FRAME_HEIGHT as f64
        );
    }

    #[test]
    fn zoom_keeps_anchor_at_same_screen_position() {
        let frame = SlideFrame::default();
        let available = Rect::from_min_size(pos2(0.0, 0.0), vec2(1000.0, 700.0));
        let fit_scale = fit_canvas_scale(available.size(), frame.size());
        let old_zoom = 1.0;
        let old_pan = vec2(80.0, -40.0);
        let anchor = pos2(650.0, 360.0);
        let old_rect = canvas_rect_for_view(available, frame.size(), fit_scale * old_zoom, old_pan);
        let local = (anchor - old_rect.min) / (fit_scale * old_zoom);

        let (new_zoom, new_pan) = zoomed_canvas_view_at(
            anchor,
            available,
            frame.size(),
            fit_scale,
            old_zoom,
            old_pan,
            2.0,
        )
        .unwrap();
        let new_rect = canvas_rect_for_view(available, frame.size(), fit_scale * new_zoom, new_pan);
        let anchored_again = new_rect.min + local * fit_scale * new_zoom;

        assert_pos_near(anchored_again, anchor);
    }

    #[test]
    fn locked_inspector_resize_preserves_ratio_from_width() {
        let size = inspector_resize_size(vec2(4.0, 2.0), vec2(6.0, 2.0), true, false, true);
        assert_vec_near(size, vec2(6.0, 3.0));
    }

    #[test]
    fn locked_inspector_resize_preserves_ratio_from_height() {
        let size = inspector_resize_size(vec2(4.0, 2.0), vec2(4.0, 5.0), false, true, true);
        assert_vec_near(size, vec2(10.0, 5.0));
    }

    #[test]
    fn unlocked_inspector_resize_allows_non_uniform_size() {
        let size = inspector_resize_size(vec2(4.0, 2.0), vec2(7.0, 5.0), true, true, false);
        assert_vec_near(size, vec2(7.0, 5.0));
    }

    #[test]
    fn world_camera_mouse_up_looks_up() {
        let mut camera = CameraFrame {
            facing: DVec3::Y,
            up: DVec3::Z,
            ..CameraFrame::default()
        };

        rotate_world_camera(&mut camera, vec2(0.0, -10.0));

        assert!(camera.facing.z > 0.0, "{:?}", camera.facing);
    }

    #[test]
    fn world_camera_movement_uses_local_basis() {
        let camera = CameraFrame {
            facing: DVec3::Y,
            up: DVec3::Z,
            ..CameraFrame::default()
        };

        assert_dvec3_near(
            world_camera_movement(&camera, true, false, false, false, false, false),
            DVec3::Y,
        );
        assert_dvec3_near(
            world_camera_movement(&camera, false, false, false, true, false, false),
            DVec3::X,
        );
        assert_dvec3_near(
            world_camera_movement(&camera, false, false, false, false, false, true),
            DVec3::Z,
        );
        assert_dvec3_near(
            world_camera_movement(&camera, false, false, false, false, true, false),
            -DVec3::Z,
        );
    }

    #[test]
    fn world_render_size_respects_oit_buffer_limit() {
        let capped = cap_render_size_for_wgpu_limits((2386, 1860), 134_217_728, 8192);
        let oit_bytes = capped.0 as u64 * capped.1 as u64 * RENDER_OIT_LAYERS as u64 * 4;

        assert!(capped.0 < 2386);
        assert!(capped.1 < 1860);
        assert!(
            oit_bytes <= (134_217_728.0 * WORLD_RENDER_BUFFER_SAFETY) as u64,
            "{capped:?} uses {oit_bytes} bytes"
        );
    }

    #[test]
    fn world_render_size_respects_texture_dimension_limit() {
        assert_eq!(
            cap_render_size_for_wgpu_limits((10_000, 100), 134_217_728, 4096),
            (4096, 100)
        );
    }

    fn assert_pos_near(actual: Pos2, expected: Pos2) {
        assert!(
            (actual.x - expected.x).abs() < 0.001,
            "{actual:?} != {expected:?}"
        );
        assert!(
            (actual.y - expected.y).abs() < 0.001,
            "{actual:?} != {expected:?}"
        );
    }

    fn assert_vec_near(actual: Vec2, expected: Vec2) {
        assert!(
            (actual.x - expected.x).abs() < 0.001,
            "{actual:?} != {expected:?}"
        );
        assert!(
            (actual.y - expected.y).abs() < 0.001,
            "{actual:?} != {expected:?}"
        );
    }

    fn assert_dvec3_near(actual: DVec3, expected: DVec3) {
        assert!(
            (actual - expected).length() < 0.001,
            "{actual:?} != {expected:?}"
        );
    }
}
