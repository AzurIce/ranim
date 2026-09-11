//! In-app profiler panel for the preview app (CPU + GPU + uploads).
//!
//! The main view is a **progress chart**: the X axis is scene progress
//! (0..total_sec, one bucket per logic frame) and every rendered frame
//! writes its sample into the bucket at the current timeline position —
//! playing, seeking or dragging the chart progressively fills (and on
//! revisit, refreshes) the samples, so performance variation across the
//! animation becomes visible. The chart can be clicked/dragged to seek.
//!
//! CPU spans ([`crate::render::cpu_probe`]) and upload stats
//! ([`crate::render::upload_probe`]) work in any build; GPU pass scopes
//! need the `profiling` feature and a device with timestamp query
//! support. Upload strategy modes can be switched live.

use eframe::egui;

use super::RanimPreviewApp;
use crate::render::upload_probe::UploadMode;

/// One sampled rendering of the scene at a specific timeline position.
#[derive(Clone, Default)]
pub(crate) struct ProgressSample {
    pub gpu_total_us: f64,
    /// Per-pass GPU times in μs (render order).
    pub gpu_passes: Vec<(String, f64)>,
    /// CPU span total in ms (leaf-level spans, no double counting).
    pub cpu_total_ms: f64,
    /// CPU spans in ms (leaf-level, e.g. eval / prepare_vitems / render_graph).
    pub cpu_spans: Vec<(String, f64)>,
    pub render_ms: f64,
    pub eval_ms: f64,
    pub upload_total_kib: f64,
    pub upload_written_kib: f64,
}

/// Metric drawn in the progress chart.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum ProfilerMetric {
    /// Stacked per-pass GPU times.
    #[default]
    GpuStacked,
    GpuTotalUs,
    /// Stacked CPU spans.
    CpuStacked,
    CpuTotalMs,
    UploadWrittenKiB,
    UploadTotalKiB,
    RenderMs,
    EvalMs,
}

impl ProfilerMetric {
    fn label(self) -> &'static str {
        match self {
            ProfilerMetric::GpuStacked => "GPU passes (stacked)",
            ProfilerMetric::GpuTotalUs => "GPU total (μs)",
            ProfilerMetric::CpuStacked => "CPU spans (stacked)",
            ProfilerMetric::CpuTotalMs => "CPU total (ms)",
            ProfilerMetric::UploadWrittenKiB => "Upload written (KiB)",
            ProfilerMetric::UploadTotalKiB => "Upload total (KiB)",
            ProfilerMetric::RenderMs => "Render (ms)",
            ProfilerMetric::EvalMs => "Eval (ms)",
        }
    }

    fn value(self, s: &ProgressSample) -> f64 {
        match self {
            ProfilerMetric::GpuStacked | ProfilerMetric::GpuTotalUs => s.gpu_total_us,
            ProfilerMetric::CpuStacked | ProfilerMetric::CpuTotalMs => s.cpu_total_ms,
            ProfilerMetric::UploadWrittenKiB => s.upload_written_kib,
            ProfilerMetric::UploadTotalKiB => s.upload_total_kib,
            ProfilerMetric::RenderMs => s.render_ms,
            ProfilerMetric::EvalMs => s.eval_ms,
        }
    }

    fn unit(self) -> &'static str {
        match self {
            ProfilerMetric::GpuStacked | ProfilerMetric::GpuTotalUs => "μs",
            ProfilerMetric::CpuStacked | ProfilerMetric::CpuTotalMs => "ms",
            ProfilerMetric::UploadWrittenKiB | ProfilerMetric::UploadTotalKiB => "KiB",
            ProfilerMetric::RenderMs | ProfilerMetric::EvalMs => "ms",
        }
    }

    /// The stacked span list this metric draws, if any.
    fn stacked_spans(self, s: &ProgressSample) -> Option<&[(String, f64)]> {
        match self {
            ProfilerMetric::GpuStacked => (!s.gpu_passes.is_empty()).then_some(&s.gpu_passes),
            ProfilerMetric::CpuStacked => (!s.cpu_spans.is_empty()).then_some(&s.cpu_spans),
            _ => None,
        }
    }
}

/// Record the just-rendered frame into the progress-sample bucket at the
/// current timeline position. Called from `render_animation`.
pub(crate) fn record_sample(app: &mut RanimPreviewApp) {
    let total_sec = app.timeline_state.total_sec;
    if total_sec <= 0.0 {
        return;
    }
    let bucket_count = (total_sec * super::DEFAULT_LOGIC_FPS).ceil() as usize;
    if app.progress_total_sec != total_sec || app.progress_samples.len() != bucket_count {
        app.progress_samples = vec![None; bucket_count];
        app.progress_total_sec = total_sec;
    }
    let idx = ((app.timeline_state.current_sec / total_sec) * bucket_count as f64) as usize;
    let Some(slot) = app.progress_samples.get_mut(idx.min(bucket_count - 1)) else {
        return;
    };
    *slot = Some(ProgressSample {
        gpu_total_us: app.gpu_pass_times.iter().map(|&(_, t)| t).sum(),
        gpu_passes: app.gpu_pass_times.clone(),
        cpu_total_ms: app.cpu_spans.iter().map(|&(_, t)| t).sum(),
        cpu_spans: app.cpu_spans.clone(),
        render_ms: app
            .last_render_time
            .map(|d| d.as_secs_f64() * 1e3)
            .unwrap_or(0.0),
        eval_ms: app
            .last_eval_time
            .map(|d| d.as_secs_f64() * 1e3)
            .unwrap_or(0.0),
        upload_total_kib: app
            .upload_stats
            .values()
            .map(|s| s.bytes as f64 / 1024.0)
            .sum(),
        upload_written_kib: app
            .upload_stats
            .values()
            .map(|s| s.written_bytes as f64 / 1024.0)
            .sum(),
    });
}

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
        .default_size([460.0, 560.0])
        .show(ctx, |ui| {
            ui_summary(app, ui);
            ui_progress_chart(app, ui);
            ui_cpu_spans(app, ui);
            ui_gpu_passes(app, ui);
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
        if !app.gpu_pass_times.is_empty() {
            let total: f64 = app.gpu_pass_times.iter().map(|&(_, t)| t).sum();
            ui.strong(format!("GPU total: {total:.0} μs"));
        }
        if !app.cpu_spans.is_empty() {
            let total: f64 = app.cpu_spans.iter().map(|&(_, t)| t).sum();
            ui.strong(format!("CPU total: {total:.2} ms"));
        }
    });
}

/// X axis = scene progress; click/drag to seek, hover for the sample values.
fn ui_progress_chart(app: &mut RanimPreviewApp, ui: &mut egui::Ui) {
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        ui.label("metric:");
        let metric = app.profiler_metric;
        egui::ComboBox::from_id_salt("profiler_metric")
            .selected_text(metric.label())
            .show_ui(ui, |ui| {
                for m in [
                    ProfilerMetric::GpuStacked,
                    ProfilerMetric::GpuTotalUs,
                    ProfilerMetric::CpuStacked,
                    ProfilerMetric::CpuTotalMs,
                    ProfilerMetric::UploadWrittenKiB,
                    ProfilerMetric::UploadTotalKiB,
                    ProfilerMetric::RenderMs,
                    ProfilerMetric::EvalMs,
                ] {
                    ui.selectable_value(&mut app.profiler_metric, m, m.label());
                }
            });
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.weak("click / drag to seek");
        });
    });

    let height = 96.0;
    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), height),
        egui::Sense::click_and_drag(),
    );
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 2.0, ui.visuals().faint_bg_color);

    let total_sec = app.timeline_state.total_sec.max(1e-9);
    let samples = &app.progress_samples;
    let n = samples.len();
    if n == 0 {
        painter.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            "no samples yet — play or scrub the timeline",
            egui::FontId::proportional(12.0),
            ui.visuals().weak_text_color(),
        );
        return;
    }

    let metric = app.profiler_metric;
    let y_max = samples
        .iter()
        .flatten()
        .map(|s| metric.value(s))
        .fold(0.0f64, f64::max)
        .max(1e-9);

    // Pass/span order and colors come from the latest sample (stable set).
    let stacked_labels: Vec<String> = match metric {
        ProfilerMetric::GpuStacked => app.gpu_pass_times.iter().map(|(l, _)| l.clone()).collect(),
        ProfilerMetric::CpuStacked => app.cpu_spans.iter().map(|(l, _)| l.clone()).collect(),
        _ => Vec::new(),
    };

    for (i, slot) in samples.iter().enumerate() {
        let Some(s) = slot else { continue };
        let x0 = egui::lerp(rect.left()..=rect.right(), i as f32 / n as f32);
        let x1 = egui::lerp(rect.left()..=rect.right(), (i + 1) as f32 / n as f32);
        let w = (x1 - x0).max(1.0);
        if let Some(spans) = metric.stacked_spans(s) {
            let mut y_base = rect.bottom();
            for (label, v) in spans {
                let h = ((v / y_max) * rect.height() as f64) as f32;
                let h = h.min(rect.height());
                let seg = egui::Rect::from_min_max(
                    egui::pos2(x0, (y_base - h).max(rect.top())),
                    egui::pos2(x0 + w, y_base),
                );
                painter.rect_filled(seg, 0.0, pass_color(&stacked_labels, label));
                y_base = (y_base - h).max(rect.top());
            }
        } else {
            let h = ((metric.value(s) / y_max) * rect.height() as f64) as f32;
            let h = h.clamp(0.0, rect.height());
            painter.rect_filled(
                egui::Rect::from_min_max(
                    egui::pos2(x0, rect.bottom() - h),
                    egui::pos2(x0 + w, rect.bottom()),
                ),
                0.0,
                ui.visuals().selection.bg_fill,
            );
        }
    }

    // Playhead at the current timeline position.
    let ph_x = egui::lerp(
        rect.left()..=rect.right(),
        (app.timeline_state.current_sec / total_sec) as f32,
    );
    painter.line_segment(
        [
            egui::pos2(ph_x, rect.top()),
            egui::pos2(ph_x, rect.bottom()),
        ],
        egui::Stroke::new(1.5, ui.visuals().strong_text_color()),
    );

    // Axis annotations.
    painter.text(
        rect.left_top() + egui::vec2(4.0, 2.0),
        egui::Align2::LEFT_TOP,
        format!("max {:.0} {}", y_max, metric.unit()),
        egui::FontId::proportional(10.0),
        ui.visuals().weak_text_color(),
    );
    painter.text(
        rect.left_bottom() + egui::vec2(4.0, -2.0),
        egui::Align2::LEFT_BOTTOM,
        "0s",
        egui::FontId::proportional(10.0),
        ui.visuals().weak_text_color(),
    );
    painter.text(
        rect.right_bottom() + egui::vec2(-4.0, -2.0),
        egui::Align2::RIGHT_BOTTOM,
        format!("{total_sec:.1}s"),
        egui::FontId::proportional(10.0),
        ui.visuals().weak_text_color(),
    );

    // Click / drag to seek (same semantics as the timeline slider).
    if (response.clicked() || response.dragged())
        && let Some(pos) = response.interact_pointer_pos()
        && let Some(sec) = x_to_sec(rect, pos.x, total_sec)
    {
        app.timeline_state.current_sec = sec.clamp(0.0, app.timeline_state.total_sec);
        app.need_eval = true;
    }

    // Hover tooltip with the sample under the pointer.
    if let Some(pos) = response.hover_pos()
        && let Some(sec) = x_to_sec(rect, pos.x, total_sec)
        && let Some(Some(s)) = samples.get((((sec / total_sec) * n as f64) as usize).min(n - 1))
    {
        let mut text = format!(
            "t = {:.3}s\nGPU total: {:.0} μs | CPU total: {:.2} ms\nrender {:.2} ms | eval {:.2} ms\nupload: {:.1} KiB written / {:.1} KiB total",
            sec,
            s.gpu_total_us,
            s.cpu_total_ms,
            s.render_ms,
            s.eval_ms,
            s.upload_written_kib,
            s.upload_total_kib
        );
        for (label, us) in s.gpu_passes.iter().take(4) {
            text.push_str(&format!("\n  {label}: {us:.0} μs"));
        }
        for (label, ms) in s.cpu_spans.iter().take(4) {
            text.push_str(&format!("\n  {label}: {ms:.2} ms"));
        }
        response.on_hover_text(text);
    }
}

fn x_to_sec(rect: egui::Rect, x: f32, total_sec: f64) -> Option<f64> {
    if x < rect.left() {
        return None;
    }
    let x = x.min(rect.right());
    let t = (x - rect.left()) / rect.width();
    Some((t as f64).clamp(0.0, 1.0) * total_sec)
}

/// Fixed palette for per-pass stacked bars, keyed by pass-label order.
const PASS_PALETTE: [egui::Color32; 8] = [
    egui::Color32::from_rgb(0x4C, 0x78, 0xE8), // blue
    egui::Color32::from_rgb(0xF2, 0x8E, 0x2B), // orange
    egui::Color32::from_rgb(0x54, 0xA2, 0x4F), // green
    egui::Color32::from_rgb(0xE6, 0x4F, 0x59), // red
    egui::Color32::from_rgb(0xB0, 0x79, 0xD1), // purple
    egui::Color32::from_rgb(0xDD, 0xCA, 0x3A), // yellow
    egui::Color32::from_rgb(0x76, 0xB7, 0xB2), // teal
    egui::Color32::from_rgb(0xFF, 0x9D, 0xA7), // pink
];

fn pass_color(pass_labels: &[String], label: &str) -> egui::Color32 {
    let idx = pass_labels
        .iter()
        .position(|l| l == label)
        .unwrap_or(usize::MAX);
    PASS_PALETTE[idx % PASS_PALETTE.len()]
}

/// Legend for the stacked pass/span colors.
fn ui_stacked_legend(spans: &[(String, f64)], ui: &mut egui::Ui) {
    if spans.is_empty() {
        return;
    }
    let labels: Vec<String> = spans.iter().map(|(l, _)| l.clone()).collect();
    ui.horizontal_wrapped(|ui| {
        for (label, _) in spans {
            let color = pass_color(&labels, label);
            let (rect, _) = ui.allocate_exact_size(egui::vec2(8.0, 8.0), egui::Sense::hover());
            ui.painter().rect_filled(rect, 1.0, color);
            ui.label(egui::RichText::new(label).small());
        }
    });
}

fn ui_cpu_spans(app: &mut RanimPreviewApp, ui: &mut egui::Ui) {
    ui.add_space(4.0);
    if app.profiler_metric == ProfilerMetric::CpuStacked {
        ui_stacked_legend(&app.cpu_spans, ui);
    }
    ui.heading("CPU spans");
    if app.cpu_spans.is_empty() {
        ui.label(egui::RichText::new("no spans recorded yet — play the animation").weak());
        return;
    }
    let total: f64 = app.cpu_spans.iter().map(|&(_, t)| t).sum();
    egui::Grid::new("cpu_span_grid")
        .num_columns(3)
        .striped(true)
        .show(ui, |ui| {
            ui.strong("span");
            ui.strong("time");
            ui.strong("share");
            ui.end_row();
            for (label, ms) in &app.cpu_spans {
                ui.label(label);
                ui.label(format!("{ms:.2} ms"));
                let share = if total > 0.0 { *ms / total } else { 0.0 };
                share_bar(ui, share as f32);
                ui.end_row();
            }
            ui.strong("total");
            ui.strong(format!("{total:.2} ms"));
            ui.label("");
            ui.end_row();
        });
}

fn ui_gpu_passes(app: &mut RanimPreviewApp, ui: &mut egui::Ui) {
    ui.add_space(4.0);
    if app.profiler_metric == ProfilerMetric::GpuStacked {
        ui_stacked_legend(&app.gpu_pass_times, ui);
    }
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
    let total: f64 = app.gpu_pass_times.iter().map(|&(_, t)| t).sum();
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
