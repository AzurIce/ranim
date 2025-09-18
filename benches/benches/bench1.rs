use criterion::{BenchmarkId, Criterion, SamplingMode, criterion_group, criterion_main};
use itertools::Itertools;
use ranim::{
    Output, SceneConfig, SceneConstructor,
    anims::transform::TransformAnim,
    glam::{DVec3, dvec3},
    items::vitem::{
        VItem,
        geometry::{Circle, Square},
    },
    prelude::*,
};

fn static_squares(r: &mut RanimScene, n: usize) {
    let buff = 0.1;
    let size = 8.0 / n as f64;

    let unit = size + buff;
    let start = dvec3(-4.0, -4.0, 0.0);
    let _squares = (0..n)
        .cartesian_product(0..n)
        .map(|(i, j)| {
            Square::new(size).with(|square| {
                square
                    .put_center_on(start + unit * DVec3::X * j as f64 + unit * DVec3::Y * i as f64);
            })
        })
        .map(|item| r.insert_and_show(item))
        .collect::<Vec<_>>();
    r.timelines_mut().forward(1.0);
}

fn transform_squares(r: &mut RanimScene, n: usize) {
    let buff = 0.1;
    let size = 8.0 / n as f64 - buff;

    let unit = size + buff;
    let start = dvec3(-4.0, -4.0, 0.0);
    let squares = (0..n)
        .cartesian_product(0..n)
        .map(|(i, j)| {
            VItem::from(Square::new(size).with(|square| {
                square
                    .put_center_on(start + unit * DVec3::X * j as f64 + unit * DVec3::Y * i as f64);
            }))
        })
        .map(|item| r.insert(item))
        .collect::<Vec<_>>();
    let circles = (0..n)
        .cartesian_product(0..n)
        .map(|(i, j)| {
            VItem::from(Circle::new(size / 2.0).with(|circle| {
                circle
                    .put_center_on(start + unit * DVec3::X * j as f64 + unit * DVec3::Y * i as f64);
            }))
        })
        .collect::<Vec<_>>();
    squares.iter().zip(circles).for_each(|(r_square, circle)| {
        r.timeline_mut(r_square)
            .play_with(|item| item.transform_to(circle));
    });
}

// 渲染性能测试函数
fn render_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("Scene Rendering");
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
                        dir: "bench",
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
                        dir: "bench",
                        ..Default::default()
                    },
                );
            });
        });
    }

    group.finish();
}

fn eval_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("Evaluating");
    group.sampling_mode(SamplingMode::Linear).sample_size(10);
    // rayon::ThreadPoolBuilder::new().build_global().unwrap();

    // 测试不同规模的场景
    for n in [10, 100, 1000, 10000].iter() {
        group.bench_with_input(BenchmarkId::new("eval_static_squares", n), n, |b, n| {
            let timeline = (|r: &mut RanimScene| static_squares(r, *n)).build_scene();
            b.iter(|| {
                timeline.eval_alpha(0.5);
            });
        });
        group.bench_with_input(BenchmarkId::new("eval_transform_squares", n), n, |b, n| {
            let timeline = (|r: &mut RanimScene| transform_squares(r, *n)).build_scene();
            b.iter(|| {
                timeline.eval_alpha(0.5);
            });
        });
    }

    group.finish();
}

criterion_group!(benches, render_benchmark, eval_benchmark);
criterion_main!(benches);
