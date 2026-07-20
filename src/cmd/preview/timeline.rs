use std::{collections::HashSet, ops::Range};

use egui::{
    Align2, Color32, CornerRadius, FontId, PointerButton, Rect, Rgba, ScrollArea, Sense, Stroke,
    TextStyle, Ui, pos2, vec2,
};

use crate::core::{AnimationInfo, AnimationInfoKind, color::palettes::manim};

const HEADER_HEIGHT: f32 = 26.0;
const TRACK_HEIGHT: f32 = 28.0;
const TRACK_GAP: f32 = 2.0;
const INDENT_WIDTH: f32 = 16.0;
const MIN_VISIBLE_SECS: f64 = 0.1;
const MIN_TREE_WIDTH: f32 = 136.0;
const MAX_TREE_WIDTH: f32 = 420.0;

type AnimationPath = Vec<usize>;

#[derive(Clone)]
struct VisibleClip {
    path: AnimationPath,
    global_range: Range<f64>,
    sequence_depth: usize,
}

#[derive(Clone)]
struct SequenceBand {
    path: AnimationPath,
    global_range: Range<f64>,
    depth: usize,
}

struct VisibleTrack {
    path: AnimationPath,
    depth: usize,
    global_range: Range<f64>,
    clips: Vec<VisibleClip>,
    sequence_bands: Vec<SequenceBand>,
}

struct ExpandedStack {
    path: AnimationPath,
    global_range: Range<f64>,
}

pub struct TimelineState {
    pub total_sec: f64,
    pub current_sec: f64,
    width_sec: f64,
    offset_sec: f64,
    animation_infos: Vec<AnimationInfo>,
    expanded: HashSet<AnimationPath>,
    selected: Option<AnimationPath>,
    tree_width: f32,
}

impl TimelineState {
    pub fn new(total_sec: f64, animation_infos: Vec<AnimationInfo>) -> Self {
        let mut expanded = HashSet::new();
        collect_default_expanded(&animation_infos, &mut Vec::new(), 0, &mut expanded);

        let tree_width = preferred_tree_width(&animation_infos);

        Self {
            total_sec,
            current_sec: 0.0,
            width_sec: total_sec.max(MIN_VISIBLE_SECS),
            offset_sec: 0.0,
            animation_infos,
            expanded,
            selected: None,
            tree_width,
        }
    }

    pub fn ui_main_timeline(&mut self, ui: &mut Ui) {
        let available_height = ui.available_height().max(HEADER_HEIGHT + TRACK_HEIGHT);

        egui::Frame::canvas(ui.style()).show(ui, |ui| {
            ScrollArea::vertical().show(ui, |ui| {
                let tracks = self.visible_tracks();
                let tracks_height = tracks.len() as f32 * (TRACK_HEIGHT + TRACK_GAP);
                let content_height = (HEADER_HEIGHT + tracks_height).max(available_height);
                let desired_size = vec2(ui.available_width(), content_height);
                let (canvas, response) =
                    ui.allocate_exact_size(desired_size, Sense::click_and_drag());

                let max_tree_width = (canvas.width() * 0.6).min(MAX_TREE_WIDTH);
                self.tree_width = self
                    .tree_width
                    .clamp(MIN_TREE_WIDTH.min(max_tree_width), max_tree_width);

                let initial_separator_x = canvas.left() + self.tree_width;
                let splitter_rect = Rect::from_min_max(
                    pos2(initial_separator_x - 4.0, canvas.top()),
                    pos2(initial_separator_x + 4.0, canvas.bottom()),
                );
                let splitter_response = ui
                    .interact(
                        splitter_rect,
                        ui.id().with("timeline_tree_splitter"),
                        Sense::click_and_drag(),
                    )
                    .on_hover_cursor(egui::CursorIcon::ResizeHorizontal);
                if splitter_response.dragged() {
                    self.tree_width = (self.tree_width
                        + ui.ctx().input(|input| input.pointer.delta().x))
                    .clamp(MIN_TREE_WIDTH, max_tree_width);
                }
                if splitter_response.double_clicked() {
                    self.tree_width = preferred_tree_width(&self.animation_infos)
                        .clamp(MIN_TREE_WIDTH, max_tree_width);
                }

                let tree_rect = Rect::from_min_max(
                    canvas.min,
                    pos2(
                        (canvas.left() + self.tree_width).min(canvas.right()),
                        canvas.bottom(),
                    ),
                );
                let time_rect =
                    Rect::from_min_max(pos2(tree_rect.right(), canvas.top()), canvas.max);

                self.interact_timeline(ui, &response, time_rect);
                self.paint_timeline(ui, canvas, tree_rect, time_rect, &tracks);
                if splitter_response.hovered() || splitter_response.dragged() {
                    ui.painter().line_segment(
                        [
                            pos2(tree_rect.right(), canvas.top()),
                            pos2(tree_rect.right(), canvas.bottom()),
                        ],
                        Stroke::new(2.0, ui.visuals().selection.stroke.color),
                    );
                }
            });
        });
    }

    fn visible_tracks(&self) -> Vec<VisibleTrack> {
        let mut tracks = Vec::new();
        let mut path = Vec::new();
        for (index, info) in self.animation_infos.iter().enumerate() {
            path.push(index);
            collect_tracks(
                &self.animation_infos,
                info,
                &mut path,
                0,
                info.range.clone(),
                &self.expanded,
                &mut tracks,
            );
            path.pop();
        }
        tracks
    }

    fn interact_timeline(&mut self, ui: &Ui, response: &egui::Response, time_rect: Rect) {
        let Some(pointer_pos) = response.hover_pos() else {
            return;
        };
        if !time_rect.contains(pointer_pos) {
            return;
        }

        if response.dragged_by(PointerButton::Secondary) {
            let delta_sec =
                response.drag_delta().x as f64 / time_rect.width() as f64 * self.width_sec;
            self.offset_sec -= delta_sec;
            self.clamp_viewport();
        } else if response.dragged_by(PointerButton::Primary) || response.clicked() {
            self.current_sec = self
                .sec_from_x(time_rect, pointer_pos.x)
                .clamp(0.0, self.total_sec);
        }

        let zoom_factor = ui.ctx().input(|input| input.zoom_delta_2d().x);
        if zoom_factor != 1.0 && time_rect.width() > 0.0 {
            let anchor_ratio = ((pointer_pos.x - time_rect.left()) / time_rect.width()) as f64;
            let anchor_sec = self.offset_sec + anchor_ratio * self.width_sec;
            let max_width = self.total_sec.max(MIN_VISIBLE_SECS);
            self.width_sec = (self.width_sec / zoom_factor as f64)
                .clamp(MIN_VISIBLE_SECS.min(max_width), max_width);
            self.offset_sec = anchor_sec - anchor_ratio * self.width_sec;
            self.clamp_viewport();
        }
    }

    fn paint_timeline(
        &mut self,
        ui: &Ui,
        canvas: Rect,
        tree_rect: Rect,
        time_rect: Rect,
        tracks: &[VisibleTrack],
    ) {
        let painter = ui.painter_at(canvas);
        let font_id = TextStyle::Body.resolve(ui.style());
        let small_font_id = TextStyle::Small.resolve(ui.style());
        let visuals = ui.visuals();

        painter.rect_filled(tree_rect, 0.0, visuals.panel_fill);
        painter.line_segment(
            [
                pos2(tree_rect.right(), canvas.top()),
                pos2(tree_rect.right(), canvas.bottom()),
            ],
            Stroke::new(1.0, visuals.widgets.noninteractive.bg_stroke.color),
        );

        painter.text(
            pos2(
                tree_rect.left() + 10.0,
                tree_rect.top() + HEADER_HEIGHT * 0.5,
            ),
            Align2::LEFT_CENTER,
            "ANIMATIONS",
            small_font_id.clone(),
            visuals.weak_text_color(),
        );
        painter.line_segment(
            [
                pos2(canvas.left(), canvas.top() + HEADER_HEIGHT),
                pos2(canvas.right(), canvas.top() + HEADER_HEIGHT),
            ],
            Stroke::new(1.0, visuals.widgets.noninteractive.bg_stroke.color),
        );

        self.paint_grid(
            &painter,
            time_rect,
            canvas.bottom(),
            &small_font_id,
            visuals,
        );

        for (track_index, track) in tracks.iter().enumerate() {
            let top =
                canvas.top() + HEADER_HEIGHT + track_index as f32 * (TRACK_HEIGHT + TRACK_GAP);
            let track_rect = Rect::from_min_max(
                pos2(canvas.left(), top),
                pos2(canvas.right(), top + TRACK_HEIGHT),
            );
            let tree_track_rect =
                Rect::from_min_max(track_rect.min, pos2(tree_rect.right(), track_rect.bottom()));
            let time_track_rect =
                Rect::from_min_max(pos2(time_rect.left(), track_rect.top()), track_rect.max);

            if track_index % 2 == 1 {
                painter.rect_filled(track_rect, 0.0, visuals.faint_bg_color);
            }

            self.paint_tree_track(
                ui,
                &painter,
                tree_track_rect,
                track,
                &font_id,
                &small_font_id,
            );
            self.paint_sequence_bands(&painter, time_track_rect, track);
            self.paint_track_clips(ui, &painter, time_track_rect, track, &font_id);
            self.paint_stack_rails(
                &painter,
                time_rect,
                tracks,
                track_index,
                track,
                canvas.top(),
            );
        }

        let playhead_x = self.x_from_sec(time_rect, self.current_sec);
        if time_rect.left() <= playhead_x && playhead_x <= time_rect.right() {
            painter.line_segment(
                [
                    pos2(playhead_x, canvas.top()),
                    pos2(playhead_x, canvas.bottom()),
                ],
                Stroke::new(1.5, visuals.selection.stroke.color),
            );
            let marker = [
                pos2(playhead_x - 5.0, canvas.top()),
                pos2(playhead_x + 5.0, canvas.top()),
                pos2(playhead_x, canvas.top() + 6.0),
            ];
            painter.add(egui::Shape::convex_polygon(
                marker.to_vec(),
                visuals.selection.stroke.color,
                Stroke::NONE,
            ));

            let time_text = format_time(self.current_sec);
            let text_size = painter.layout_no_wrap(
                time_text.clone(),
                small_font_id.clone(),
                visuals.text_color(),
            );
            let badge_width = text_size.rect.width() + 10.0;
            let badge_left = (playhead_x - badge_width * 0.5)
                .clamp(time_rect.left(), time_rect.right() - badge_width);
            let badge_rect = Rect::from_min_size(
                pos2(badge_left, canvas.top() + 3.0),
                vec2(badge_width, HEADER_HEIGHT - 6.0),
            );
            painter.rect_filled(badge_rect, CornerRadius::same(3), visuals.selection.bg_fill);
            painter.text(
                badge_rect.center(),
                Align2::CENTER_CENTER,
                time_text,
                small_font_id,
                visuals.selection.stroke.color,
            );
        }
    }

    fn paint_tree_track(
        &mut self,
        ui: &Ui,
        painter: &egui::Painter,
        rect: Rect,
        track: &VisibleTrack,
        font_id: &FontId,
        small_font_id: &FontId,
    ) {
        let info = animation_info_at_path(&self.animation_infos, &track.path);
        let selected = self.selected.as_ref() == Some(&track.path);
        if selected {
            painter.rect_filled(rect, 0.0, ui.visuals().selection.bg_fill);
        }

        for depth in 0..track.depth {
            let x = rect.left() + 14.0 + depth as f32 * INDENT_WIDTH;
            painter.line_segment(
                [pos2(x, rect.top()), pos2(x, rect.bottom())],
                Stroke::new(1.0, ui.visuals().widgets.noninteractive.bg_stroke.color),
            );
        }

        let indent_x = rect.left() + 6.0 + track.depth as f32 * INDENT_WIDTH;
        let is_expandable_stack =
            info.kind == AnimationInfoKind::Stack && !info.children.is_empty();
        let label_x = if is_expandable_stack {
            let marker = if self.expanded.contains(&track.path) {
                egui_phosphor::regular::CARET_DOWN
            } else {
                egui_phosphor::regular::CARET_RIGHT
            };
            painter.text(
                pos2(indent_x + 8.0, rect.center().y),
                Align2::CENTER_CENTER,
                marker,
                font_id.clone(),
                ui.visuals().text_color(),
            );
            indent_x + 20.0
        } else {
            indent_x + 4.0
        };

        let duration = track.global_range.end - track.global_range.start;
        painter.text(
            pos2(rect.right() - 8.0, rect.center().y),
            Align2::RIGHT_CENTER,
            format_duration(duration),
            small_font_id.clone(),
            ui.visuals().weak_text_color(),
        );

        let label_rect = Rect::from_min_max(
            pos2(label_x, rect.top()),
            pos2(rect.right() - 54.0, rect.bottom()),
        );
        let text_color = if info.enabled {
            ui.visuals().text_color()
        } else {
            ui.visuals().weak_text_color()
        };
        painter.with_clip_rect(label_rect).text(
            pos2(label_x, rect.center().y),
            Align2::LEFT_CENTER,
            display_anim_name(info),
            font_id.clone(),
            text_color,
        );

        let response = ui
            .interact(
                rect,
                ui.id().with(("animation_track", &track.path)),
                Sense::click(),
            )
            .on_hover_text(format!("{}\n{}", info.anim_name, format_duration(duration)));
        if response.clicked() {
            self.selected = Some(track.path.clone());
        }

        if is_expandable_stack {
            let toggle_rect = Rect::from_min_size(
                pos2(indent_x, rect.top() + 4.0),
                vec2(18.0, rect.height() - 8.0),
            );
            let toggle_response = ui.interact(
                toggle_rect,
                ui.id().with(("animation_track_toggle", &track.path)),
                Sense::click(),
            );
            if toggle_response.clicked() {
                toggle_path(&mut self.expanded, &track.path);
            }
        }
    }

    fn paint_sequence_bands(
        &self,
        painter: &egui::Painter,
        track_rect: Rect,
        track: &VisibleTrack,
    ) {
        for band in &track.sequence_bands {
            let start_x = self.x_from_sec(track_rect, band.global_range.start);
            let end_x = self.x_from_sec(track_rect, band.global_range.end);
            if track_rect.right() < start_x || end_x < track_rect.left() {
                continue;
            }

            let vertical_inset = (2.0 + band.depth as f32 * 2.0).min(8.0);
            let rect = Rect::from_min_max(
                pos2(
                    start_x.max(track_rect.left()),
                    track_rect.top() + vertical_inset,
                ),
                pos2(
                    end_x.min(track_rect.right()),
                    track_rect.bottom() - vertical_inset,
                ),
            );
            let selected = self.selected.as_ref() == Some(&band.path);
            let color = if selected {
                painter
                    .ctx()
                    .style_of(egui::Theme::Dark)
                    .visuals
                    .selection
                    .stroke
                    .color
            } else {
                animation_color(AnimationInfoKind::Sequence).gamma_multiply(0.55)
            };
            painter.rect_filled(rect, CornerRadius::same(3), color.gamma_multiply(0.12));
            painter.line_segment(
                [rect.left_bottom(), rect.right_bottom()],
                Stroke::new(if selected { 2.0 } else { 1.0 }, color),
            );
        }
    }

    fn paint_track_clips(
        &mut self,
        ui: &Ui,
        painter: &egui::Painter,
        track_rect: Rect,
        track: &VisibleTrack,
        font_id: &FontId,
    ) {
        for clip in &track.clips {
            let info = animation_info_at_path(&self.animation_infos, &clip.path);
            let start_x = self.x_from_sec(track_rect, clip.global_range.start);
            let end_x = self.x_from_sec(track_rect, clip.global_range.end);
            if track_rect.right() < start_x || end_x < track_rect.left() {
                continue;
            }

            let nested_inset = (5.0 + clip.sequence_depth as f32 * 1.5).min(9.0);
            let zero_duration =
                (clip.global_range.end - clip.global_range.start).abs() < f64::EPSILON;
            let visible_start = start_x.max(track_rect.left());
            let visible_end = if zero_duration {
                (start_x + 4.0).min(track_rect.right())
            } else {
                end_x.min(track_rect.right()).max(visible_start + 1.0)
            };
            let clip_rect = Rect::from_min_max(
                pos2(visible_start, track_rect.top() + nested_inset),
                pos2(visible_end, track_rect.bottom() - nested_inset),
            );

            let response = ui
                .interact(
                    clip_rect.expand2(vec2(2.0, 3.0)),
                    ui.id().with(("animation_clip", &clip.path)),
                    Sense::click(),
                )
                .on_hover_text(format!(
                    "{}\n{} - {}",
                    info.anim_name,
                    format_time(clip.global_range.start),
                    format_time(clip.global_range.end)
                ));
            let selected = self.selected.as_ref() == Some(&clip.path);
            let hovered = response.hovered();
            self.paint_clip_body(
                painter,
                clip_rect,
                info,
                selected,
                hovered,
                zero_duration,
                font_id,
            );

            if response.clicked() {
                self.selected = Some(clip.path.clone());
            }

            if info.kind == AnimationInfoKind::Stack
                && !info.children.is_empty()
                && clip.path != track.path
            {
                let marker = if self.expanded.contains(&clip.path) {
                    egui_phosphor::regular::CARET_DOWN
                } else {
                    egui_phosphor::regular::CARET_RIGHT
                };
                let toggle_rect = Rect::from_min_size(
                    pos2(clip_rect.left() + 2.0, clip_rect.top()),
                    vec2(14.0, clip_rect.height()),
                );
                painter.text(
                    toggle_rect.center(),
                    Align2::CENTER_CENTER,
                    marker,
                    font_id.clone(),
                    Color32::BLACK,
                );
                let toggle_response = ui.interact(
                    toggle_rect.expand(2.0),
                    ui.id().with(("animation_clip_toggle", &clip.path)),
                    Sense::click(),
                );
                if toggle_response.clicked() {
                    toggle_path(&mut self.expanded, &clip.path);
                }
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn paint_clip_body(
        &self,
        painter: &egui::Painter,
        rect: Rect,
        info: &AnimationInfo,
        selected: bool,
        hovered: bool,
        zero_duration: bool,
        font_id: &FontId,
    ) {
        let mut color = animation_color_for_info(info);
        if !info.enabled {
            color = color.gamma_multiply(0.38);
        } else if hovered {
            color = color.gamma_multiply(1.15);
        }

        let border = if selected {
            painter
                .ctx()
                .style_of(egui::Theme::Dark)
                .visuals
                .selection
                .stroke
                .color
        } else {
            color.gamma_multiply(0.72)
        };
        painter.rect_filled(rect, CornerRadius::same(4), border);
        let inner = rect.shrink(if selected { 2.0 } else { 1.0 });
        painter.rect_filled(inner, CornerRadius::same(3), color);

        if zero_duration {
            painter.line_segment(
                [
                    pos2(rect.center().x, rect.top() - 2.0),
                    pos2(rect.center().x, rect.bottom() + 2.0),
                ],
                Stroke::new(2.0, color),
            );
            return;
        }

        painter.line_segment(
            [inner.left_top(), inner.right_top()],
            Stroke::new(1.0, Color32::WHITE.gamma_multiply(0.28)),
        );

        if !info.enabled {
            let mut x = rect.left() - rect.height();
            while x < rect.right() {
                painter.line_segment(
                    [pos2(x, rect.bottom()), pos2(x + rect.height(), rect.top())],
                    Stroke::new(1.0, Color32::BLACK.gamma_multiply(0.35)),
                );
                x += 7.0;
            }
        }

        let label_inset = if info.kind == AnimationInfoKind::Stack && !info.children.is_empty() {
            17.0
        } else {
            5.0
        };
        if rect.width() > label_inset + 24.0 {
            let label_rect = Rect::from_min_max(
                pos2(rect.left() + label_inset, rect.top()),
                pos2(rect.right() - 4.0, rect.bottom()),
            );
            painter.with_clip_rect(label_rect).text(
                pos2(label_rect.left(), label_rect.center().y),
                Align2::LEFT_CENTER,
                display_anim_name(info),
                font_id.clone(),
                Color32::BLACK,
            );
        }
    }

    fn paint_stack_rails(
        &self,
        painter: &egui::Painter,
        time_rect: Rect,
        tracks: &[VisibleTrack],
        track_index: usize,
        track: &VisibleTrack,
        canvas_top: f32,
    ) {
        for clip in &track.clips {
            let info = animation_info_at_path(&self.animation_infos, &clip.path);
            if info.kind != AnimationInfoKind::Stack || !self.expanded.contains(&clip.path) {
                continue;
            }

            let last_descendant = tracks
                .iter()
                .enumerate()
                .skip(track_index + 1)
                .take_while(|(_, candidate)| is_descendant(&candidate.path, &clip.path))
                .map(|(index, _)| index)
                .last();
            let Some(last_descendant) = last_descendant else {
                continue;
            };

            let rail_top = canvas_top
                + HEADER_HEIGHT
                + track_index as f32 * (TRACK_HEIGHT + TRACK_GAP)
                + TRACK_HEIGHT;
            let rail_bottom = canvas_top
                + HEADER_HEIGHT
                + last_descendant as f32 * (TRACK_HEIGHT + TRACK_GAP)
                + TRACK_HEIGHT;
            let color = animation_color(AnimationInfoKind::Stack).gamma_multiply(0.48);
            for sec in [clip.global_range.start, clip.global_range.end] {
                let x = self.x_from_sec(time_rect, sec);
                if time_rect.left() <= x && x <= time_rect.right() {
                    painter.line_segment(
                        [pos2(x, rail_top), pos2(x, rail_bottom)],
                        Stroke::new(1.0, color),
                    );
                }
            }
        }
    }

    fn paint_grid(
        &self,
        painter: &egui::Painter,
        time_rect: Rect,
        bottom: f32,
        font_id: &FontId,
        visuals: &egui::Visuals,
    ) {
        if time_rect.width() <= 0.0 || self.width_sec <= 0.0 {
            return;
        }

        let painter = painter.with_clip_rect(time_rect);

        let header_rect = Rect::from_min_max(
            time_rect.min,
            pos2(time_rect.right(), time_rect.top() + HEADER_HEIGHT),
        );
        painter.rect_filled(header_rect, 0.0, visuals.panel_fill);

        let step = nice_grid_step(self.width_sec, time_rect.width());
        let first = (self.offset_sec / step).floor() as i64;
        let last = ((self.offset_sec + self.width_sec) / step).ceil() as i64;
        let grid_color = visuals.widgets.noninteractive.bg_stroke.color;
        let text_color = visuals.weak_text_color();

        for index in first..=last {
            let sec = index as f64 * step;
            let x = self.x_from_sec(time_rect, sec);
            painter.line_segment(
                [pos2(x, header_rect.bottom()), pos2(x, bottom)],
                Stroke::new(1.0, grid_color.gamma_multiply(0.52)),
            );
            painter.text(
                pos2(x + 5.0, header_rect.center().y),
                Align2::LEFT_CENTER,
                format_grid_time(sec),
                font_id.clone(),
                text_color,
            );
        }
    }

    fn x_from_sec(&self, rect: Rect, sec: f64) -> f32 {
        rect.left() + ((sec - self.offset_sec) / self.width_sec) as f32 * rect.width()
    }

    fn sec_from_x(&self, rect: Rect, x: f32) -> f64 {
        self.offset_sec + ((x - rect.left()) / rect.width()) as f64 * self.width_sec
    }

    fn clamp_viewport(&mut self) {
        let max_width = self.total_sec.max(MIN_VISIBLE_SECS);
        self.width_sec = self
            .width_sec
            .clamp(MIN_VISIBLE_SECS.min(max_width), max_width);
        let max_offset = (self.total_sec - self.width_sec).max(0.0);
        self.offset_sec = self.offset_sec.clamp(0.0, max_offset);
    }
}

fn collect_default_expanded(
    infos: &[AnimationInfo],
    path: &mut AnimationPath,
    stack_depth: usize,
    expanded: &mut HashSet<AnimationPath>,
) {
    for (index, info) in infos.iter().enumerate() {
        path.push(index);
        if info.kind == AnimationInfoKind::Stack && !info.children.is_empty() && stack_depth < 2 {
            expanded.insert(path.clone());
        }
        let next_stack_depth = stack_depth + usize::from(info.kind == AnimationInfoKind::Stack);
        collect_default_expanded(&info.children, path, next_stack_depth, expanded);
        path.pop();
    }
}

#[allow(clippy::too_many_arguments)]
fn collect_tracks(
    roots: &[AnimationInfo],
    info: &AnimationInfo,
    path: &mut AnimationPath,
    depth: usize,
    global_range: Range<f64>,
    expanded: &HashSet<AnimationPath>,
    output: &mut Vec<VisibleTrack>,
) {
    match info.kind {
        AnimationInfoKind::Sequence => {
            let mut clips = Vec::new();
            let mut sequence_bands = Vec::new();
            let mut expanded_stacks = Vec::new();
            collect_sequence_content(
                info,
                path,
                global_range.clone(),
                0,
                expanded,
                &mut clips,
                &mut sequence_bands,
                &mut expanded_stacks,
            );
            output.push(VisibleTrack {
                path: path.clone(),
                depth,
                global_range,
                clips,
                sequence_bands,
            });

            for stack in expanded_stacks {
                let stack_info = animation_info_at_path(roots, &stack.path);
                for (index, child) in stack_info.children.iter().enumerate() {
                    let mut child_path = stack.path.clone();
                    child_path.push(index);
                    let child_range =
                        map_child_range(stack_info, &stack.global_range, &child.range);
                    collect_tracks(
                        roots,
                        child,
                        &mut child_path,
                        depth + 1,
                        child_range,
                        expanded,
                        output,
                    );
                }
            }
        }
        AnimationInfoKind::Stack => {
            output.push(VisibleTrack {
                path: path.clone(),
                depth,
                global_range: global_range.clone(),
                clips: vec![VisibleClip {
                    path: path.clone(),
                    global_range: global_range.clone(),
                    sequence_depth: 0,
                }],
                sequence_bands: Vec::new(),
            });

            if expanded.contains(path) {
                for (index, child) in info.children.iter().enumerate() {
                    path.push(index);
                    let child_range = map_child_range(info, &global_range, &child.range);
                    collect_tracks(roots, child, path, depth + 1, child_range, expanded, output);
                    path.pop();
                }
            }
        }
        AnimationInfoKind::Eval | AnimationInfoKind::Static => {
            output.push(VisibleTrack {
                path: path.clone(),
                depth,
                global_range: global_range.clone(),
                clips: vec![VisibleClip {
                    path: path.clone(),
                    global_range,
                    sequence_depth: 0,
                }],
                sequence_bands: Vec::new(),
            });
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn collect_sequence_content(
    info: &AnimationInfo,
    path: &mut AnimationPath,
    global_range: Range<f64>,
    sequence_depth: usize,
    expanded: &HashSet<AnimationPath>,
    clips: &mut Vec<VisibleClip>,
    bands: &mut Vec<SequenceBand>,
    expanded_stacks: &mut Vec<ExpandedStack>,
) {
    bands.push(SequenceBand {
        path: path.clone(),
        global_range: global_range.clone(),
        depth: sequence_depth,
    });

    for (index, child) in info.children.iter().enumerate() {
        path.push(index);
        let child_range = map_child_range(info, &global_range, &child.range);
        if child.kind == AnimationInfoKind::Sequence {
            collect_sequence_content(
                child,
                path,
                child_range,
                sequence_depth + 1,
                expanded,
                clips,
                bands,
                expanded_stacks,
            );
        } else {
            clips.push(VisibleClip {
                path: path.clone(),
                global_range: child_range.clone(),
                sequence_depth,
            });
            if child.kind == AnimationInfoKind::Stack && expanded.contains(path) {
                expanded_stacks.push(ExpandedStack {
                    path: path.clone(),
                    global_range: child_range,
                });
            }
        }
        path.pop();
    }
}

fn map_child_range(
    parent: &AnimationInfo,
    parent_global_range: &Range<f64>,
    child_range: &Range<f64>,
) -> Range<f64> {
    if parent.content_duration_secs <= 0.0 {
        return parent_global_range.start..parent_global_range.start;
    }

    let duration = parent_global_range.end - parent_global_range.start;
    let start_alpha = child_range.start / parent.content_duration_secs;
    let end_alpha = child_range.end / parent.content_duration_secs;
    parent_global_range.start + duration * start_alpha
        ..parent_global_range.start + duration * end_alpha
}

fn animation_info_at_path<'a>(roots: &'a [AnimationInfo], path: &[usize]) -> &'a AnimationInfo {
    let mut info = &roots[path[0]];
    for &index in &path[1..] {
        info = &info.children[index];
    }
    info
}

fn is_descendant(candidate: &[usize], parent: &[usize]) -> bool {
    candidate.len() > parent.len() && candidate.starts_with(parent)
}

fn toggle_path(expanded: &mut HashSet<AnimationPath>, path: &[usize]) {
    if !expanded.remove(path) {
        expanded.insert(path.to_vec());
    }
}

fn display_anim_name(info: &AnimationInfo) -> &str {
    if info.kind == AnimationInfoKind::Static {
        "Hold"
    } else {
        short_anim_name(&info.anim_name)
    }
}

fn short_anim_name(name: &str) -> &str {
    let outer_type = name.split('<').next().unwrap_or(name);
    let short = outer_type.rsplit("::").next().unwrap_or(outer_type);
    if short == "{{closure}}" {
        "Eval"
    } else {
        short
    }
}

fn preferred_tree_width(infos: &[AnimationInfo]) -> f32 {
    fn required_width(infos: &[AnimationInfo], stack_depth: usize, widest: &mut f32) {
        for info in infos {
            if info.kind != AnimationInfoKind::Sequence || !info.children.is_empty() {
                let indent = stack_depth as f32 * INDENT_WIDTH;
                let toggle = if info.kind == AnimationInfoKind::Stack && !info.children.is_empty() {
                    20.0
                } else {
                    4.0
                };
                let label = display_anim_name(info).chars().count() as f32 * 7.25;
                let duration_column = 54.0;
                *widest = widest.max(6.0 + indent + toggle + label + duration_column);
            }
            let child_depth = stack_depth + usize::from(info.kind == AnimationInfoKind::Stack);
            required_width(&info.children, child_depth, widest);
        }
    }

    let mut widest = MIN_TREE_WIDTH;
    required_width(infos, 0, &mut widest);
    widest.clamp(MIN_TREE_WIDTH, 320.0)
}

fn animation_color_for_info(info: &AnimationInfo) -> Color32 {
    if info.kind == AnimationInfoKind::Eval && info.anim_name.contains("::Static<") {
        animation_color(AnimationInfoKind::Static)
    } else {
        animation_color(info.kind)
    }
}

fn animation_color(kind: AnimationInfoKind) -> Color32 {
    let rgba = match kind {
        AnimationInfoKind::Eval => manim::BLUE_C.to_rgba8(),
        AnimationInfoKind::Sequence => manim::TEAL_C.to_rgba8(),
        AnimationInfoKind::Stack => manim::ORANGE.to_rgba8(),
        AnimationInfoKind::Static => manim::YELLOW_C.to_rgba8(),
    };
    Rgba::from_srgba_unmultiplied(rgba.r, rgba.g, rgba.b, rgba.a).into()
}

fn nice_grid_step(width_sec: f64, width_points: f32) -> f64 {
    let target_lines = (width_points / 90.0).max(1.0) as f64;
    let raw_step = width_sec / target_lines;
    let magnitude = 10.0f64.powf(raw_step.log10().floor());
    let normalized = raw_step / magnitude;
    let factor = if normalized <= 1.0 {
        1.0
    } else if normalized <= 2.0 {
        2.0
    } else if normalized <= 5.0 {
        5.0
    } else {
        10.0
    };
    factor * magnitude
}

fn format_grid_time(sec: f64) -> String {
    if sec.abs() >= 10.0 {
        format!("{sec:.1}s")
    } else {
        format!("{sec:.2}s")
    }
}

fn format_duration(sec: f64) -> String {
    if sec >= 10.0 {
        format!("{sec:.1}s")
    } else {
        format!("{sec:.3}s")
    }
}

fn format_time(sec: f64) -> String {
    format!("{sec:.3}s")
}
