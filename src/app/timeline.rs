use egui::{Align2, PointerButton, Rgba, Stroke, pos2, remap_clamp};

use super::{TimelineInfo, TimelineState};

pub fn ui_canvas(
    state: &mut TimelineState,
    info: &TimelineInfo,
    (min_ms, max_ms): (i64, i64),
) -> f32 {
    if state.canvas_width_ms <= 0.0 {
        state.canvas_width_ms = (max_ms - min_ms) as f32;
    }
    let mut cursor_y = info.canvas.top();
    cursor_y += info.text_height; // Time labels

    cursor_y
}

pub fn interact_with_canvas(state: &mut TimelineState, info: &TimelineInfo) {
    let response = &info.response;

    if response.drag_delta().x != 0.0 {
        state.sideways_pan_in_points += response.drag_delta().x;
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
            state.canvas_width_ms /= zoom_factor;
            if let Some(mouse_pos) = response.hover_pos() {
                let zoom_center = mouse_pos.x - info.canvas.min.x;
                state.sideways_pan_in_points =
                    (state.sideways_pan_in_points - zoom_center) * zoom_factor + zoom_center;
            }
        }
    }

    // Reset view
    if response.double_clicked() {
        // TODO
    }
}

pub fn paint_timeline(
    info: &TimelineInfo,
    rect: egui::Rect,
    state: &TimelineState,
    start_ms: i64,
) -> Vec<egui::Shape> {
    let mut shapes = vec![];

    if state.canvas_width_ms <= 0.0 {
        return shapes;
    }

    let alpha_multiplier = 0.3;

    // The maximum number of lines, 4 pixels gap
    let max_lines = rect.width() / 4.0;
    // The minimum grid spacing, 1 ms
    let mut grid_spacing_ms = 1;
    // Increase the grid spacing until it's less than the maximum number of lines
    while state.canvas_width_ms / (grid_spacing_ms as f32) > max_lines {
        grid_spacing_ms *= 10;
    }
    // dbg!(state.sideways_pan_in_points);
    // dbg!(state.canvas_width_ms);
    // dbg!(grid_spacing_ms);

    let num_tiny_lines = state.canvas_width_ms / grid_spacing_ms as f32;
    let zoom_factor = remap_clamp(num_tiny_lines, (0.1 * max_lines)..=max_lines, 1.0..=0.0);
    let zoom_factor = zoom_factor * zoom_factor;
    let big_alpha = remap_clamp(zoom_factor, 0.0..=1.0, 0.5..=1.0);
    let medium_alpha = remap_clamp(zoom_factor, 0.0..=1.0, 0.1..=0.5);
    let tiny_alpha = remap_clamp(zoom_factor, 0.0..=1.0, 0.0..=0.1);

    let mut grid_ms = 0;

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
                info.painter.fonts(|f| {
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
                info.painter.fonts(|f| {
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
    } else if grid_ms % 1000 == 0 {
        format!("{sec:.1} s")
    } else if grid_ms % 10_000 == 0 {
        format!("{sec:.2} s")
    } else {
        format!("{sec:.3} s")
    }
}
