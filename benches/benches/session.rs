//! Frame-sampling comparison between the two `SceneSession` implementations:
//! the pure evaluator (`SceneEvaluator`) and the retained-world player
//! (`ScenePlayer`, M2 LogicWorld).

use std::hint::black_box;

use benches::test_scenes::{static_squares, transform_squares};
use criterion::{
    BatchSize, BenchmarkId, Criterion, SamplingMode, criterion_group, criterion_main,
    measurement::WallTime,
};
use ranim::{SceneConstructor, prelude::*};
use ranim_core::{EvaluatedFrame, SceneEvaluator, ScenePlayer, SceneSession, SealedRanimScene};

/// Sample `frames` evenly spaced points across the scene, reusing one frame
/// buffer like the render loop does.
fn sample_loop<S: SceneSession>(scene: SealedRanimScene, frames: usize) {
    let mut session = S::from_sealed(scene);
    let total = session.total_secs();
    let mut frame = EvaluatedFrame::new();
    for i in 0..frames {
        frame.clear();
        session.sample_at(black_box(total * i as f64 / frames as f64), &mut frame);
        black_box(&frame);
    }
}

fn bench_pair(
    group: &mut criterion::BenchmarkGroup<'_, WallTime>,
    scene_name: &str,
    n: usize,
    build: impl Fn(&mut RanimScene, usize) + Copy + Send + Sync + 'static,
) {
    let id = |session: &str| BenchmarkId::new(format!("{scene_name}/{session}"), n);
    group.bench_with_input(id("evaluator"), &n, |b, n| {
        b.iter_batched(
            || (|r: &mut RanimScene| build(r, *n)).build_scene(),
            |scene| sample_loop::<SceneEvaluator>(scene, 60),
            BatchSize::SmallInput,
        );
    });
    group.bench_with_input(id("player"), &n, |b, n| {
        b.iter_batched(
            || (|r: &mut RanimScene| build(r, *n)).build_scene(),
            |scene| sample_loop::<ScenePlayer>(scene, 60),
            BatchSize::SmallInput,
        );
    });
}

fn session_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("session");
    group.sampling_mode(SamplingMode::Linear).sample_size(10);

    for n in [10, 50] {
        bench_pair(&mut group, "static_squares", n, static_squares);
        bench_pair(&mut group, "transform_squares", n, transform_squares);
    }

    group.finish();
}

criterion_group!(benches, session_benchmark);
criterion_main!(benches);
