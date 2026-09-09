//! GPU upload profiling: quantifies the per-frame full-buffer upload cost of
//! the merged VItem/MeshItem buffers and A/B-tests experimental upload
//! strategies implemented in `ranim_render::upload_probe`.
//!
//! The upload strategy is selected process-wide via `RANIM_PROFILE_UPLOAD`:
//!
//! ```bash
//! RANIM_PROFILE_UPLOAD=count        cargo run -p benches --bin profile_gpu_upload --release
//! RANIM_PROFILE_UPLOAD=skip-equal   cargo run -p benches --bin profile_gpu_upload --release
//! RANIM_PROFILE_UPLOAD=dirty-ranges cargo run -p benches --bin profile_gpu_upload --release
//! ```
//!
//! Scenarios:
//! - `static(n)`: one evaluated frame, re-rendered every frame with identical
//!   content (steady-state / preview-idle case; `eval_at_alpha` is not
//!   idempotent, so the frame is evaluated once and reused).
//! - `static-pan(n)`: same, plus a camera pan applied per frame: items
//!   unchanged, only the viewport uniform changes.
//! - `morph(n)`: squares morphing into circles, evaluated per frame
//!   (worst case: point data changes every frame).
//!
//! Per scenario it reports (averaged per frame):
//! - eval time (frame acquisition, CPU only)
//! - submit time (`Renderer::render_frame`, CPU side, no GPU wait)
//! - full frame time (render + device poll, i.e. GPU-inclusive)
//! - uploaded vs actually-written bytes and CPU time spent in write calls
//!
//! It also prints a content-redundancy analysis: how many bytes of the
//! full upload actually changed between consecutive frames (the upper bound
//! any change-tracking upload scheme could reach), derived from the
//! `CoreItem` payloads without touching the renderer.
//!
//! GPU pass scopes require `--features benches/gpu-scopes`; note that this
//! enables the wgpu profiler, which forces a device poll per frame and
//! inflates the full-frame time.

use std::time::{Duration, Instant};

use benches::test_scenes::{static_squares, transform_squares};
use ranim::{SceneConstructor, prelude::*};
use ranim_core::{SealedRanimScene, core_item::CoreItem};
use ranim_render::{
    Renderer,
    upload_probe::{self, UploadMode},
    utils::WgpuContext,
    world::RenderFrame,
};

const WIDTH: u32 = 1920;
const HEIGHT: u32 = 1080;
const WARMUP_FRAMES: usize = 10;
const MEASURED_FRAMES: usize = 60;

#[derive(Clone, Copy, PartialEq)]
enum ScenarioKind {
    /// One evaluated frame, re-rendered as-is: nothing changes frame to frame.
    Static,
    /// Same, with the camera panned a bit further every frame.
    StaticPan,
    /// Grid of squares morphing into circles: points/attrs change every frame.
    Morph,
}

struct Scenario {
    name: String,
    kind: ScenarioKind,
    n: usize,
}

impl Scenario {
    fn build_scene(&self) -> SealedRanimScene {
        let n = self.n;
        match self.kind {
            ScenarioKind::Static | ScenarioKind::StaticPan => {
                (|r: &mut RanimScene| static_squares(r, n)).build_scene()
            }
            ScenarioKind::Morph => (|r: &mut RanimScene| transform_squares(r, n)).build_scene(),
        }
    }
}

type Frame = Vec<((usize, usize), CoreItem)>;

/// Produces the per-frame `CoreItem` payload for a scenario. `Static` kinds
/// evaluate the scene once and reuse the payload (with camera mutation for
/// `StaticPan`); `Morph` re-evaluates at an advancing alpha.
struct FrameFeed {
    scene: SealedRanimScene,
    kind: ScenarioKind,
    base: Frame,
    frame: usize,
    total: usize,
}

impl FrameFeed {
    fn new(scenario: &Scenario, total: usize) -> Self {
        let scene = scenario.build_scene();
        let base = match scenario.kind {
            // End state: all staggered show()s complete.
            ScenarioKind::Static | ScenarioKind::StaticPan => scene.eval_at_alpha(1.0).collect(),
            ScenarioKind::Morph => Vec::new(),
        };
        Self {
            scene,
            kind: scenario.kind,
            base,
            frame: 0,
            total,
        }
    }

    /// (frame payload, was-evaluated): eval is only measured when it happens.
    fn next(&mut self) -> (Frame, bool) {
        let frame = self.frame;
        self.frame += 1;
        match self.kind {
            ScenarioKind::Static => (self.base.clone(), false),
            ScenarioKind::StaticPan => {
                let mut items = self.base.clone();
                let t = frame as f64 / self.total as f64;
                for (_, item) in &mut items {
                    if let CoreItem::CameraFrame(cam) = item {
                        cam.pos.x += t * 0.8;
                        cam.pos.y += t * 0.4;
                    }
                }
                (items, false)
            }
            ScenarioKind::Morph => {
                // t in [0.1, 0.6): inside the morph window.
                let alpha = 0.1 + 0.5 * (frame as f64 / self.total as f64);
                (self.scene.eval_at_alpha(alpha).collect(), true)
            }
        }
    }
}

struct FrameTimings {
    eval: Duration,
    submit: Duration,
    full: Duration,
}

fn run_scenario(ctx: &WgpuContext, scenario: &Scenario) {
    let total = WARMUP_FRAMES + MEASURED_FRAMES;
    let mut feed = FrameFeed::new(scenario, total);
    let mut renderer = Renderer::new(ctx, WIDTH, HEIGHT, 8);
    let mut render_textures = renderer.new_render_textures(ctx);
    let mut store = RenderFrame::new();
    let clear_color = wgpu::Color::BLACK;

    let mut timings = Vec::with_capacity(MEASURED_FRAMES);
    for frame in 0..total {
        if frame == WARMUP_FRAMES {
            // Drop counters accumulated during warmup (reallocs etc.).
            let _ = upload_probe::take_stats();
        }
        let (items, evaluated) = feed.next();
        let t0 = Instant::now();
        store.update(items.into_iter());
        let t1 = Instant::now();
        renderer.render_frame(&mut render_textures, clear_color, &store);
        let t2 = Instant::now();
        ctx.device
            .poll(wgpu::PollType::wait_indefinitely())
            .unwrap();
        let t3 = Instant::now();
        if frame >= WARMUP_FRAMES {
            timings.push(FrameTimings {
                // Only count eval when it actually happened (morph); the
                // static clone is bookkeeping, reported as ~0.
                eval: if evaluated { t1 - t0 } else { Duration::ZERO },
                submit: t2 - t1,
                full: t3 - t2,
            });
        }
    }

    let stats = upload_probe::take_stats();
    let frames = MEASURED_FRAMES as f64;
    let frames_f = timings.len().max(1) as f64;
    let avg = |f: fn(&FrameTimings) -> Duration| {
        timings.iter().map(f).sum::<Duration>() / frames_f as u32
    };

    let mut total_bytes = 0u64;
    let mut total_written = 0u64;
    let mut total_cpu = Duration::ZERO;
    let mut total_diff = Duration::ZERO;
    println!(
        "\n=== {} (upload mode: {:?}) ===",
        scenario.name,
        upload_probe::mode()
    );
    println!(
        "{:<28} {:>8} {:>10} {:>9} {:>9} {:>10}",
        "buffer", "calls/f", "bytes/f", "KiB/f", "written", "cpu μs/f"
    );
    for (label, s) in &stats {
        total_bytes += s.bytes;
        total_written += s.written_bytes;
        total_cpu += s.cpu_time;
        total_diff += s.diff_time;
        let written_pct = if s.bytes > 0 {
            100.0 * s.written_bytes as f64 / s.bytes as f64
        } else {
            0.0
        };
        println!(
            "{:<28} {:>8.1} {:>10.0} {:>9.1} {:>8.1}% {:>10.1}",
            label,
            s.calls as f64 / frames,
            s.bytes as f64 / frames,
            s.bytes as f64 / frames / 1024.0,
            written_pct,
            s.cpu_time.as_secs_f64() * 1e6 / frames,
        );
    }
    let total_written_pct = if total_bytes > 0 {
        100.0 * total_written as f64 / total_bytes as f64
    } else {
        0.0
    };
    println!(
        "{:<28} {:>8} {:>10.0} {:>9.1} {:>8.1}% {:>10.1}",
        "TOTAL",
        "",
        total_bytes as f64 / frames,
        total_bytes as f64 / frames / 1024.0,
        total_written_pct,
        total_cpu.as_secs_f64() * 1e6 / frames,
    );
    println!(
        "eval avg {:>7.2} ms | submit avg {:>7.2} ms | full avg {:>7.2} ms (max {:>7.2} ms) | upload cpu {:>6.2} ms | diff cpu {:>6.2} ms",
        avg(|t| t.eval).as_secs_f64() * 1e3,
        avg(|t| t.submit).as_secs_f64() * 1e3,
        avg(|t| t.full).as_secs_f64() * 1e3,
        timings
            .iter()
            .map(|t| t.full)
            .max()
            .unwrap_or_default()
            .as_secs_f64()
            * 1e3,
        total_cpu.as_secs_f64() * 1e3 / frames,
        total_diff.as_secs_f64() * 1e3 / frames,
    );

    #[cfg(feature = "gpu-scopes")]
    if let Some(scopes) = renderer.take_last_gpu_scopes() {
        println!("--- GPU pass scopes (last processed frame) ---");
        ranim_render::profiling_utils::scopes_to_console_recursive(&scopes, 0);
    }
    #[cfg(not(feature = "gpu-scopes"))]
    let _ = &renderer;
}

/// How many bytes of the per-frame full upload actually differ between
/// consecutive frames, derived from the evaluated `CoreItem` payloads.
/// This is the upper bound any change-tracking upload scheme could reach.
fn redundancy_analysis(scenario: &Scenario) {
    let total = MEASURED_FRAMES;
    let mut feed = FrameFeed::new(scenario, total);
    let mut prev = feed.next().0;
    let mut full_bytes = 0u64;
    let mut changed_bytes = 0u64;
    let mut items_changed = 0usize;
    let mut items_total = 0usize;
    let (mut pts_d, mut fill_d, mut stroke_d, mut xform_d) = (0usize, 0usize, 0usize, 0usize);

    for _ in 1..total {
        let curr = feed.next().0;
        items_total = curr.len();
        for (id, item) in &curr {
            let prev_item = prev.iter().find(|(pid, _)| *pid == *id).map(|(_, p)| p);
            let (full, changed) = model_item_bytes(prev_item, item);
            full_bytes += full;
            if changed > 20 {
                // more than the unavoidable clip_boxes re-init
                items_changed += 1;
            }
            changed_bytes += changed;
            if let (Some(CoreItem::VItem(p)), CoreItem::VItem(v)) = (prev_item, item) {
                pts_d += usize::from(v.points != p.points);
                fill_d += usize::from(v.fill_rgbas != p.fill_rgbas);
                stroke_d += usize::from(
                    v.stroke_rgbas != p.stroke_rgbas || v.stroke_widths != p.stroke_widths,
                );
                xform_d += usize::from(v.transform != p.transform);
            }
        }
        prev = curr;
    }
    println!(
        "    field dirty counts/frame: points {pts_d:.0}, fills {fill_d:.0}, strokes {stroke_d:.0}, transforms {xform_d:.0}"
    );

    let frames = (total - 1) as f64;
    println!(
        "\n--- redundancy: {} ---\nitems/frame: {items_total}, items changed/frame: {:.1} ({:.1}%), full upload: {:.1} KiB/frame, minimal upload: {:.1} KiB/frame ({:.1}% redundant)",
        scenario.name,
        items_changed as f64 / frames,
        100.0 * items_changed as f64 / frames / items_total.max(1) as f64,
        full_bytes as f64 / frames / 1024.0,
        changed_bytes as f64 / frames / 1024.0,
        100.0 * (1.0 - changed_bytes as f64 / full_bytes.max(1) as f64),
    );
}

/// (full bytes, changed bytes) for one item against its previous version.
/// Buffer classes per VItem (see `VItemsBuffer::update`):
/// item_infos 16B, planes 32B, transforms 64B, clip_boxes 20B (re-init),
/// points3d 16B/pt, points2d 16B/pt (zero-init), fills/strokes 16B/attr,
/// widths 4B/attr (attrs = ceil(points/2)). Camera -> viewport uniform
/// (small, always re-uploaded) is ignored here.
fn model_item_bytes(prev: Option<&CoreItem>, curr: &CoreItem) -> (u64, u64) {
    let CoreItem::VItem(v) = curr else {
        return (0, 0);
    };
    let pts = v.points.len() as u64;
    let attrs = pts.div_ceil(2);
    let full = 16 + 32 + 64 + 20 + 16 * pts + 16 * pts + 16 * attrs + 16 * attrs + 4 * attrs;

    let Some(CoreItem::VItem(p)) = prev else {
        return (full, full); // newly appeared item
    };
    let points_dirty = v.points != p.points;
    let fill_dirty = v.fill_rgbas != p.fill_rgbas;
    let stroke_dirty = v.stroke_rgbas != p.stroke_rgbas || v.stroke_widths != p.stroke_widths;
    let transform_dirty = v.transform != p.transform;
    let normal_dirty = v.normal != p.normal;
    let mut changed = 0u64;
    if points_dirty {
        changed += 16 * pts; // points3d
    }
    if points_dirty || normal_dirty {
        changed += 32; // planes (origin = points[0], normal)
    }
    if transform_dirty {
        changed += 64; // transforms
    }
    changed += 20; // clip_boxes must be re-initialized every frame
    if fill_dirty {
        changed += 16 * attrs;
    }
    if stroke_dirty {
        changed += 16 * attrs + 4 * attrs;
    }
    // item_infos: constant unless topology changes; points2d: zero-init only.
    (full, changed)
}

fn main() {
    println!("profile_gpu_upload v2 (frame-feed)");
    match upload_probe::mode() {
        UploadMode::Off => {
            eprintln!(
                "set RANIM_PROFILE_UPLOAD=count|skip-equal|dirty-ranges to enable upload profiling"
            );
            std::process::exit(2);
        }
        mode => println!("upload mode: {mode:?}"),
    }
    #[cfg(feature = "gpu-scopes")]
    println!(
        "WARNING: gpu-scopes feature enabled — per-frame device poll inflates full-frame timings"
    );

    let scenarios = [
        Scenario {
            name: format!("static({})", 20),
            kind: ScenarioKind::Static,
            n: 20,
        },
        Scenario {
            name: format!("static({})", 40),
            kind: ScenarioKind::Static,
            n: 40,
        },
        Scenario {
            name: format!("static({})", 60),
            kind: ScenarioKind::Static,
            n: 60,
        },
        Scenario {
            name: format!("static-pan({})", 40),
            kind: ScenarioKind::StaticPan,
            n: 40,
        },
        Scenario {
            name: format!("morph({})", 20),
            kind: ScenarioKind::Morph,
            n: 20,
        },
        Scenario {
            name: format!("morph({})", 40),
            kind: ScenarioKind::Morph,
            n: 40,
        },
    ];

    // Content redundancy is independent of the upload mode; run it once.
    for s in &scenarios {
        redundancy_analysis(s);
    }

    let ctx = pollster::block_on(WgpuContext::new());
    for s in &scenarios {
        run_scenario(&ctx, s);
    }
}
