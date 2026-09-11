//! In-app GPU profiler panel for the preview app.
//!
//! Shows per-pass GPU timer scopes (needs the `profiling` feature and a
//! device with timestamp query support), a frame-time history sparkline,
//! and per-buffer upload statistics from [`crate::render::upload_probe`]
//! (works in any build; the upload-strategy modes can be switched live).

use std::collections::VecDeque;

use eframe::egui;

use super::RanimPreviewApp;
use crate::render::upload_probe::UploadMode;

/// Ring-buffer length for the frame history sparklines.
pub(crate) const HISTORY_LEN: usize = 600;

/// GPU timer features requested from eframe's device so wgpu-profiler
/// scopes produce results (intersected with adapter support at the call
/// site, so device creation can never fail because of them).
#[cfg(feature = "profiling")]
pub(crate) fn gpu_timer_features() -> wgpu::Features {
    wgpu::Features::TIMESTAMP_QUERY
        | wgpu::Features::TIMESTAMP_QUERY_INSIDE_ENCODERS
        | wgpu::Features::TIMESTAMP_QUERY_INSIDE_PASSES
}

/// Flatten a wgpu-profiler scope tree into `(label, μs)` pairs.
#[cfg(feature = "profiling")]
pub(crate) fn flatten_scopes(
    scopes: &[wgpu_profiler::GpuTimerQueryResult],
    out: &mut Vec<(String, f64)>,
) {
    for scope in scopes {
        if let Some(time) = &scope.time {
            out.push((scope.label.clone(), (time.end - time.start) * 1e6));
        }
        flatten_scopes(&scope.nested_queries, out);
    }
}

pub(crate) fn ui_profiler_window(app: &mut RanimPreviewApp, ctx: &egui::Context) {
    let mut open = app.profiler_open;
    egui::Window::new("GPU Profiler")
        .open(&mut open)
        .default_size([440.0, 540.0])
        .show(ctx, |ui| {
            ui_summary(app, ui);
            ui_gpu_passes(app, ui);
            ui_gpu_history(app, ui);
            ui_uploads(app, ui);
        });
    app.profiler_open = open;
}

fn ui_summary(app: &mut RanimPreviewApp, ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        if let Some(d) = app.last_render_time {
            ui.label(format!("Render: {:.2} ms", d.as_secs_f64() * 1e3));
        }
        if let Some(d) = app.last_eval_time {
            ui.label(format!("Eval: {:.2} ms", d.as_secs_f64() * 1e3));
        }
        if let Some(total) = gpu_total_us(app) {
            ui.strong(format!("GPU total: {total:.0} μs"));
        }
    });
}

fn gpu_total_us(app: &RanimPreviewApp) -> Option<f64> {
    (!app.gpu_pass_times.is_empty()).then(|| app.gpu_pass_times.iter().map(|&(_, t)| t).sum())
}

fn ui_gpu_passes(app: &mut RanimPreviewApp, ui: &mut egui::Ui) {
    ui.add_space(4.0);
    ui.heading("GPU passes");
    if !cfg!(feature = "profiling") {
        ui.label(
            egui::RichText::new("GPU timers unavailable — rebuild with `--features profiling`")
                .weak(),
        );
        return;
    }
    if app.gpu_pass_times.is_empty() {
        ui.label(
            egui::RichText::new(
                "no GPU timer data yet — play the animation, or the adapter may \
                 lack timestamp query support",
            )
            .weak(),
        );
        return;
    }
    let Some(total) = gpu_total_us(app) else {
        return;
    };
    egui::Grid::new("gpu_pass_grid")
        .num_columns(3)
        .striped(true)
        .show(ui, |ui| {
            ui.strong("pass");
            ui.strong("time");
            ui.strong("share");
            ui.end_row();
            for (label, us) in &app.gpu_pass_times {
                ui.label(label);
                ui.label(format!("{us:.1} μs"));
                let share = if total > 0.0 { *us / total } else { 0.0 };
                share_bar(ui, share as f32);
                ui.end_row();
            }
            ui.strong("total");
            ui.strong(format!("{total:.1} μs"));
            ui.label("");
            ui.end_row();
        });
}

fn ui_gpu_history(app: &mut RanimPreviewApp, ui: &mut egui::Ui) {
    if app.gpu_frame_history.len() < 2 {
        return;
    }
    ui.add_space(4.0);
    sparkline(ui, &app.gpu_frame_history, "GPU total per frame (μs)");
}

fn ui_uploads(app: &mut RanimPreviewApp, ui: &mut egui::Ui) {
    ui.add_space(8.0);
    ui.heading("Buffer uploads");
    ui.horizontal(|ui| {
        ui.label("mode:");
        let prev = crate::render::upload_probe::mode();
        let mut mode = prev;
        egui::ComboBox::from_id_salt("upload_probe_mode")
            .selected_text(format!("{mode:?}"))
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut mode, UploadMode::Off, "Off");
                ui.selectable_value(&mut mode, UploadMode::Count, "Count");
                ui.selectable_value(&mut mode, UploadMode::SkipEqual, "SkipEqual");
                ui.selectable_value(&mut mode, UploadMode::DirtyRanges, "DirtyRanges");
            });
        if mode != prev {
            crate::render::upload_probe::set_mode(mode);
        }
    });

    if app.upload_stats.is_empty() {
        if crate::render::upload_probe::mode().enabled() {
            ui.label(
                egui::RichText::new("waiting for a rendered frame (play or scrub the timeline)")
                    .weak(),
            );
        }
        return;
    }

    let (mut calls, mut bytes, mut written, mut cpu_ns) = (0u64, 0u64, 0u64, 0u128);
    egui::Grid::new("upload_stats_grid")
        .num_columns(5)
        .striped(true)
        .show(ui, |ui| {
            ui.strong("buffer");
            ui.strong("calls");
            ui.strong("KiB");
            ui.strong("written");
            ui.strong("cpu μs");
            ui.end_row();
            for (label, s) in &app.upload_stats {
                calls += s.calls;
                bytes += s.bytes;
                written += s.written_bytes;
                cpu_ns += s.cpu_time.as_nanos();
                ui.label(*label);
                ui.label(s.calls.to_string());
                ui.label(format!("{:.1}", s.bytes as f64 / 1024.0));
                let pct = if s.bytes > 0 {
                    100.0 * s.written_bytes as f64 / s.bytes as f64
                } else {
                    0.0
                };
                ui.label(format!("{pct:.0}%"));
                ui.label(format!("{:.1}", s.cpu_time.as_secs_f64() * 1e6));
                ui.end_row();
            }
            ui.strong("TOTAL");
            ui.strong(calls.to_string());
            ui.strong(format!("{:.1}", bytes as f64 / 1024.0));
            let pct = if bytes > 0 {
                100.0 * written as f64 / bytes as f64
            } else {
                0.0
            };
            ui.strong(format!("{pct:.0}%"));
            ui.strong(format!("{:.1}", cpu_ns as f64 / 1e3));
            ui.end_row();
        });
    ui.label(
        egui::RichText::new(
            "per rendered frame; `written` counts bytes actually handed to \
             write_buffer (skipped/range uploads excluded)",
        )
        .small()
        .weak(),
    );

    if app.upload_written_history.len() >= 2 {
        ui.add_space(4.0);
        sparkline(ui, &app.upload_written_history, "written KiB per frame");
    }
}

fn share_bar(ui: &mut egui::Ui, frac: f32) {
    let (rect, _) = ui.allocate_exact_size(
        egui::vec2(96.0, ui.spacing().interact_size.y),
        egui::Sense::hover(),
    );
    let frac = frac.clamp(0.0, 1.0);
    ui.painter()
        .rect_filled(rect, 2.0, ui.visuals().faint_bg_color);
    ui.painter().rect_filled(
        egui::Rect::from_min_size(rect.min, egui::vec2(rect.width() * frac, rect.height())),
        2.0,
        ui.visuals().selection.bg_fill,
    );
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        format!("{:.0}%", frac * 100.0),
        egui::FontId::proportional(10.0),
        ui.visuals().text_color(),
    );
}

fn sparkline(ui: &mut egui::Ui, data: &VecDeque<f64>, label: &str) {
    let (rect, _) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), 44.0), egui::Sense::hover());
    let max = data.iter().copied().fold(0.0f64, f64::max).max(1e-9);
    let min = data.iter().copied().fold(f64::INFINITY, f64::min);
    let avg = data.iter().sum::<f64>() / data.len() as f64;

    ui.painter().rect_stroke(
        rect,
        2.0,
        ui.visuals().widgets.inactive.bg_stroke,
        egui::StrokeKind::Inside,
    );

    let n = data.len();
    let points: Vec<egui::Pos2> = data
        .iter()
        .enumerate()
        .map(|(i, v)| {
            let t = i as f32 / (n as f32 - 1.0).max(1.0);
            let x = egui::lerp(rect.left()..=rect.right(), t);
            let y = egui::lerp(rect.bottom()..=rect.top(), (*v / max) as f32);
            egui::pos2(x, y)
        })
        .collect();
    if points.len() >= 2 {
        ui.painter().add(egui::Shape::line(
            points,
            egui::Stroke::new(1.5, ui.visuals().selection.bg_fill),
        ));
    }
    ui.painter().text(
        rect.left_top() + egui::vec2(4.0, 2.0),
        egui::Align2::LEFT_TOP,
        format!("{label}   min {:.0} / avg {:.0} / max {:.0}", min, avg, max),
        egui::FontId::proportional(10.0),
        ui.visuals().weak_text_color(),
    );
}
