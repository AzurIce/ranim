use benches::test_scenes::{static_squares, transform_squares};
use criterion::{BenchmarkId, Criterion, SamplingMode, criterion_group, criterion_main};
use ranim::{Output, SceneConfig, cmd::render_scene_output, prelude::*};

// 渲染性能测试函数
fn render_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("render");
    group.sampling_mode(SamplingMode::Linear).sample_size(10);

    // 测试不同规模的场景
    for n in [5, 10, 20, 40].iter() {
        group.bench_with_input(BenchmarkId::new("static_squares", n), n, |b, n| {
            b.iter(|| {
                // 执行渲染
                render_scene_output(
                    |r: &mut RanimScene| static_squares(r, *n),
                    format!("static_squares_{n}"),
                    &SceneConfig::default(),
                    &Output {
                        dir: "bench".to_string(),
                        ..Default::default()
                    },
                );
            });
        });
        group.bench_with_input(BenchmarkId::new("transform_squares", n), n, |b, n| {
            b.iter(|| {
                // 执行渲染
                render_scene_output(
                    |r: &mut RanimScene| transform_squares(r, *n),
                    format!("transform_squares_{n}"),
                    &SceneConfig::default(),
                    &Output {
                        dir: "bench".to_string(),
                        ..Default::default()
                    },
                );
            });
        });
    }

    group.finish();
}

criterion_group!(benches, render_benchmark);
criterion_main!(benches);
