//! Bench: the seal-time audio bake vs. a flattened span-mixer prototype.
//!
//! - `bake_*` — the shipped path: build the scene, `seal()`, which bakes
//!   the whole audio plane by walking the tree per output sample
//!   (`AnimationCell::mix_at`). Iterations include tree construction
//!   (dominated by the bake on every non-trivial shape).
//! - `flat_*` — a hand-rolled mixer over a flattened track list
//!   (start frame, window length, clip, gain): per-track tight loops with
//!   linear interpolation, the same math as `AudioTrack::sample_at`.
//!
//! Scene shapes (all 5 s at 48 kHz stereo, 240k output frames):
//!
//! - `seq_N`      — N sounds in sequence (disjoint windows);
//! - `stack_N`    — N sounds stacked at the origin (all audible everywhere);
//! - `silent_N`   — 1 short sound + N-1 visual cells (mostly silent tree);
//! - `depth_N`    — 1 sound nested in N stacked containers;
//! - `warp_*`     — every cell carries a non-linear rate (`smooth`);
//! - `flat_copy`  — single track, no interpolation (memory floor).

use std::f64::consts::TAU;
use std::hint::black_box;
use std::sync::Arc;

use criterion::{BenchmarkId, Criterion, SamplingMode, criterion_group, criterion_main};
use ranim::core::{SceneEvaluator, audio::AudioClip, utils::rate_functions::smooth};
use ranim::prelude::*;

const SR: u32 = 48_000;
const TOTAL_SECS: f64 = 5.0;
const OUT_FRAMES: usize = (TOTAL_SECS * SR as f64) as usize;

fn tone(secs: f64) -> AudioClip {
    AudioClip::sine(440.0, secs, 0.5)
}

/// N sequential sounds filling the window (disjoint play windows).
fn seq_sounds(n: usize) -> SceneEvaluator {
    let clip = tone(TOTAL_SECS / n as f64);
    let mut seq = AnimSequence::new();
    for _ in 0..n {
        seq.push(Sound::new(clip.clone()));
    }
    let mut scene = RanimScene::new();
    scene.play(seq);
    scene.seal().into_evaluator(120.0)
}

/// N sounds stacked at the origin, each covering the whole window.
fn stack_sounds(n: usize) -> SceneEvaluator {
    let clip = tone(TOTAL_SECS);
    let mut stack = AnimStack::new();
    for _ in 0..n {
        stack.push(Sound::new(clip.clone()));
    }
    let mut scene = RanimScene::new();
    scene.play(stack);
    scene.seal().into_evaluator(120.0)
}

/// One short sound plus `n - 1` visual cells: almost the whole tree is silent
/// at any instant, but the per-sample walk still visits every cell.
fn silent_tree(n: usize) -> SceneEvaluator {
    let mut stack = AnimStack::new();
    stack.push(Sound::new(tone(0.1)));
    for _ in 1..n {
        stack.push(CameraFrame::default().show().with_duration(TOTAL_SECS));
    }
    let mut scene = RanimScene::new();
    scene.play(stack);
    scene.seal().into_evaluator(120.0)
}

/// One sound nested in `depth` stacked containers.
fn nested_sound(depth: usize) -> SceneEvaluator {
    let mut cur = AnimStack::new();
    cur.push(Sound::new(tone(TOTAL_SECS)));
    for _ in 1..depth {
        let mut outer = AnimStack::new();
        outer.push(cur);
        cur = outer;
    }
    let mut scene = RanimScene::new();
    scene.play(cur);
    scene.seal().into_evaluator(120.0)
}

// Warp variants: every cell — containers, sounds, and visual fillers —
// carries a non-linear rate, so no affine fast path exists anywhere and all
// sound leaves resolve through the per-sample path replay.

fn warp_seq_sounds(n: usize) -> SceneEvaluator {
    let clip = tone(TOTAL_SECS / n as f64);
    let mut seq = AnimSequence::new();
    for _ in 0..n {
        seq.push(Sound::new(clip.clone()).with_rate_func(smooth));
    }
    let mut scene = RanimScene::new();
    scene.play(seq.with_rate_func(smooth));
    scene.seal().into_evaluator(120.0)
}

fn warp_stack_sounds(n: usize) -> SceneEvaluator {
    let clip = tone(TOTAL_SECS);
    let mut stack = AnimStack::new();
    for _ in 0..n {
        stack.push(Sound::new(clip.clone()).with_rate_func(smooth));
    }
    let mut scene = RanimScene::new();
    scene.play(stack.with_rate_func(smooth));
    scene.seal().into_evaluator(120.0)
}

fn warp_silent_tree(n: usize) -> SceneEvaluator {
    let mut stack = AnimStack::new();
    stack.push(Sound::new(tone(0.1)).with_rate_func(smooth));
    for _ in 1..n {
        stack.push(
            CameraFrame::default()
                .show()
                .with_duration(TOTAL_SECS)
                .with_rate_func(smooth),
        );
    }
    let mut scene = RanimScene::new();
    scene.play(stack.with_rate_func(smooth));
    scene.seal().into_evaluator(120.0)
}

fn warp_nested_sound(depth: usize) -> SceneEvaluator {
    let mut inner = AnimStack::new();
    inner.push(Sound::new(tone(TOTAL_SECS)).with_rate_func(smooth));
    let mut cur = inner.with_rate_func(smooth);
    for _ in 1..depth {
        let mut outer = AnimStack::new();
        outer.push(cur);
        cur = outer.with_rate_func(smooth);
    }
    let mut scene = RanimScene::new();
    scene.play(cur);
    scene.seal().into_evaluator(120.0)
}

/// A flattened track: a clip placed at an output-frame window.
struct FlatTrack {
    pcm: Arc<[f32]>, // interleaved stereo at SR
    start_frame: usize,
    len_frames: usize,
    gain: f32,
}

/// Shared stereo sine source for the flat mixer.
fn stereo_sine() -> Arc<[f32]> {
    (0..OUT_FRAMES)
        .flat_map(|i| {
            let v = (0.5 * (TAU * 440.0 * i as f64 / SR as f64).sin()) as f32;
            [v, v]
        })
        .collect()
}

/// Prototype mixer: tight per-track loops, no tree, no walk outside windows.
/// The leaf math mirrors `AudioTrack::sample_at` (linear interpolation, gain).
fn mix_flat(tracks: &[FlatTrack]) -> Vec<f32> {
    let mut out = vec![0.0f32; OUT_FRAMES * 2];
    for track in tracks {
        let src_frames = track.pcm.len() / 2;
        for i in 0..track.len_frames {
            let out_frame = track.start_frame + i;
            if out_frame >= OUT_FRAMES {
                break;
            }
            let src = i as f64; // step == 1.0: source and output grids coincide
            let f0 = src.floor() as usize;
            if f0 >= src_frames {
                break;
            }
            let frac = (src - f0 as f64) as f32;
            let f1 = (f0 + 1).min(src_frames - 1);
            let (s0, s1) = (f0 * 2, f1 * 2);
            let l = track.pcm[s0] + (track.pcm[s1] - track.pcm[s0]) * frac;
            let r = track.pcm[s0 + 1] + (track.pcm[s1 + 1] - track.pcm[s0 + 1]) * frac;
            out[out_frame * 2] += l * track.gain;
            out[out_frame * 2 + 1] += r * track.gain;
        }
    }
    out
}

fn audio_mix_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("audio_mix");
    group.sampling_mode(SamplingMode::Linear);
    group.sample_size(10);
    group.warm_up_time(std::time::Duration::from_millis(300));
    group.measurement_time(std::time::Duration::from_secs(2));

    // The seal-time bake: build the scene, seal, and bake the audio plane
    // (per-sample tree walk). Bench measures the whole path — tree
    // construction is included but dominated by the bake on every
    // non-trivial shape.
    for (name, n) in [
        ("seq", 1usize),
        ("seq", 10),
        ("seq", 100),
        ("seq", 1000),
        ("stack", 1),
        ("stack", 10),
        ("stack", 100),
        ("silent", 100),
        ("silent", 1000),
        ("depth", 2),
        ("depth", 8),
        ("depth", 64),
        ("warp_seq", 100),
        ("warp_seq", 1000),
        ("warp_stack", 100),
        ("warp_silent", 1000),
        ("warp_depth", 64),
    ] {
        let build: fn(usize) -> SceneEvaluator = match name {
            "seq" => seq_sounds,
            "stack" => stack_sounds,
            "silent" => silent_tree,
            "depth" => nested_sound,
            "warp_seq" => warp_seq_sounds,
            "warp_stack" => warp_stack_sounds,
            "warp_silent" => warp_silent_tree,
            _ => warp_nested_sound,
        };
        group.bench_with_input(BenchmarkId::new(format!("bake_{name}"), n), &n, |b, n| {
            b.iter(|| black_box(build(*n)));
        });
    }

    // The seal-time flattened prototype over identical content.
    let pcm = stereo_sine();
    for (name, n) in [
        ("stack", 1usize),
        ("stack", 10),
        ("stack", 100),
        ("seq", 10),
        ("seq", 1000),
    ] {
        let per = OUT_FRAMES / n.max(1);
        let tracks: Vec<FlatTrack> = (0..n)
            .map(|i| FlatTrack {
                pcm: pcm.clone(),
                start_frame: if name == "seq" { i * per } else { 0 },
                len_frames: if name == "seq" { per } else { OUT_FRAMES },
                gain: 1.0,
            })
            .collect();
        group.bench_with_input(BenchmarkId::new(format!("flat_{name}"), n), &n, |b, _| {
            b.iter(|| black_box(mix_flat(&tracks)));
        });
    }

    // Memory floor: one track, straight copy + gain, no interpolation.
    group.bench_function("flat_copy_1", |b| {
        b.iter(|| {
            let mut out = vec![0.0f32; OUT_FRAMES * 2];
            for i in 0..OUT_FRAMES {
                out[i * 2] = pcm[i * 2];
                out[i * 2 + 1] = pcm[i * 2 + 1];
            }
            black_box(out);
        });
    });

    group.finish();
}

criterion_group!(benches, audio_mix_benchmark);
criterion_main!(benches);
