use egui::{
    Align2, Color32, CornerRadius, FontId, Id, LayerId, Order, Painter, Pos2, Rect, Stroke,
    StrokeKind, Ui, Vec2, pos2, vec2,
};

use crate::object::PaintCtx;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GizmoState {
    pub selected: bool,
    pub locked: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GizmoDash {
    pub length: f32,
    pub gap: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GizmoStroke {
    pub color: Color32,
    pub width: f32,
    pub dash: Option<GizmoDash>,
}

impl GizmoStroke {
    pub fn solid(color: Color32, width: f32) -> Self {
        Self {
            color,
            width,
            dash: None,
        }
    }

    pub fn dashed(color: Color32, width: f32, length: f32, gap: f32) -> Self {
        Self {
            color,
            width,
            dash: Some(GizmoDash { length, gap }),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GizmoMarkerShape {
    Square,
    Circle,
}

#[derive(Debug, Clone)]
pub enum GizmoPrimitive {
    Rect {
        rect: Rect,
        expand_px: f32,
        corner_radius: u8,
        fill: Option<Color32>,
        stroke: GizmoStroke,
    },
    Line {
        start: Pos2,
        end: Pos2,
        stroke: GizmoStroke,
    },
    Marker {
        position: Pos2,
        size_px: f32,
        shape: GizmoMarkerShape,
        fill: Color32,
        stroke: Option<Stroke>,
    },
    Label {
        position: Pos2,
        offset_px: Vec2,
        align: Align2,
        text: String,
        font: FontId,
        color: Color32,
        shadow: bool,
    },
}

#[derive(Debug, Default, Clone)]
pub struct GizmoScene {
    primitives: Vec<GizmoPrimitive>,
}

impl GizmoScene {
    pub fn primitives(&self) -> &[GizmoPrimitive] {
        &self.primitives
    }

    pub fn rect(&mut self, rect: Rect, stroke: GizmoStroke) -> &mut Self {
        self.primitives.push(GizmoPrimitive::Rect {
            rect,
            expand_px: 0.0,
            corner_radius: 0,
            fill: None,
            stroke,
        });
        self
    }

    pub fn line(&mut self, start: Pos2, end: Pos2, stroke: GizmoStroke) -> &mut Self {
        self.primitives
            .push(GizmoPrimitive::Line { start, end, stroke });
        self
    }

    pub fn marker(
        &mut self,
        position: Pos2,
        size_px: f32,
        shape: GizmoMarkerShape,
        fill: Color32,
    ) -> &mut Self {
        self.primitives.push(GizmoPrimitive::Marker {
            position,
            size_px,
            shape,
            fill,
            stroke: None,
        });
        self
    }

    pub fn label(
        &mut self,
        position: Pos2,
        offset_px: Vec2,
        align: Align2,
        text: impl Into<String>,
        color: Color32,
    ) -> &mut Self {
        self.primitives.push(GizmoPrimitive::Label {
            position,
            offset_px,
            align,
            text: text.into(),
            font: FontId::proportional(12.0),
            color,
            shadow: true,
        });
        self
    }

    pub fn selection(&mut self, rect: Rect, name: &str, locked: bool) -> &mut Self {
        let color = if locked {
            Color32::from_rgb(220, 138, 32)
        } else {
            Color32::from_rgb(20, 105, 240)
        };
        self.primitives.push(GizmoPrimitive::Rect {
            rect,
            expand_px: 3.0,
            corner_radius: 2,
            fill: None,
            stroke: GizmoStroke::solid(color, 1.5),
        });

        if !locked {
            for position in rect_handles(rect) {
                self.marker(position, 7.0, GizmoMarkerShape::Square, color);
            }
        }

        self.label(
            pos2(rect.min.x, rect.max.y),
            vec2(0.0, -8.0),
            Align2::LEFT_BOTTOM,
            name,
            color,
        );
        self
    }

    pub fn size_frame(&mut self, rect: Rect) -> &mut Self {
        let color = Color32::from_rgb(0, 150, 136);
        self.rect(rect, GizmoStroke::dashed(color, 1.0, 5.0, 3.0));
        self.primitives.push(GizmoPrimitive::Label {
            position: pos2(rect.max.x, rect.min.y),
            offset_px: vec2(-4.0, -5.0),
            align: Align2::RIGHT_BOTTOM,
            text: format!(
                "{} x {}",
                format_measure(rect.width()),
                format_measure(rect.height())
            ),
            font: FontId::monospace(11.0),
            color,
            shadow: true,
        });
        self
    }
}

pub fn paint_gizmos(ui: &Ui, ctx: &PaintCtx, scene: &GizmoScene) {
    let clip_rect = ui.clip_rect().intersect(ctx.canvas_rect.expand(32.0));
    let painter = ui
        .ctx()
        .layer_painter(LayerId::new(Order::Foreground, Id::new("slide_gizmos")))
        .with_clip_rect(clip_rect);

    for primitive in scene.primitives() {
        match primitive {
            GizmoPrimitive::Rect {
                rect,
                expand_px,
                corner_radius,
                fill,
                stroke,
            } => {
                let screen_rect = ctx.scene_rect_to_screen(*rect).expand(*expand_px);
                if let Some(fill) = fill {
                    painter.rect_filled(screen_rect, CornerRadius::same(*corner_radius), *fill);
                }
                paint_rect_stroke(
                    &painter,
                    screen_rect,
                    CornerRadius::same(*corner_radius),
                    *stroke,
                );
            }
            GizmoPrimitive::Line { start, end, stroke } => {
                paint_line(
                    &painter,
                    ctx.scene_pos_to_screen(*start),
                    ctx.scene_pos_to_screen(*end),
                    *stroke,
                );
            }
            GizmoPrimitive::Marker {
                position,
                size_px,
                shape,
                fill,
                stroke,
            } => {
                let center = ctx.scene_pos_to_screen(*position);
                match shape {
                    GizmoMarkerShape::Square => {
                        let rect = Rect::from_center_size(center, Vec2::splat(*size_px));
                        painter.rect_filled(rect, CornerRadius::same(1), *fill);
                        if let Some(stroke) = stroke {
                            painter.rect_stroke(
                                rect,
                                CornerRadius::same(1),
                                *stroke,
                                StrokeKind::Inside,
                            );
                        }
                    }
                    GizmoMarkerShape::Circle => {
                        painter.circle_filled(center, *size_px / 2.0, *fill);
                        if let Some(stroke) = stroke {
                            painter.circle_stroke(center, *size_px / 2.0, *stroke);
                        }
                    }
                }
            }
            GizmoPrimitive::Label {
                position,
                offset_px,
                align,
                text,
                font,
                color,
                shadow,
            } => {
                let position = ctx.scene_pos_to_screen(*position) + *offset_px;
                if *shadow {
                    painter.text(
                        position + vec2(1.0, 1.0),
                        *align,
                        text,
                        font.clone(),
                        Color32::from_black_alpha(180),
                    );
                }
                painter.text(position, *align, text, font.clone(), *color);
            }
        }
    }
}

fn paint_rect_stroke(painter: &Painter, rect: Rect, radius: CornerRadius, stroke: GizmoStroke) {
    if stroke.dash.is_none() {
        painter.rect_stroke(
            rect,
            radius,
            Stroke::new(stroke.width, stroke.color),
            StrokeKind::Outside,
        );
        return;
    }

    let points = [
        rect.left_top(),
        rect.right_top(),
        rect.right_bottom(),
        rect.left_bottom(),
        rect.left_top(),
    ];
    for edge in points.windows(2) {
        paint_line(painter, edge[0], edge[1], stroke);
    }
}

fn paint_line(painter: &Painter, start: Pos2, end: Pos2, stroke: GizmoStroke) {
    let Some(dash) = stroke.dash else {
        painter.line_segment([start, end], Stroke::new(stroke.width, stroke.color));
        return;
    };

    for [dash_start, dash_end] in dashed_segments(start, end, dash) {
        painter.line_segment(
            [dash_start, dash_end],
            Stroke::new(stroke.width, stroke.color),
        );
    }
}

fn dashed_segments(start: Pos2, end: Pos2, dash: GizmoDash) -> Vec<[Pos2; 2]> {
    let delta = end - start;
    let distance = delta.length();
    if distance <= f32::EPSILON {
        return Vec::new();
    }

    let direction = delta / distance;
    let dash_length = dash.length.max(1.0);
    let period = dash_length + dash.gap.max(0.0);
    let mut offset = 0.0;
    let mut segments = Vec::new();
    while offset < distance {
        let segment_end = (offset + dash_length).min(distance);
        segments.push([start + direction * offset, start + direction * segment_end]);
        offset += period;
    }
    segments
}

fn rect_handles(rect: Rect) -> [Pos2; 8] {
    [
        pos2(rect.min.x, rect.max.y),
        pos2(rect.center().x, rect.max.y),
        rect.max,
        pos2(rect.max.x, rect.center().y),
        pos2(rect.max.x, rect.min.y),
        pos2(rect.center().x, rect.min.y),
        rect.min,
        pos2(rect.min.x, rect.center().y),
    ]
}

fn format_measure(value: f32) -> String {
    if value.abs() >= 100.0 {
        format!("{value:.0}")
    } else if value.abs() >= 10.0 {
        format!("{value:.1}")
    } else {
        format!("{value:.2}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selection_builds_bounds_handles_and_label() {
        let mut scene = GizmoScene::default();
        scene.selection(
            Rect::from_min_max(Pos2::ZERO, pos2(2.0, 1.0)),
            "Text",
            false,
        );

        assert_eq!(scene.primitives().len(), 10);
        assert!(matches!(scene.primitives()[0], GizmoPrimitive::Rect { .. }));
        assert!(matches!(
            scene.primitives()[9],
            GizmoPrimitive::Label { .. }
        ));
    }

    #[test]
    fn locked_selection_hides_resize_handles() {
        let mut scene = GizmoScene::default();
        scene.selection(Rect::from_min_max(Pos2::ZERO, pos2(2.0, 1.0)), "Text", true);

        assert_eq!(scene.primitives().len(), 2);
    }

    #[test]
    fn size_frame_reports_scene_dimensions() {
        let mut scene = GizmoScene::default();
        scene.size_frame(Rect::from_min_max(pos2(-1.0, -0.5), pos2(1.4, 0.5)));

        let GizmoPrimitive::Label { text, .. } = &scene.primitives()[1] else {
            panic!("expected size label");
        };
        assert_eq!(text, "2.40 x 1.00");
    }

    #[test]
    fn dashed_line_keeps_pixel_sized_segments() {
        let segments = dashed_segments(
            Pos2::ZERO,
            pos2(12.0, 0.0),
            GizmoDash {
                length: 4.0,
                gap: 2.0,
            },
        );

        assert_eq!(segments.len(), 2);
        assert_eq!(segments[0], [Pos2::ZERO, pos2(4.0, 0.0)]);
        assert_eq!(segments[1], [pos2(6.0, 0.0), pos2(10.0, 0.0)]);
    }
}
