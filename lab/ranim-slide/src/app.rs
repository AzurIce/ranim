use egui::{
    Align2, CentralPanel, Color32, CornerRadius, CursorIcon, DragValue, FontId, Id, Key, LayerId,
    Margin, Order, Panel, Pos2, Rect, Sense, Stroke, StrokeKind, Ui, Vec2, pos2, vec2,
};

use crate::model::{Deck, Element, ElementKind, SLIDE_SIZE, SLIDE_WIDTH};

const LEFT_PANEL_WIDTH: f32 = 188.0;
const RIGHT_PANEL_WIDTH: f32 = 280.0;
const TOOLBAR_HEIGHT: f32 = 44.0;
const CANVAS_PADDING: f32 = 48.0;

pub struct RanimSlideApp {
    deck: Deck,
}

impl RanimSlideApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut fonts = egui::FontDefinitions::default();
        egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);
        cc.egui_ctx.set_fonts(fonts);

        let mut deck = Deck::default();
        deck.add_rect_to_current_page();
        deck.add_text_to_current_page();

        Self { deck }
    }

    fn ui_toolbar(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.heading("Ranim Slide");
            ui.separator();

            if ui.button("Add Page").clicked() {
                self.deck.push_page();
            }

            if ui.button("Rect").clicked() {
                self.deck.add_rect_to_current_page();
            }

            if ui.button("Text").clicked() {
                self.deck.add_text_to_current_page();
            }

            let can_delete_element = self.deck.current_page().selected_element.is_some();
            if ui
                .add_enabled(can_delete_element, egui::Button::new("Delete"))
                .clicked()
            {
                self.deck.current_page_mut().delete_selected();
            }
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
            }
        }

        ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
            if ui
                .add_enabled(self.deck.pages.len() > 1, egui::Button::new("Remove Page"))
                .clicked()
            {
                self.deck.remove_current_page();
            }
        });
    }

    fn ui_canvas(&mut self, ui: &mut Ui) {
        let available = ui.available_size();
        let scale = ((available.x - CANVAS_PADDING * 2.0) / SLIDE_SIZE.x)
            .min((available.y - CANVAS_PADDING * 2.0) / SLIDE_SIZE.y)
            .clamp(0.1, 2.0);
        let canvas_size = SLIDE_SIZE * scale;
        let canvas_min = pos2(
            ui.min_rect().center().x - canvas_size.x / 2.0,
            ui.min_rect().center().y - canvas_size.y / 2.0,
        );
        let canvas_rect = Rect::from_min_size(canvas_min, canvas_size);

        let canvas_response = ui.interact(
            canvas_rect,
            Id::new("slide_canvas"),
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

        let dragged_delta = if canvas_response.dragged() {
            canvas_response.drag_delta() / scale
        } else {
            Vec2::ZERO
        };

        if canvas_response.clicked() {
            let selected = canvas_response
                .interact_pointer_pos()
                .and_then(|pointer_pos| screen_to_slide(pointer_pos, canvas_rect, scale))
                .and_then(|slide_pos| self.deck.current_page().element_at(slide_pos));
            self.deck.current_page_mut().select_element(selected);
        }

        if dragged_delta != Vec2::ZERO {
            if let Some(element) = self.deck.current_page_mut().selected_element_mut() {
                element.translate(dragged_delta);
                ui.ctx().set_cursor_icon(CursorIcon::Grabbing);
            }
        }

        for element in &self.deck.current_page().elements {
            paint_element(ui, element, canvas_rect, scale);
        }

        painter.text(
            canvas_rect.left_bottom() + vec2(0.0, 22.0),
            Align2::LEFT_TOP,
            format!("{:.0}%", scale * 100.0),
            FontId::proportional(12.0),
            Color32::from_rgb(110, 118, 130),
        );
    }

    fn ui_inspector(&mut self, ui: &mut Ui) {
        ui.heading("Inspector");
        ui.add_space(8.0);

        let Some(element) = self.deck.current_page_mut().selected_element_mut() else {
            ui.label("No element selected");
            return;
        };

        ui.label("Name");
        ui.text_edit_singleline(&mut element.name);
        ui.add_space(10.0);

        ui.label("Position");
        ui.horizontal(|ui| {
            let mut x = element.rect.min.x;
            let mut y = element.rect.min.y;
            let x_changed = ui
                .add(DragValue::new(&mut x).speed(1.0).prefix("X "))
                .changed();
            let y_changed = ui
                .add(DragValue::new(&mut y).speed(1.0).prefix("Y "))
                .changed();
            if x_changed || y_changed {
                element.set_pos(x, y);
            }
        });

        ui.label("Size");
        ui.horizontal(|ui| {
            let mut width = element.rect.width();
            let mut height = element.rect.height();
            let width_changed = ui
                .add(DragValue::new(&mut width).speed(1.0).prefix("W "))
                .changed();
            let height_changed = ui
                .add(DragValue::new(&mut height).speed(1.0).prefix("H "))
                .changed();
            if width_changed || height_changed {
                element.set_size(width, height);
            }
        });

        ui.add_space(10.0);
        ui.label("Fill");
        ui.color_edit_button_srgba(&mut element.fill);

        if let ElementKind::Text { content, size } = &mut element.kind {
            ui.add_space(10.0);
            ui.label("Text");
            ui.text_edit_multiline(content);
            ui.add(
                DragValue::new(size)
                    .speed(1.0)
                    .range(8.0..=160.0)
                    .prefix("Size "),
            );
        }
    }
}

impl eframe::App for RanimSlideApp {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx();

        if ctx.input(|input| input.key_pressed(Key::Delete) || input.key_pressed(Key::Backspace)) {
            self.deck.current_page_mut().delete_selected();
        }

        Panel::top("toolbar")
            .exact_size(TOOLBAR_HEIGHT)
            .show_inside(ui, |ui| {
                ui.add_space(5.0);
                self.ui_toolbar(ui);
            });

        Panel::left("pages")
            .exact_size(LEFT_PANEL_WIDTH)
            .frame(panel_frame())
            .show_inside(ui, |ui| {
                self.ui_pages(ui);
            });

        Panel::right("inspector")
            .exact_size(RIGHT_PANEL_WIDTH)
            .frame(panel_frame())
            .show_inside(ui, |ui| {
                self.ui_inspector(ui);
            });

        CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(Color32::from_rgb(229, 233, 239))
                    .inner_margin(Margin::same(0)),
            )
            .show_inside(ui, |ui| {
                self.ui_canvas(ui);
            });
    }
}

fn panel_frame() -> egui::Frame {
    egui::Frame::new()
        .fill(Color32::from_rgb(244, 246, 249))
        .inner_margin(Margin::same(12))
        .stroke(Stroke::new(1.0_f32, Color32::from_rgb(210, 216, 224)))
}

fn paint_element(ui: &Ui, element: &Element, canvas_rect: Rect, scale: f32) {
    let rect = slide_rect_to_screen(element.rect, canvas_rect, scale);
    let painter = ui.painter();

    match &element.kind {
        ElementKind::Rect => {
            painter.rect_filled(rect, CornerRadius::same(2), element.fill);
            painter.rect_stroke(
                rect,
                CornerRadius::same(2),
                Stroke::new(1.0_f32, element.stroke),
                StrokeKind::Outside,
            );
        }
        ElementKind::Text { content, size } => {
            painter.text(
                rect.left_top(),
                Align2::LEFT_TOP,
                content,
                FontId::proportional(size * scale),
                element.fill,
            );
        }
    }

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

fn slide_rect_to_screen(rect: Rect, canvas_rect: Rect, scale: f32) -> Rect {
    Rect::from_min_size(
        canvas_rect.min + rect.min.to_vec2() * scale,
        rect.size() * scale,
    )
}

fn screen_to_slide(pos: Pos2, canvas_rect: Rect, scale: f32) -> Option<Pos2> {
    if !canvas_rect.contains(pos) {
        return None;
    }

    let local = (pos - canvas_rect.min) / scale;
    if !(0.0..=SLIDE_WIDTH).contains(&local.x) || !(0.0..=SLIDE_SIZE.y).contains(&local.y) {
        return None;
    }

    Some(pos2(local.x, local.y))
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
