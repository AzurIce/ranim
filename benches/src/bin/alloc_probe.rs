//! Per-frame allocation profile for ranim's eval/extract chain and the render
//! pipeline.
//!
//! Installs a counting global allocator and sweeps scene scale, reporting
//! allocations, frees, bytes, transient live peak and wall time per frame,
//! plus a power-of-two size-class histogram. Writes a JSON report for
//! charting:
//!
//! ```text
//! cargo run -p benches --bin alloc_probe --release -- --json alloc-report/results.json
//! ```

use std::{
    alloc::{GlobalAlloc, Layout, System},
    sync::atomic::{AtomicI64, AtomicU64, Ordering},
    time::{Duration, Instant},
};

use benches::test_scenes::{static_squares, transform_squares};
use ranim::{
    SceneConstructor,
    core::Extract,
    items::vitem::geometry::{Circle, Square},
    prelude::*,
};
use ranim_render::{Renderer, utils::WgpuContext, world::RenderFrame};

const NB: usize = 20;

#[global_allocator]
static ALLOC: CountingAlloc = CountingAlloc;

static ALLOC_COUNT: AtomicU64 = AtomicU64::new(0);
static FREE_COUNT: AtomicU64 = AtomicU64::new(0);
static ALLOC_BYTES: AtomicU64 = AtomicU64::new(0);
static LIVE_BYTES: AtomicI64 = AtomicI64::new(0);
static PEAK_LIVE: AtomicU64 = AtomicU64::new(0);
static HIST: [AtomicU64; NB] = [const { AtomicU64::new(0) }; NB];

struct CountingAlloc;

fn bucket(size: usize) -> usize {
    (usize::BITS - size.max(1).leading_zeros()).clamp(1, NB as u32) as usize - 1
}

fn count_alloc(layout: Layout) {
    if layout.size() == 0 {
        return;
    }
    ALLOC_COUNT.fetch_add(1, Ordering::Relaxed);
    ALLOC_BYTES.fetch_add(layout.size() as u64, Ordering::Relaxed);
    LIVE_BYTES.fetch_add(layout.size() as i64, Ordering::Relaxed);
    HIST[bucket(layout.size())].fetch_add(1, Ordering::Relaxed);
}

fn count_dealloc(layout: Layout) {
    if layout.size() == 0 {
        return;
    }
    FREE_COUNT.fetch_add(1, Ordering::Relaxed);
    LIVE_BYTES.fetch_sub(layout.size() as i64, Ordering::Relaxed);
}

unsafe impl GlobalAlloc for CountingAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ptr = unsafe { System.alloc(layout) };
        if !ptr.is_null() {
            count_alloc(layout);
        }
        ptr
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let ptr = unsafe { System.alloc_zeroed(layout) };
        if !ptr.is_null() {
            count_alloc(layout);
        }
        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        count_dealloc(layout);
        unsafe { System.dealloc(ptr, layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, old_layout: Layout, new_size: usize) -> *mut u8 {
        let moved = unsafe { System.realloc(ptr, old_layout, new_size) };
        if !moved.is_null() {
            count_dealloc(old_layout);
            unsafe {
                count_alloc(Layout::from_size_align_unchecked(
                    new_size,
                    old_layout.align(),
                ));
            }
        }
        moved
    }
}

struct Snap {
    iters: u64,
    allocs: u64,
    frees: u64,
    bytes: u64,
    live_delta: i64,
    peak_live: i64,
    hist: Vec<u64>,
    elapsed: Duration,
}

impl Snap {
    fn per_frame(&self, v: u64) -> f64 {
        v as f64 / self.iters as f64
    }

    fn us_per_frame(&self) -> f64 {
        self.elapsed.as_secs_f64() * 1e6 / self.iters as f64
    }

    fn reset_counters() {
        ALLOC_COUNT.store(0, Ordering::Relaxed);
        FREE_COUNT.store(0, Ordering::Relaxed);
        ALLOC_BYTES.store(0, Ordering::Relaxed);
        PEAK_LIVE.store(0, Ordering::Relaxed);
        for h in HIST.iter() {
            h.store(0, Ordering::Relaxed);
        }
    }

    fn measure(iters: u32, mut f: impl FnMut()) -> Snap {
        f(); // warmup
        Self::reset_counters();
        let live0 = LIVE_BYTES.load(Ordering::Relaxed);
        let start = Instant::now();
        for _ in 0..iters {
            f();
            let cur = LIVE_BYTES.load(Ordering::Relaxed) - live0;
            PEAK_LIVE.fetch_max(cur.max(0) as u64, Ordering::Relaxed);
        }
        let elapsed = start.elapsed();
        Snap {
            iters: iters as u64,
            allocs: ALLOC_COUNT.swap(0, Ordering::Relaxed),
            frees: FREE_COUNT.swap(0, Ordering::Relaxed),
            bytes: ALLOC_BYTES.swap(0, Ordering::Relaxed),
            live_delta: LIVE_BYTES.load(Ordering::Relaxed) - live0,
            peak_live: PEAK_LIVE.swap(0, Ordering::Relaxed) as i64,
            hist: HIST.iter().map(|h| h.swap(0, Ordering::Relaxed)).collect(),
            elapsed,
        }
    }
}

const CPU_ITERS: u32 = 30;

fn profile_eval_phase(
    scene: &str,
    grid: usize,
    timeline: &ranim_core::SealedRanimScene,
    json: &mut Vec<String>,
) {
    let output_items = timeline.eval_at_alpha(0.5).count();

    let eval = Snap::measure(CPU_ITERS, || {
        std::hint::black_box(timeline.eval_at_alpha(0.5).collect::<Vec<_>>());
    });
    report(json, "cpu", &[("eval", scene, grid, output_items, &eval)]);
    println!(
        "{scene:>18} grid={grid:>3} items={output_items:>5}  eval: allocs={:>9.0} frees={:>9.0} bytes={:>10.0} peak={:>8.0} net={:>5.0}  {:>8.1} µs/frame",
        eval.per_frame(eval.allocs),
        eval.per_frame(eval.frees),
        eval.per_frame(eval.bytes),
        eval.per_frame(eval.peak_live as u64),
        eval.per_frame(eval.live_delta.unsigned_abs()),
        eval.us_per_frame(),
    );

    // eval + CPU-side render store refresh (RenderFrame clear+extend).
    let mut store = RenderFrame::new();
    let upd = Snap::measure(CPU_ITERS, || {
        store.update(timeline.eval_at_alpha(0.5));
    });
    report(
        json,
        "cpu",
        &[("eval+update", scene, grid, output_items, &upd)],
    );
    println!(
        "{scene:>18} grid={grid:>3} items={output_items:>5}  eval+update: allocs={:>9.0} bytes={:>10.0}  {:>8.1} µs/frame",
        upd.per_frame(upd.allocs),
        upd.per_frame(upd.bytes),
        upd.us_per_frame(),
    );
}

fn gpu_section(json: &mut Vec<String>) {
    println!("-- gpu --");
    let ctx = match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        pollster::block_on(WgpuContext::new())
    })) {
        Ok(ctx) => ctx,
        Err(_) => {
            println!("no wgpu adapter, skipping gpu section");
            return;
        }
    };
    let mut renderer = Renderer::new(&ctx, 1920, 1080, 8);
    let mut textures = renderer.new_render_textures(&ctx);
    let clear_color = wgpu::Color::BLACK;

    let grid = 32;
    let timeline = (|r: &mut RanimScene| static_squares(r, grid)).build_scene();
    let mut store = RenderFrame::new();
    store.update(timeline.eval_at_alpha(0.5));
    renderer.render_frame(&mut textures, clear_color, &store);
    ctx.device
        .poll(wgpu::PollType::wait_indefinitely())
        .unwrap();

    let steady = Snap::measure(20, || {
        renderer.render_frame(&mut textures, clear_color, &store);
    });
    let output_items = grid * grid;
    report(
        json,
        "gpu",
        &[(
            "render steady",
            "static_squares",
            grid,
            output_items,
            &steady,
        )],
    );
    println!(
        "static render (grid={grid}): allocs={:>7.1} bytes={:>8.0}  {:>8.1} µs/frame",
        steady.per_frame(steady.allocs),
        steady.per_frame(steady.bytes),
        steady.us_per_frame(),
    );

    let timeline = (|r: &mut RanimScene| transform_squares(r, grid)).build_scene();
    let mut store = RenderFrame::new();
    let frames = 30.0;
    let mut alpha = 0.0f64;
    let animated = Snap::measure(30, || {
        store.update(timeline.eval_at_alpha(alpha));
        alpha += 1.0 / frames;
        renderer.render_frame(&mut textures, clear_color, &store);
    });
    ctx.device
        .poll(wgpu::PollType::wait_indefinitely())
        .unwrap();
    report(
        json,
        "gpu",
        &[(
            "eval+update+render",
            "transform_squares",
            grid,
            output_items,
            &animated,
        )],
    );
    println!(
        "animated pipeline (grid={grid}): allocs={:>7.1} bytes={:>8.0}  {:>8.1} µs/frame",
        animated.per_frame(animated.allocs),
        animated.per_frame(animated.bytes),
        animated.us_per_frame(),
    );
}

fn micro_section(json: &mut Vec<String>) {
    println!("-- micro --");
    let square = Square::new(2.0);
    let circle = Circle::new(2.0);
    #[allow(clippy::type_complexity)] // three local closures over Copy geometry items
    let cases: [(&str, Box<dyn Fn()>); 3] = [
        (
            "square.clone",
            Box::new(|| {
                std::hint::black_box(square.clone());
            }),
        ),
        (
            "square.extract",
            Box::new(|| {
                std::hint::black_box(square.extract());
            }),
        ),
        (
            "circle.extract",
            Box::new(|| {
                std::hint::black_box(circle.extract());
            }),
        ),
    ];
    for (name, f) in cases {
        let snap = Snap::measure(1000, f);
        report(json, "micro", &[(name, "-", 0, 1, &snap)]);
        println!(
            "{name:>16}: allocs={:>4.1} bytes={:>6.0} peak={:>5.0}  {:>6.2} µs/op",
            snap.per_frame(snap.allocs),
            snap.per_frame(snap.bytes),
            snap.per_frame(snap.peak_live as u64),
            snap.us_per_frame(),
        );
    }
}

fn report(json: &mut Vec<String>, section: &str, rows: &[(&str, &str, usize, usize, &Snap)]) {
    for (phase, scene, grid, items, snap) in rows {
        let hist = snap
            .hist
            .iter()
            .map(|c| c.to_string())
            .collect::<Vec<_>>()
            .join(",");
        json.push(format!(
            "{{\"section\":\"{section}\",\"phase\":\"{phase}\",\"scene\":\"{scene}\",\"grid\":{grid},\"items\":{items},\
             \"iters\":{},\"allocs_per_frame\":{:.2},\"frees_per_frame\":{:.2},\"bytes_per_frame\":{:.1},\
             \"live_delta\":{},\"peak_live_per_frame\":{},\"us_per_frame\":{:.3},\"hist\":[{hist}]}}",
            snap.iters,
            snap.per_frame(snap.allocs),
            snap.per_frame(snap.frees),
            snap.per_frame(snap.bytes),
            snap.live_delta,
            snap.peak_live,
            snap.us_per_frame(),
        ));
    }
}

fn main() {
    let mut json_path = None;
    let mut cpu_only = false;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--json" => json_path = args.next(),
            "--cpu-only" => cpu_only = true,
            other => panic!("unknown arg: {other}"),
        }
    }

    let mut json = Vec::new();
    let grids = [1usize, 3, 10, 20, 40];

    println!("-- cpu eval chain (items = grid² squares) --");
    let empty = (|_: &mut RanimScene| {}).build_scene();
    profile_eval_phase("empty", 0, &empty, &mut json);
    for grid in grids {
        let timeline = (|r: &mut RanimScene| static_squares(r, grid)).build_scene();
        profile_eval_phase("static_squares", grid, &timeline, &mut json);
    }
    for grid in grids {
        let timeline = (|r: &mut RanimScene| transform_squares(r, grid)).build_scene();
        profile_eval_phase("transform_squares", grid, &timeline, &mut json);
    }

    micro_section(&mut json);
    if !cpu_only {
        gpu_section(&mut json);
    }

    if let Some(path) = json_path {
        std::fs::write(&path, format!("[\n  {}\n]\n", json.join(",\n  ")))
            .expect("write json report");
        println!("wrote {path}");
    }
}
