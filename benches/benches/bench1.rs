use std::hint::black_box;

use benches::test_scenes::{static_squares, transform_squares};
use criterion::{BenchmarkId, Criterion, SamplingMode, criterion_group, criterion_main};
use ranim::{SceneConstructor, prelude::*};

fn eval_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("eval");
    group.sampling_mode(SamplingMode::Linear).sample_size(10);
    // rayon::ThreadPoolBuilder::new().build_global().unwrap();

    // 测试不同规模的场景
    for n in [10, 100, 1000].iter() {
        group.bench_with_input(BenchmarkId::new("eval_static_squares", n), n, |b, n| {
            let timeline = (|r: &mut RanimScene| static_squares(r, *n)).build_scene();
            b.iter(|| {
                black_box(timeline.eval_at_alpha(0.5).collect::<Vec<_>>());
            });
        });
        group.bench_with_input(BenchmarkId::new("eval_transform_squares", n), n, |b, n| {
            let timeline = (|r: &mut RanimScene| transform_squares(r, *n)).build_scene();
            b.iter(|| {
                black_box(timeline.eval_at_alpha(0.5).collect::<Vec<_>>());
            });
        });
    }

    group.finish();
}

criterion_group!(benches, eval_benchmark);
criterion_main!(benches);
