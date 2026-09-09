//! Spike: three mixing strategies on a semantics-equivalent toy cell tree.
//!
//! The toy mirrors `AnimationCell::mix_at`: each cell owns a window in its
//! parent's content coordinates and maps parent time to content time via
//! `t = internal · rate((x − start) / dur)`; audio leaves sample their clip at
//! content time. Strategies compared:
//!
//! - `walk` — per-output-sample full-tree recursion (today's `mix_audio`);
//! - `block` — render-style: per 1024-frame block, ONE descent with window
//!   intersection; linear-rate paths (detected by comparing the rate fn
//!   pointer to `lin`) compose an affine global→content map on the fly and
//!   run a tight leaf loop; any non-linear rate on the path falls back to
//!   per-sample path evaluation — literally "eval through" along that
//!   leaf's path only;
//! - `flat` — seal-time flattening of affine leaves into a sorted span list
//!   (one descent for the whole timeline), then tight loops only.
//!
//! Correctness cross-check: `walk` vs `block` vs `flat` outputs are compared
//! elementwise on a mixed scene before benchmarking.

use std::f64::consts::TAU;
use std::hint::black_box;
use std::sync::Arc;

use criterion::{BenchmarkId, Criterion, SamplingMode, criterion_group, criterion_main};

const SR: f64 = 48_000.0;
const TOTAL: f64 = 5.0;
const FRAMES: usize = (TOTAL * SR) as usize; // 240_000
const BLOCK: usize = 1024;

fn lin(t: f64) -> f64 {
    t
}
fn smooth(t: f64) -> f64 {
    t * t * (3.0 - 2.0 * t)
}

// MARK: toy tree (mirrors AnimationCell::mix_at semantics)

#[derive(Clone)]
struct Track {
    pcm: Arc<[f32]>, // mono at SR
    play_len: f64,   // seconds of audible content
    gain: f32,
}

impl Track {
    fn sample_at(&self, t: f64) -> [f32; 2] {
        if !(0.0..self.play_len).contains(&t) {
            return [0.0; 2];
        }
        let src = t * SR;
        let f0 = src.floor() as usize;
        if f0 >= self.pcm.len() {
            return [0.0; 2];
        }
        let frac = (src - f0 as f64) as f32;
        let f1 = (f0 + 1).min(self.pcm.len() - 1);
        let v = (self.pcm[f0] + (self.pcm[f1] - self.pcm[f0]) * frac) * self.gain;
        [v, v]
    }
}

#[derive(Clone)]
enum Content {
    Leaf(Track),
    Group { internal: f64, children: Vec<Cell> },
}

#[derive(Clone)]
struct Cell {
    start: f64,
    end: f64,
    rate: fn(f64) -> f64,
    content: Content,
}

impl Cell {
    fn internal(&self) -> f64 {
        match &self.content {
            Content::Leaf(track) => track.play_len,
            Content::Group { internal, .. } => *internal,
        }
    }
    fn dur(&self) -> f64 {
        self.end - self.start
    }
}

// MARK: strategy 1 — per-sample walk (today's mix_audio)

fn cell_mix_at(c: &Cell, x: f64) -> [f32; 2] {
    if x < c.start || x >= c.end {
        return [0.0; 2];
    }
    let t = c.internal() * (c.rate)((x - c.start) / c.dur());
    match &c.content {
        Content::Leaf(track) => track.sample_at(t),
        Content::Group { children, .. } => {
            let mut acc = [0.0f32; 2];
            for child in children {
                let s = cell_mix_at(child, t);
                acc[0] += s[0];
                acc[1] += s[1];
            }
            acc
        }
    }
}

fn mix_walk(root: &[Cell]) -> Vec<f32> {
    let mut out = vec![0.0f32; FRAMES * 2];
    for f in 0..FRAMES {
        let x = f as f64 / SR;
        let mut acc = [0.0f32; 2];
        for c in root {
            let s = cell_mix_at(c, x);
            acc[0] += s[0];
            acc[1] += s[1];
        }
        out[f * 2] = acc[0];
        out[f * 2 + 1] = acc[1];
    }
    out
}

// MARK: strategy 2 — render-style block descent ("eval through" per block)

/// y = a + b·x, content seconds as an affine function of global seconds.
#[derive(Clone, Copy)]
struct Aff {
    a: f64,
    b: f64,
}

impl Aff {
    fn id() -> Self {
        Self { a: 0.0, b: 1.0 }
    }
    /// self' = m ∘ self: y = m_a + m_b·(a + b·x)
    fn then(self, m_a: f64, m_b: f64) -> Self {
        Self {
            a: m_a + m_b * self.a,
            b: m_b * self.b,
        }
    }
    fn inv_y(&self, y: f64) -> f64 {
        (y - self.a) / self.b
    }
}

/// One descent for one block. `span` is the requested time range in this
/// cell's parent-content coordinates, `global` the conservative global range
/// that can still reach here, `aff` the composed global→parent-content map
/// (valid while every rate so far was `lin`).
fn descend<'a>(
    c: &'a Cell,
    span: (f64, f64),
    global: (f64, f64),
    aff: Option<Aff>,
    path: &mut Vec<&'a Cell>,
    out: &mut [f32],
    t0: usize,
) {
    let lo = span.0.max(c.start);
    let hi = span.1.min(c.end);
    if lo >= hi {
        return;
    }
    let is_lin = std::ptr::fn_addr_eq(c.rate as fn(f64) -> f64, lin as fn(f64) -> f64);
    let internal = c.internal();
    // This cell's map: t = m_a + m_b·x in parent coordinates.
    let (m_a, m_b) = if is_lin {
        (-c.start * internal / c.dur(), internal / c.dur())
    } else {
        (0.0, f64::NAN) // unused
    };
    let (new_span, new_aff, new_global) = if is_lin {
        let ns = (m_a + m_b * lo, m_a + m_b * hi);
        let na = aff.map(|aff| aff.then(m_a, m_b));
        // Exact: global frames mapping into the intersected content span.
        let ng = match na {
            Some(aff) => (aff.inv_y(ns.0), aff.inv_y(ns.1)),
            None => global,
        };
        (ns, na, ng)
    } else {
        // Conservative: the whole content axis; global range can only narrow
        // through the map, so carrying it unchanged stays sound.
        ((0.0, internal), None, global)
    };

    path.push(c);
    match &c.content {
        Content::Leaf(track) => match new_aff {
            Some(aff) => {
                // Audible where content time ∈ [0, play_len): exact in global.
                let clo = new_span.0.max(0.0);
                let chi = new_span.1.min(internal);
                if clo < chi {
                    // Bounds in global frames. Lower bound takes an epsilon
                    // before ceil: the f64 round trip (÷SR then ×SR) can land
                    // a hair above an integer, and a bare ceil would skip the
                    // block's first frame. Upper bound is ceil = exclusive
                    // (the t == chi frame must stay silent, matching
                    // sample_at's half-open window).
                    let g_lo =
                        (((aff.inv_y(clo) * SR - 1e-9).ceil() as i64).max(t0 as i64)) as usize;
                    let g_hi = (((aff.inv_y(chi) * SR).ceil() as i64).min(t0 as i64 + BLOCK as i64))
                        .max(0) as usize;
                    let n = track.pcm.len();
                    for g in g_lo..g_hi {
                        let t = aff.a + aff.b * (g as f64 / SR);
                        // Window guard: makes the epsilon slack and any bound
                        // drift harmless (one compare per sample).
                        if !(0.0..internal).contains(&t) {
                            continue;
                        }
                        let src = t * SR;
                        let f0 = src.floor();
                        let frac = (src - f0) as f32;
                        let f0 = f0 as usize;
                        if f0 >= n {
                            continue;
                        }
                        let f1 = (f0 + 1).min(n - 1);
                        let v =
                            (track.pcm[f0] + (track.pcm[f1] - track.pcm[f0]) * frac) * track.gain;
                        let o = g * 2;
                        out[o] += v;
                        out[o + 1] += v;
                    }
                }
            }
            None => {
                // "Eval through": per sample, walk this leaf's path only.
                let g_lo = (new_global.0 * SR).ceil().max(t0 as f64) as usize;
                let g_hi = (new_global.1 * SR).floor().min(t0 as f64 + BLOCK as f64) as usize;
                for g in g_lo..g_hi {
                    let mut v = g as f64 / SR;
                    let mut audible = true;
                    for pc in path.iter() {
                        if !(pc.start..pc.end).contains(&v) {
                            audible = false;
                            break;
                        }
                        v = pc.internal() * (pc.rate)((v - pc.start) / pc.dur());
                    }
                    if audible {
                        let s = track.sample_at(v);
                        let o = g * 2;
                        out[o] += s[0];
                        out[o + 1] += s[1];
                    }
                }
            }
        },
        Content::Group { children, .. } => {
            for child in children {
                descend(child, new_span, new_global, new_aff, path, out, t0);
            }
        }
    }
    path.pop();
}

fn mix_block(root: &[Cell]) -> Vec<f32> {
    let mut out = vec![0.0f32; FRAMES * 2];
    for b in 0..FRAMES / BLOCK {
        let t0 = b * BLOCK;
        let lo = t0 as f64 / SR;
        let hi = (t0 + BLOCK) as f64 / SR;
        let mut path = Vec::new();
        for c in root {
            descend(
                c,
                (lo, hi),
                (lo, hi),
                Some(Aff::id()),
                &mut path,
                &mut out,
                t0,
            );
        }
    }
    out
}

// MARK: strategy 3 — seal-time flatten (affine leaves only)

struct FlatSpan {
    g_lo: usize,
    g_hi: usize,
    a: f64,
    b: f64, // t(g) = a + b·(g / SR)
    play_len: f64,
    pcm: Arc<[f32]>,
    gain: f32,
}

fn flatten_descend(c: &Cell, span: (f64, f64), aff: Aff, out: &mut Vec<FlatSpan>) {
    // Affine-only lowering: a non-linear rate has no composed affine, so its
    // subtree is left to the descent strategy (the real design's fallback).
    if !std::ptr::fn_addr_eq(c.rate as fn(f64) -> f64, lin as fn(f64) -> f64) {
        return;
    }
    let lo = span.0.max(c.start);
    let hi = span.1.min(c.end);
    if lo >= hi {
        return;
    }
    let internal = c.internal();
    let m_b = internal / c.dur();
    let m_a = -c.start * m_b;
    let ns = (m_a + m_b * lo, m_a + m_b * hi);
    let na = aff.then(m_a, m_b);
    match &c.content {
        Content::Leaf(track) => {
            let clo = ns.0.max(0.0);
            let chi = ns.1.min(internal);
            if clo < chi {
                out.push(FlatSpan {
                    g_lo: (na.inv_y(clo) * SR - 1e-9).ceil() as usize,
                    g_hi: (na.inv_y(chi) * SR).ceil() as usize,
                    a: na.a,
                    b: na.b,
                    play_len: internal,
                    pcm: track.pcm.clone(),
                    gain: track.gain,
                });
            }
        }
        Content::Group { children, .. } => {
            for child in children {
                flatten_descend(child, ns, na, out);
            }
        }
    }
}

fn mix_flat(spans: &[FlatSpan]) -> Vec<f32> {
    let mut out = vec![0.0f32; FRAMES * 2];
    for s in spans {
        let n = s.pcm.len();
        for g in s.g_lo..s.g_hi.min(FRAMES) {
            let t = s.a + s.b * (g as f64 / SR);
            if !(0.0..s.play_len).contains(&t) {
                continue;
            }
            let src = t * SR;
            let f0 = src.floor() as usize;
            if f0 >= n {
                continue;
            }
            let frac = (src - f0 as f64) as f32;
            let f1 = (f0 + 1).min(n - 1);
            let v = (s.pcm[f0] + (s.pcm[f1] - s.pcm[f0]) * frac) * s.gain;
            out[g * 2] += v;
            out[g * 2 + 1] += v;
        }
    }
    out
}

// MARK: scenes

fn sine_pcm() -> Arc<[f32]> {
    (0..FRAMES)
        .map(|i| (0.5 * (TAU * 440.0 * i as f64 / SR).sin()) as f32)
        .collect()
}

fn leaf(pcm: &Arc<[f32]>, start: f64, len: f64) -> Cell {
    Cell {
        start,
        end: start + len,
        rate: lin,
        content: Content::Leaf(Track {
            pcm: pcm.clone(),
            play_len: len,
            gain: 1.0,
        }),
    }
}

/// A visual stand-in: a leaf-shaped cell with no audio (empty group).
fn visual(start: f64, len: f64) -> Cell {
    Cell {
        start,
        end: start + len,
        rate: lin,
        content: Content::Group {
            internal: len,
            children: Vec::new(),
        },
    }
}

fn group(children: Vec<Cell>, internal: f64, rate: fn(f64) -> f64) -> Cell {
    Cell {
        start: 0.0,
        end: TOTAL,
        rate,
        content: Content::Group { internal, children },
    }
}

fn scene_seq(pcm: &Arc<[f32]>, n: usize) -> Vec<Cell> {
    let per = TOTAL / n as f64;
    vec![group(
        (0..n).map(|i| leaf(pcm, i as f64 * per, per)).collect(),
        TOTAL,
        lin,
    )]
}

fn scene_stack(pcm: &Arc<[f32]>, n: usize) -> Vec<Cell> {
    vec![group(
        (0..n).map(|_| leaf(pcm, 0.0, TOTAL)).collect(),
        TOTAL,
        lin,
    )]
}

fn scene_silent(pcm: &Arc<[f32]>, n: usize) -> Vec<Cell> {
    let mut children = vec![leaf(pcm, 0.0, 0.1)];
    children.extend((1..n).map(|_| visual(0.0, TOTAL)));
    vec![group(children, TOTAL, lin)]
}

fn scene_depth(pcm: &Arc<[f32]>, depth: usize) -> Vec<Cell> {
    let mut cur = leaf(pcm, 0.0, TOTAL);
    for _ in 1..depth {
        cur = group(vec![cur], TOTAL, lin);
    }
    vec![cur]
}

fn scene_warp_seq(pcm: &Arc<[f32]>, n: usize) -> Vec<Cell> {
    let per = TOTAL / n as f64;
    vec![group(
        (0..n).map(|i| leaf(pcm, i as f64 * per, per)).collect(),
        TOTAL,
        smooth,
    )]
}

fn scene_warp_stack(pcm: &Arc<[f32]>, n: usize) -> Vec<Cell> {
    vec![group(
        (0..n).map(|_| leaf(pcm, 0.0, TOTAL)).collect(),
        TOTAL,
        smooth,
    )]
}

/// Warp EVERY cell — containers, leaves, visual stand-ins: no linear rate
/// survives anywhere, so the block descent's affine path never applies.
fn warp_all(cells: &mut [Cell]) {
    fn warp_cell(cell: &mut Cell) {
        cell.rate = smooth;
        if let Content::Group { children, .. } = &mut cell.content {
            for child in children {
                warp_cell(child);
            }
        }
    }
    for cell in cells {
        warp_cell(cell);
    }
}

// MARK: cross-check + benchmark

fn close_enough(a: &[f32], b: &[f32]) -> bool {
    a.len() == b.len()
        && a.iter()
            .zip(b)
            .all(|(x, y)| (x - y).abs() < 1e-5 || (x.abs() < 1e-6 && y.abs() < 1e-6))
}

fn audio_block_benchmark(c: &mut Criterion) {
    let pcm = sine_pcm();

    // Correctness: the three strategies must agree on a mixed scene
    // (overlapping + sequential + nested + a non-linear warp path).
    {
        let mixed = vec![group(
            vec![
                leaf(&pcm, 0.5, 2.0),
                leaf(&pcm, 1.0, 1.0),
                group(vec![leaf(&pcm, 3.0, 1.5)], TOTAL, smooth),
                visual(0.0, TOTAL),
                leaf(&pcm, 4.25, 0.5),
            ],
            TOTAL,
            lin,
        )];
        let walked = mix_walk(&mixed);
        let blocked = mix_block(&mixed);
        assert!(
            close_enough(&walked, &blocked),
            "block != walk (first diff at frame {})",
            walked
                .iter()
                .zip(&blocked)
                .position(|(x, y)| (x - y).abs() >= 1e-5 && !(x.abs() < 1e-6 && y.abs() < 1e-6))
                .map(|i| i / 2)
                .unwrap_or(0)
        );
        let mut spans = Vec::new();
        for c in &mixed {
            flatten_descend(c, (0.0, TOTAL), Aff::id(), &mut spans);
        }
        let flat = mix_flat(&spans);
        // The warp child is affine-unrepresentable, so compare flat against a
        // walk of the same scene without it.
        let linear_only = vec![group(
            vec![
                leaf(&pcm, 0.5, 2.0),
                leaf(&pcm, 1.0, 1.0),
                visual(0.0, TOTAL),
                leaf(&pcm, 4.25, 0.5),
            ],
            TOTAL,
            lin,
        )];
        assert!(
            close_enough(&mix_walk(&linear_only), &flat),
            "flat != walk(linear-only)"
        );
    }

    let mut group = c.benchmark_group("audio_block");
    group.sampling_mode(SamplingMode::Linear);
    group.sample_size(10);
    group.warm_up_time(std::time::Duration::from_millis(300));
    group.measurement_time(std::time::Duration::from_secs(2));

    for (scene, n, has_flat) in [
        ("seq", 1000usize, true),
        ("stack", 100, true),
        ("silent", 1000, true),
        ("depth", 64, true),
        ("warp_seq", 100, false),
        ("warp_stack", 10, false),
        ("all_warp_seq", 1000, false),
        ("all_warp_seq", 100, false),
        ("all_warp_stack", 100, false),
        ("all_warp_silent", 1000, false),
        ("all_warp_depth", 64, false),
    ] {
        let mut cells = match scene {
            "seq" => scene_seq(&pcm, n),
            "stack" => scene_stack(&pcm, n),
            "silent" => scene_silent(&pcm, n),
            "depth" => scene_depth(&pcm, n),
            "warp_seq" => scene_warp_seq(&pcm, n),
            "warp_stack" => scene_warp_stack(&pcm, n),
            "all_warp_seq" => scene_seq(&pcm, n),
            "all_warp_stack" => scene_stack(&pcm, n),
            "all_warp_silent" => scene_silent(&pcm, n),
            _ => scene_depth(&pcm, n),
        };
        if scene.starts_with("all_warp") {
            warp_all(&mut cells);
        }
        group.bench_with_input(
            BenchmarkId::new("walk", format!("{scene}_{n}")),
            &n,
            |b, _| {
                b.iter(|| black_box(mix_walk(&cells)));
            },
        );
        group.bench_with_input(
            BenchmarkId::new("block", format!("{scene}_{n}")),
            &n,
            |b, _| {
                b.iter(|| black_box(mix_block(&cells)));
            },
        );
        if has_flat {
            let mut spans = Vec::new();
            for c in &cells {
                flatten_descend(c, (0.0, TOTAL), Aff::id(), &mut spans);
            }
            group.bench_with_input(
                BenchmarkId::new("flat", format!("{scene}_{n}")),
                &n,
                |b, _| {
                    b.iter(|| black_box(mix_flat(&spans)));
                },
            );
        }
    }

    group.finish();
}

criterion_group!(benches, audio_block_benchmark);
criterion_main!(benches);
