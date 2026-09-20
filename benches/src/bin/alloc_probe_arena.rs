//! Arena spike for the per-frame eval/extract allocation churn, written in
//! the **std `allocator_api`** shape: `#![feature(allocator_api)]` +
//! `Vec<T, &Bump>` with `Bump: Allocator` (bumpalo's `allocator_api`
//! feature). This is exactly the code that becomes legal, feature-gate-free
//! once the `allocator_api` stabilization lands — the spike doubles as a
//! preview of that API surface.
//!
//! Decomposes the measured churn (see alloc_probe) into:
//! - S1 "arena only": the *same* per-frame work structure as the real chain
//!   (anim clone -> From rebuild (closepath flags + render points + attr
//!   collects) -> core pushed into the output vec; morph adds the lerp
//!   stage), but every vector is arena-allocated. Isolates allocator
//!   overhead.
//! - S2 "arena + direct": the target shape — one pass writing straight into
//!   the final arena-allocated render arrays (lerp fused, closepath flags
//!   cached at build time: they are structural and invariant under point
//!   lerp).
//!
//! The counting `#[global_allocator]` below is only the *measurement
//! instrument*: it counts malloc events outside the arena. Steady-state
//! arena frames should report 0 allocations because the `Bump` reuses its
//! chunks across `reset()`s.
//!
//! Baseline is the real `timeline.eval_at_alpha(..).collect()` chain.
//! Lives in a gitignored bin; no library code is touched. Requires the
//! pinned nightly (`nix develop`).
//!
//! ```text
//! cargo run -p benches --bin alloc_probe_arena --release -- --json alloc-report/arena_results.json
//! ```

#![feature(allocator_api)]

use std::{
    alloc::{GlobalAlloc, Layout, System},
    hint::black_box,
    sync::atomic::{AtomicI64, AtomicU64, Ordering},
    time::{Duration, Instant},
};

use bumpalo::Bump;
use ranim::{
    SceneConstructor,
    glam::{DVec3, Mat4, dvec3},
    items::vitem::{
        VItem as AnimVItem,
        geometry::{Circle, Square},
    },
    prelude::*,
};
use ranim_core::arena::VItem as ArenaCoreVItem;
use ranim_core::{
    components::{rgba::Rgba, width::Width},
    traits::Alignable,
};

// MARK: counting allocator (measurement instrument only)

#[global_allocator]
static ALLOC: CountingAlloc = CountingAlloc;

static ALLOC_COUNT: AtomicU64 = AtomicU64::new(0);
static ALLOC_BYTES: AtomicU64 = AtomicU64::new(0);
static LIVE_BYTES: AtomicI64 = AtomicI64::new(0);

struct CountingAlloc;

unsafe impl GlobalAlloc for CountingAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ptr = unsafe { System.alloc(layout) };
        if !ptr.is_null() && layout.size() > 0 {
            ALLOC_COUNT.fetch_add(1, Ordering::Relaxed);
            ALLOC_BYTES.fetch_add(layout.size() as u64, Ordering::Relaxed);
            LIVE_BYTES.fetch_add(layout.size() as i64, Ordering::Relaxed);
        }
        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        if layout.size() > 0 {
            LIVE_BYTES.fetch_sub(layout.size() as i64, Ordering::Relaxed);
        }
        unsafe { System.dealloc(ptr, layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, old_layout: Layout, new_size: usize) -> *mut u8 {
        let moved = unsafe { System.realloc(ptr, old_layout, new_size) };
        if !moved.is_null() && new_size > 0 {
            if new_size > old_layout.size() {
                ALLOC_BYTES.fetch_add((new_size - old_layout.size()) as u64, Ordering::Relaxed);
            }
            LIVE_BYTES.fetch_add(
                new_size as i64 - old_layout.size() as i64,
                Ordering::Relaxed,
            );
        }
        moved
    }
}

struct Snap {
    iters: u64,
    allocs: u64,
    bytes: u64,
    live_delta: i64,
    elapsed: Duration,
}

impl Snap {
    fn per_frame(&self, v: u64) -> f64 {
        v as f64 / self.iters as f64
    }

    fn us_per_op(&self) -> f64 {
        self.elapsed.as_secs_f64() * 1e6 / self.iters as f64
    }

    fn measure(iters: u32, mut f: impl FnMut()) -> Snap {
        for _ in 0..3 {
            f(); // warmup (3x so the Bump reaches its chunk equilibrium)
        }
        ALLOC_COUNT.store(0, Ordering::Relaxed);
        ALLOC_BYTES.store(0, Ordering::Relaxed);
        let live0 = LIVE_BYTES.load(Ordering::Relaxed);
        let start = Instant::now();
        for _ in 0..iters {
            f();
        }
        Snap {
            iters: iters as u64,
            allocs: ALLOC_COUNT.swap(0, Ordering::Relaxed),
            bytes: ALLOC_BYTES.swap(0, Ordering::Relaxed),
            live_delta: LIVE_BYTES.load(Ordering::Relaxed) - live0,
            elapsed: start.elapsed(),
        }
    }
}

// MARK: arena-allocated mirrors of the per-frame data
//
// `BVec<'a, T>` is the plain std `Vec<T, &'a Bump>` — no wrapper collection,
// the same type that `Vec::new_in`/`with_capacity_in` produce once
// `allocator_api` stabilizes.

type BVec<'a, T> = std::vec::Vec<T, &'a Bump>;

/// `Vec::from_iter_in` equivalent: reserve from the size hint, then extend.
fn bump_collect<'a, T>(bump: &'a Bump, iter: impl Iterator<Item = T>) -> BVec<'a, T> {
    let (lo, _) = iter.size_hint();
    let mut v = Vec::with_capacity_in(lo, bump);
    v.extend(iter);
    v
}

struct BumpFrame<'a> {
    items: BVec<'a, ArenaCoreVItem<&'a Bump>>,
}

fn lerp_rgba(a: Rgba, b: Rgba, t: f64) -> Rgba {
    Rgba(a.0.lerp(b.0, t as f32))
}

fn lerp_width(a: Width, b: Width, t: f64) -> Width {
    Width(a.0 + (b.0 - a.0) * t as f32)
}

/// Mirrors `self.clone()` on the animation-side VItem.
fn bump_anim_clone<'a>(bump: &'a Bump, src: &AnimVItem) -> BumpAnimVItem<'a> {
    BumpAnimVItem {
        vpoints: bump_collect(bump, src.vpoints.iter().copied()),
        stroke_widths: bump_collect(bump, src.stroke_widths.iter().cloned()),
        stroke_rgbas: bump_collect(bump, src.stroke_rgbas.iter().cloned()),
        fill_rgbas: bump_collect(bump, src.fill_rgbas.iter().cloned()),
    }
}

struct BumpAnimVItem<'a> {
    vpoints: BVec<'a, DVec3>,
    stroke_widths: BVec<'a, Width>,
    stroke_rgbas: BVec<'a, Rgba>,
    fill_rgbas: BVec<'a, Rgba>,
}

/// Mirrors `From<ranim_items::VItem> for core VItem`: a closepath flags vec +
/// a render points collect + 3 attr collects, all arena-allocated.
///
/// Squares/circles are a single closed subpath, so the flag scan collapses to
/// an endpoint comparison — same allocation shape as
/// `VPointVec::get_closepath_flags` (one bool vec), same result here.
fn bump_core_from_anim<'a>(
    bump: &'a Bump,
    anim: &BumpAnimVItem<'a>,
    normal: Option<DVec3>,
) -> ArenaCoreVItem<&'a Bump> {
    let n = anim.vpoints.len();
    let closed = n >= 2 && anim.vpoints[0] == anim.vpoints[n - 1];
    let mut flags = Vec::with_capacity_in(n, bump);
    flags.resize(n, closed);
    ArenaCoreVItem {
        normal: normal.map(|n| n.as_vec3()),
        transform: Mat4::IDENTITY,
        points: bump_collect(
            bump,
            anim.vpoints
                .iter()
                .zip(flags.iter())
                .map(|(p, f)| p.as_vec3().extend(if *f { 1.0 } else { 0.0 })),
        ),
        fill_rgbas: bump_collect(bump, anim.fill_rgbas.iter().cloned()),
        stroke_rgbas: bump_collect(bump, anim.stroke_rgbas.iter().cloned()),
        stroke_widths: bump_collect(bump, anim.stroke_widths.iter().cloned()),
    }
}

/// S1 static: anim clone -> core From -> push into the frame vec.
fn s1_extract<'a>(bump: &'a Bump, src: &AnimVItem, out: &mut BVec<'a, ArenaCoreVItem<&'a Bump>>) {
    let cl = bump_anim_clone(bump, src);
    let core = bump_core_from_anim(bump, &cl, src.normal);
    out.push(core);
}

/// S1 morph: + the lerp stage (anim VItem lerp allocates 4 fresh vectors).
fn s1_extract_lerped<'a>(
    bump: &'a Bump,
    a: &AnimVItem,
    b: &AnimVItem,
    t: f64,
    out: &mut BVec<'a, ArenaCoreVItem<&'a Bump>>,
) {
    let lerped = BumpAnimVItem {
        vpoints: bump_collect(
            bump,
            a.vpoints
                .iter()
                .zip(b.vpoints.iter())
                .map(|(p, q)| p.lerp(q, t)),
        ),
        stroke_widths: bump_collect(
            bump,
            a.stroke_widths
                .iter()
                .zip(b.stroke_widths.iter())
                .map(|(p, q)| lerp_width(*p, *q, t)),
        ),
        stroke_rgbas: bump_collect(
            bump,
            a.stroke_rgbas
                .iter()
                .zip(b.stroke_rgbas.iter())
                .map(|(p, q)| lerp_rgba(*p, *q, t)),
        ),
        fill_rgbas: bump_collect(
            bump,
            a.fill_rgbas
                .iter()
                .zip(b.fill_rgbas.iter())
                .map(|(p, q)| lerp_rgba(*p, *q, t)),
        ),
    };
    let core = bump_core_from_anim(bump, &lerped, if t < 0.5 { a.normal } else { b.normal });
    out.push(core);
}

// MARK: scene sources (real ranim types, built once)

fn build_sources(grid: usize) -> (Vec<AnimVItem>, Vec<(AnimVItem, AnimVItem)>) {
    let buff = 0.1;
    let size = 8.0 / grid as f64;
    let unit = size + buff;
    let start = dvec3(-4.0, -4.0, 0.0);

    let mut statics = Vec::new();
    let mut pairs = Vec::new();
    for i in 0..grid {
        for j in 0..grid {
            let pos = start + unit * DVec3::X * j as f64 + unit * DVec3::Y * i as f64;
            let mut sq = AnimVItem::from(Square::new(size));
            for p in sq.vpoints.0.iter_mut() {
                *p += pos;
            }
            statics.push(sq.clone());

            let circ_size = (8.0 / grid as f64 - buff).max(0.02);
            let mut circle = AnimVItem::from(Circle::new(circ_size / 2.0));
            for p in circle.vpoints.0.iter_mut() {
                *p += pos;
            }
            if !sq.is_aligned(&circle) {
                sq.align_with(&mut circle);
            }
            pairs.push((sq, circle));
        }
    }
    (statics, pairs)
}

// MARK: measurement

fn report(json: &mut Vec<String>, variant: &str, scene: &str, items: usize, s: &Snap) {
    json.push(format!(
        "{{\"variant\":\"{variant}\",\"scene\":\"{scene}\",\"items\":{items},\
         \"iters\":{},\"allocs_per_frame\":{:.2},\"bytes_per_frame\":{:.1},\
         \"live_delta\":{},\"us_per_frame\":{:.3}}}",
        s.iters,
        s.per_frame(s.allocs),
        s.per_frame(s.bytes),
        s.live_delta,
        s.elapsed.as_secs_f64() * 1e6 / s.iters as f64,
    ));
}

fn print_row(variant: &str, scene: &str, items: usize, s: &Snap) {
    let allocs = s.per_frame(s.allocs);
    let bytes = s.per_frame(s.bytes);
    let live = s.per_frame(s.live_delta.max(0) as u64);
    let us = s.us_per_op();
    println!(
        "{variant:>9} {scene:>6} items={items:>5}  allocs={allocs:>8.0} bytes={bytes:>9.0} net={live:>5.0}  {us:>8.1} μs/f",
    );
}

fn main() {
    let mut json_path = None;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--json" => json_path = args.next(),
            other => panic!("unknown arg: {other}"),
        }
    }

    let mut json = Vec::new();
    let grids = [1usize, 3, 10, 20, 40];
    let iters = 30;

    for grid in grids {
        let items = grid * grid;
        let (statics, pairs) = build_sources(grid);
        let flags_static: Vec<Vec<bool>> = statics
            .iter()
            .map(|v| v.vpoints.get_closepath_flags())
            .collect();
        let flags_pair: Vec<Vec<bool>> = pairs
            .iter()
            .map(|(a, _)| a.vpoints.get_closepath_flags())
            .collect();

        // Baselines: the real chain.
        let static_timeline =
            (|r: &mut RanimScene| benches::test_scenes::static_squares(r, grid)).build_scene();
        let morph_timeline =
            (|r: &mut RanimScene| benches::test_scenes::transform_squares(r, grid)).build_scene();

        let s = Snap::measure(iters, || {
            black_box(static_timeline.eval_at_alpha(0.5).collect::<Vec<_>>());
        });
        report(&mut json, "baseline", "static", items, &s);
        print_row("baseline", "static", items, &s);

        let s = Snap::measure(iters, || {
            black_box(morph_timeline.eval_at_alpha(0.5).collect::<Vec<_>>());
        });
        report(&mut json, "baseline", "morph", items, &s);
        print_row("baseline", "morph", items, &s);

        // Arena variants share one bump per grid; reset between frames.
        let mut bump = Bump::with_capacity(32 * 1024 * 1024);

        let s = Snap::measure(iters, || {
            let mut frame = BumpFrame {
                items: Vec::with_capacity_in(items, &bump),
            };
            for src in &statics {
                s1_extract(&bump, src, &mut frame.items);
            }
            black_box(frame.items.len());
            drop(frame);
            bump.reset();
        });
        report(&mut json, "s1_arena", "static", items, &s);
        print_row("s1_arena", "static", items, &s);

        let s = Snap::measure(iters, || {
            let mut frame = BumpFrame {
                items: Vec::with_capacity_in(items, &bump),
            };
            for (a, b) in &pairs {
                s1_extract_lerped(&bump, a, b, 0.5, &mut frame.items);
            }
            black_box(frame.items.len());
            drop(frame);
            bump.reset();
        });
        report(&mut json, "s1_arena", "morph", items, &s);
        print_row("s1_arena", "morph", items, &s);

        let s = Snap::measure(iters, || {
            let mut frame = BumpFrame {
                items: Vec::with_capacity_in(items, &bump),
            };
            for (idx, src) in statics.iter().enumerate() {
                src.extract_into_arena(&flags_static[idx], &bump, &mut frame.items);
            }
            black_box(frame.items.len());
            drop(frame);
            bump.reset();
        });
        report(&mut json, "s2_direct", "static", items, &s);
        print_row("s2_direct", "static", items, &s);

        let s = Snap::measure(iters, || {
            let mut frame = BumpFrame {
                items: Vec::with_capacity_in(items, &bump),
            };
            for (idx, (a, b)) in pairs.iter().enumerate() {
                a.lerp_extract_into_arena(b, 0.5, &flags_pair[idx], &bump, &mut frame.items);
            }
            black_box(frame.items.len());
            drop(frame);
            bump.reset();
        });
        report(&mut json, "s2_direct", "morph", items, &s);
        print_row("s2_direct", "morph", items, &s);
    }

    if let Some(path) = json_path {
        std::fs::write(&path, format!("[\n  {}\n]\n", json.join(",\n  ")))
            .expect("write json report");
        println!("wrote {path}");
    }
}
