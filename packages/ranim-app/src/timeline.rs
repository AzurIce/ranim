use egui::{
    Align2, Color32, Frame, PointerButton, Rect, Rgba, ScrollArea, Shape, Stroke, TextStyle,
    emath::GuiRounding, pos2, remap_clamp,
};

use ranim_core::{TimelineInfo, color::palettes::manim};

use super::TimelineInfoState;

pub struct TimelineState {
    pub total_sec: f64,
    pub current_sec: f64,
    pub width_sec: f64,
    pub offset_points: f32,
    pub timeline_infos: Vec<TimelineInfo>,
}

#[allow(unused)]
impl TimelineState {
    pub fn new(total_sec: f64, timeline_infos: Vec<TimelineInfo>) -> Self {
        Self {
            total_sec,
            current_sec: 0.0,
            width_sec: total_sec,
            offset_points: 0.0,
            timeline_infos,
        }
    }
    pub fn ui_preview_timeline(&mut self, ui: &mut egui::Ui) {
        const PREVIEW_HEIGHT: f32 = 30.0;

        Frame::canvas(ui.style()).show(ui, |ui| {
            let mut rect = ui.available_rect_before_wrap();
            // rect.set_bottom(ui.min_rect().bottom());
            rect.set_height(PREVIEW_HEIGHT);

            let font_id = TextStyle::Body.resolve(ui.style());
            let painter = ui.painter_at(rect);
            let shape_id = painter.add(Shape::Noop);
            painter.set(
                shape_id,
                Shape::Vec(paint_time_grid(
                    rect,
                    &painter,
                    font_id,
                    (self.total_sec * 1000.0) as i64,
                    (self.current_sec * 1000.0) as i64,
                )),
            );

            ui.allocate_rect(rect, egui::Sense::hover());
        });
    }

    pub fn interact_preview_timeline(&mut self, info: &TimelineInfoState) {
        let response = &info.response;

        if response.clicked()
            && let Some(mouse_pos) = response.hover_pos()
        {}

        // if response.drag_delta().x != 0.0 {
        //     self.offset_points += response.drag_delta().x;
        // }

        // if response.hovered() {
        //     let mut zoom_factor = info.ctx.input(|i| i.zoom_delta_2d().x);

        //     if response.dragged_by(PointerButton::Secondary) {
        //         let delta = (response.drag_delta().y * 0.01).exp();
        //         // dbg!(delta);
        //         zoom_factor *= delta;
        //     }

        //     // dbg!(state.canvas_width_ms);
        //     if zoom_factor != 1.0 {
        //         self.width_sec /= zoom_factor as f64;
        //         if let Some(mouse_pos) = response.hover_pos() {
        //             let zoom_center = mouse_pos.x - info.canvas.min.x;
        //             self.offset_points =
        //                 (self.offset_points - zoom_center) * zoom_factor + zoom_center;
        //         }
        //     }
        // }

        // // Reset view
        // if response.double_clicked() {
        //     // TODO
        // }
    }

    pub fn ui_main_timeline(&mut self, ui: &mut egui::Ui) {
        Frame::canvas(ui.style()).show(ui, |ui| {
            let available_height = ui.max_rect().bottom() - ui.min_rect().bottom();
            ScrollArea::vertical().show(ui, |ui| {
                let mut canvas = ui.available_rect_before_wrap();
                canvas.max.y = f32::INFINITY;

                let response = ui.interact(
                    canvas,
                    ui.id().with("canvas"),
                    egui::Sense::click_and_drag(),
                );
                let info = TimelineInfoState {
                    ctx: ui.ctx().clone(),
                    canvas,
                    response,
                    painter: ui.painter_at(canvas),
                    text_height: 15.0,
                    font_id: TextStyle::Body.resolve(ui.style()),
                };

                self.interact_main_timeline(&info);

                let timeline_shape_id = info.painter.add(Shape::Noop);

                let max_y = ui_canvas(self, &info);
                let mut used_rect = canvas;
                used_rect.max.y = max_y.max(used_rect.min.y + available_height);

                info.painter.set(
                    timeline_shape_id,
                    Shape::Vec(paint_timeline(
                        &info,
                        used_rect,
                        self,
                        (self.current_sec * 1000.0) as i64,
                    )),
                );

                ui.allocate_rect(used_rect, egui::Sense::hover());
            });
        });
    }

    pub fn interact_main_timeline(&mut self, info: &TimelineInfoState) {
        let response = &info.response;

        if response.drag_delta().x != 0.0 {
            self.offset_points += response.drag_delta().x;
        }

        if response.hovered() {
            let mut zoom_factor = info.ctx.input(|i| i.zoom_delta_2d().x);

            if response.dragged_by(PointerButton::Secondary) {
                let delta = (response.drag_delta().y * 0.01).exp();
                // dbg!(delta);
                zoom_factor *= delta;
            }

            // dbg!(state.canvas_width_ms);
            if zoom_factor != 1.0 {
                let old_width_sec = self.width_sec;
                self.width_sec /= zoom_factor as f64;
                self.width_sec = self.width_sec.clamp(100.0 / 1000.0, self.total_sec);
                zoom_factor = (old_width_sec / self.width_sec) as f32;
                if let Some(mouse_pos) = response.hover_pos() {
                    let zoom_center = mouse_pos.x - info.canvas.min.x;
                    self.offset_points =
                        (self.offset_points - zoom_center) * zoom_factor + zoom_center;
                }
            }
        }

        // Reset view
        if response.double_clicked() {
            // TODO
        }
    }
}

pub fn ui_canvas(state: &mut TimelineState, info: &TimelineInfoState) -> f32 {
    let line_height = 16.0;
    let gap = 4.0;

    let mut start_y = info.canvas.top();
    start_y += info.text_height; // Time labels
    let end_y = start_y + state.timeline_infos.len() as f32 * (line_height + gap);

    for (idx, timeline_info) in state.timeline_infos.iter().enumerate() {
        let local_y = idx as f32 * (line_height + gap);

        let top_y = start_y + local_y;
        let bottom_y = top_y + line_height;

        for animation_info in &timeline_info.animation_infos {
            // if animation_info.anim_name.as_str() == "Static" {
            //     continue;
            // }
            let start_x = info.point_from_ms(state, (animation_info.range.start * 1000.0) as i64);
            let end_x = info.point_from_ms(state, (animation_info.range.end * 1000.0) as i64);

            if info.canvas.max.x < start_x || end_x < info.canvas.min.x {
                continue;
            }

            let rect = Rect::from_min_max(pos2(start_x, top_y), pos2(end_x, bottom_y));
            let rect_color = if animation_info
                .anim_name
                .starts_with("ranim_core::animation::Static")
            {
                manim::YELLOW_C.to_rgba8()
            } else {
                manim::BLUE_C.to_rgba8()
            };

            info.painter.rect_filled(
                rect,
                4.0,
                egui::Rgba::from_srgba_unmultiplied(
                    rect_color.r,
                    rect_color.g,
                    rect_color.b,
                    (0.9 * 255.0) as u8,
                ),
            );

            let wide_enough_for_text = end_x - start_x > 32.0;
            if wide_enough_for_text {
                let text = format!(
                    "{} {:6.3} s",
                    animation_info.anim_name,
                    animation_info.range.end - animation_info.range.start
                );

                let painter = info.painter.with_clip_rect(rect.intersect(info.canvas));

                let pos = pos2(start_x + 4.0, top_y + 0.5 * (16.0 - info.text_height));
                let pos = pos.round_to_pixels(painter.pixels_per_point());
                const TEXT_COLOR: Color32 = Color32::BLACK;
                painter.text(
                    pos,
                    Align2::LEFT_TOP,
                    text,
                    info.font_id.clone(),
                    TEXT_COLOR,
                );
            }
        }
    }

    end_y
}

pub fn paint_time_grid(
    rect: egui::Rect,
    painter: &egui::Painter,
    font_id: egui::FontId,
    width_ms: i64,
    current_ms: i64,
) -> Vec<egui::Shape> {
    if width_ms <= 0 {
        return vec![];
    }

    let mut shapes = vec![];

    let alpha_multiplier = 0.3;

    // The maximum number of lines, 4 pixels gap
    let max_lines = (rect.width() / 4.0).floor() as i64;
    // The minimum grid spacing, 1 ms
    let mut grid_spacing_ms = 1;
    // Increase the grid spacing until it's less than the maximum number of lines
    while width_ms / grid_spacing_ms > max_lines {
        grid_spacing_ms *= 10;
    }

    let num_tiny_lines = width_ms / grid_spacing_ms;
    let zoom_factor = remap_clamp(
        num_tiny_lines as f32,
        (0.1 * max_lines as f32)..=max_lines as f32,
        1.0..=0.0,
    );
    let zoom_factor = zoom_factor * zoom_factor;
    let big_alpha = remap_clamp(zoom_factor, 0.0..=1.0, 0.5..=1.0);
    let medium_alpha = remap_clamp(zoom_factor, 0.0..=1.0, 0.1..=0.5);
    let tiny_alpha = remap_clamp(zoom_factor, 0.0..=1.0, 0.0..=0.1);

    let ppms = rect.width() / width_ms as f32;
    (0..num_tiny_lines).for_each(|i| {
        let ms = grid_spacing_ms * i;
        let line_x = rect.min.x + ms as f32 * ppms;

        let big_line = ms % (grid_spacing_ms * 100) == 0;
        let medium_line = ms % (grid_spacing_ms * 10) == 0;

        let line_alpha = if big_line {
            big_alpha
        } else if medium_line {
            medium_alpha
        } else {
            tiny_alpha
        };

        shapes.push(egui::Shape::line_segment(
            [pos2(line_x, rect.min.y), pos2(line_x, rect.max.y)],
            Stroke::new(1.0, Rgba::from_white_alpha(line_alpha * alpha_multiplier)),
        ));

        let text_alpha = if big_line {
            medium_alpha
        } else if medium_line {
            tiny_alpha
        } else {
            0.0
        };

        if text_alpha > 0.0 {
            let text = grid_text(ms);
            let text_x = line_x + 4.0;
            let text_color = Rgba::from_white_alpha((text_alpha * 2.0).min(1.0)).into();
            // Timestamp on top
            painter.fonts_mut(|f| {
                shapes.push(egui::Shape::text(
                    f,
                    pos2(text_x, rect.min.y),
                    Align2::LEFT_TOP,
                    &text,
                    font_id.clone(),
                    text_color,
                ));
            });
            // Timestamp on bottom
            painter.fonts_mut(|f| {
                shapes.push(egui::Shape::text(
                    f,
                    pos2(text_x, rect.max.y - 12.0),
                    Align2::LEFT_TOP,
                    &text,
                    font_id.clone(),
                    text_color,
                ));
            });
        }
    });

    let current_line_x = current_ms as f32 * ppms;
    shapes.push(egui::Shape::line_segment(
        [
            pos2(current_line_x, rect.min.y),
            pos2(current_line_x, rect.max.y),
        ],
        Stroke::new(1.0, Rgba::from_white_alpha(alpha_multiplier)),
    ));
    shapes
}

pub fn paint_timeline(
    info: &TimelineInfoState,
    rect: egui::Rect,
    state: &TimelineState,
    current_ms: i64,
) -> Vec<egui::Shape> {
    let mut shapes = vec![];

    if state.width_sec <= 0.0 {
        return shapes;
    }

    let alpha_multiplier = 0.3;

    let start_ms = 0;
    // The maximum number of lines, 4 pixels gap
    let max_lines = rect.width() / 4.0;
    // The minimum grid spacing, 1 ms
    let mut grid_spacing_ms = 1;
    // Increase the grid spacing until it's less than the maximum number of lines
    while state.width_sec as f32 * 1000.0 / grid_spacing_ms as f32 > max_lines {
        grid_spacing_ms *= 10;
    }
    // dbg!(state.sideways_pan_in_points);
    // dbg!(state.canvas_width_ms);
    // dbg!(grid_spacing_ms);

    let num_tiny_lines = state.width_sec as f32 * 1000.0 / grid_spacing_ms as f32;
    let zoom_factor = remap_clamp(num_tiny_lines, (0.1 * max_lines)..=max_lines, 1.0..=0.0);
    let zoom_factor = zoom_factor * zoom_factor;
    let big_alpha = remap_clamp(zoom_factor, 0.0..=1.0, 0.5..=1.0);
    let medium_alpha = remap_clamp(zoom_factor, 0.0..=1.0, 0.1..=0.5);
    let tiny_alpha = remap_clamp(zoom_factor, 0.0..=1.0, 0.0..=0.1);

    let mut grid_ms = 0;

    let current_line_x = info.point_from_ms(state, current_ms);
    shapes.push(egui::Shape::line_segment(
        [
            pos2(current_line_x, rect.min.y),
            pos2(current_line_x, rect.max.y),
        ],
        Stroke::new(1.0, Rgba::from_white_alpha(alpha_multiplier)),
    ));
    loop {
        let line_x = info.point_from_ms(state, start_ms + grid_ms);
        if line_x > rect.max.x {
            break;
        }
        if rect.min.x <= line_x {
            let big_line = grid_ms % (grid_spacing_ms * 100) == 0;
            let medium_line = grid_ms % (grid_spacing_ms * 10) == 0;

            let line_alpha = if big_line {
                big_alpha
            } else if medium_line {
                medium_alpha
            } else {
                tiny_alpha
            };

            shapes.push(egui::Shape::line_segment(
                [pos2(line_x, rect.min.y), pos2(line_x, rect.max.y)],
                Stroke::new(1.0, Rgba::from_white_alpha(line_alpha * alpha_multiplier)),
            ));

            let text_alpha = if big_line {
                medium_alpha
            } else if medium_line {
                tiny_alpha
            } else {
                0.0
            };

            if text_alpha > 0.0 {
                let text = grid_text(grid_ms);
                let text_x = line_x + 4.0;
                let text_color = Rgba::from_white_alpha((text_alpha * 2.0).min(1.0)).into();
                // Timestamp on top
                info.painter.fonts_mut(|f| {
                    shapes.push(egui::Shape::text(
                        f,
                        pos2(text_x, rect.min.y),
                        Align2::LEFT_TOP,
                        &text,
                        info.font_id.clone(),
                        text_color,
                    ));
                });
                // Timestamp on bottom
                info.painter.fonts_mut(|f| {
                    shapes.push(egui::Shape::text(
                        f,
                        pos2(text_x, rect.max.y - info.text_height),
                        Align2::LEFT_TOP,
                        &text,
                        info.font_id.clone(),
                        text_color,
                    ));
                });
            }
        }

        grid_ms += grid_spacing_ms;
    }

    // println!("paint_timeline: {:?}", shapes.len());

    shapes
}

fn grid_text(grid_ms: i64) -> String {
    let sec = grid_ms as f64 / 1000.0;
    if grid_ms % 1_000 == 0 {
        format!("{sec:.0} s")
    } else if grid_ms % 100 == 0 {
        format!("{sec:.1} s")
    } else if grid_ms % 10 == 0 {
        format!("{sec:.2} s")
    } else {
        format!("{sec:.3} s")
    }
}
