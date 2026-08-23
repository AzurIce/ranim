//! CPU-side comparison of the two render-feeding paths:
//! - buffered: SceneEvaluator -> EvaluatedFrame -> RenderFrame -> reconcile
//! - direct:   ScenePlayer materialize -> world exchange -> reconcile_logic

use std::hint::black_box;

use benches::test_scenes::{static_squares, transform_squares};
use criterion::{
    BatchSize, BenchmarkId, Criterion, SamplingMode, criterion_group, criterion_main,
    measurement::WallTime,
};
use ranim::{SceneConstructor, bevy_ecs::world::World, prelude::*};
use ranim_core::{EvaluatedFrame, SceneEvaluator, ScenePlayer, SealedRanimScene};
use ranim_render::world::{RenderFrame, reconcile, reconcile_logic};

const FRAMES: usize = 60;

fn run_buffered(scene: SealedRanimScene) {
    let mut session = SceneEvaluator::new(scene);
    let total = session.total_secs();
    let mut render_world = World::new();
    let mut store = RenderFrame::new();
    let mut frame = EvaluatedFrame::new();
    for i in 0..FRAMES {
        frame.clear();
        session.sample_at(black_box(total * i as f64 / FRAMES as f64), &mut frame);
        store.update(frame.drain(..));
        reconcile(black_box(&mut render_world), black_box(&store));
    }
}

fn run_direct(scene: SealedRanimScene) {
    let mut player = ScenePlayer::new(scene);
    let total = player.total_secs();
    let mut render_world = World::new();
    for i in 0..FRAMES {
        player.materialize_at(black_box(total * i as f64 / FRAMES as f64));
        let mut logic = player.take_world();
        reconcile_logic(black_box(&mut render_world), black_box(&mut logic));
        player.put_world(logic);
    }
}

fn bench_pair(
    group: &mut criterion::BenchmarkGroup<'_, WallTime>,
    scene_name: &str,
    n: usize,
    build: impl Fn(&mut RanimScene, usize) + Copy + Send + Sync + 'static,
) {
    let id = |path: &str| BenchmarkId::new(format!("{scene_name}/{path}"), n);
    group.bench_with_input(id("buffered"), &n, |b, n| {
        b.iter_batched(
            || (|r: &mut RanimScene| build(r, *n)).build_scene(),
            run_buffered,
            BatchSize::SmallInput,
        );
    });
    group.bench_with_input(id("direct"), &n, |b, n| {
        b.iter_batched(
            || (|r: &mut RanimScene| build(r, *n)).build_scene(),
            run_direct,
            BatchSize::SmallInput,
        );
    });
}

fn extract_path_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("extract_path");
    group.sampling_mode(SamplingMode::Linear).sample_size(10);

    for n in [10, 50] {
        bench_pair(&mut group, "static_squares", n, static_squares);
        bench_pair(&mut group, "transform_squares", n, transform_squares);
    }

    group.finish();
}

criterion_group!(benches, extract_path_benchmark);
criterion_main!(benches);
