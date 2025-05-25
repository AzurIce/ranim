use criterion::{BenchmarkId, Criterion, SamplingMode, criterion_group, criterion_main};
use itertools::Itertools;
use ranim::{
    animation::transform::TransformAnim,
    glam::{DVec3, dvec3},
    items::vitem::{
        VItem,
        geometry::{Circle, Square},
    },
    prelude::*,
};

#[scene]
struct StaticSquareScene(pub usize);

impl TimelineConstructor for StaticSquareScene {
    fn construct(self, timeline: &RanimScene, _camera: TimelineId<CameraFrame>) {
        let buff = 0.1;
        let size = 8.0 / self.0 as f64;

        let unit = size + buff;
        let start = dvec3(-4.0, -4.0, 0.0);
        let squares = (0..self.0)
            .cartesian_product(0..self.0)
            .map(|(i, j)| {
                Square::new(size).with(|square| {
                    square.put_center_on(
                        start + unit * DVec3::X * j as f64 + unit * DVec3::Y * i as f64,
                    );
                })
            })
            .collect::<Vec<_>>();
        let _squares = timeline.pin(squares);
        timeline.forward(1.0);
    }
}

#[scene]
struct TransformSquareScene(pub usize);

impl TimelineConstructor for TransformSquareScene {
    fn construct(self, timeline: &RanimScene, _camera: TimelineId<CameraFrame>) {
        let buff = 0.1;
        let size = 8.0 / self.0 as f64 - buff;

        let unit = size + buff;
        let start = dvec3(-4.0, -4.0, 0.0);
        let squares = (0..self.0)
            .cartesian_product(0..self.0)
            .map(|(i, j)| {
                VItem::from(Square::new(size).with(|square| {
                    square.put_center_on(
                        start + unit * DVec3::X * j as f64 + unit * DVec3::Y * i as f64,
                    );
                }))
            })
            .collect::<Vec<_>>();
        let circles = (0..self.0)
            .cartesian_product(0..self.0)
            .map(|(i, j)| {
                VItem::from(Circle::new(size / 2.0).with(|circle| {
                    circle.put_center_on(
                        start + unit * DVec3::X * j as f64 + unit * DVec3::Y * i as f64,
                    );
                }))
            })
            .collect::<Vec<_>>();
        timeline.play(
            squares
                .into_iter()
                .zip(circles)
                .map(|(square, circle)| square.transform_to(circle))
                .collect::<Vec<_>>(),
        );
    }
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
                render_scene(
                    StaticSquareScene(*n),
                    &AppOptions {
                        output_dir: format!("./output-bench/static_squares/{}", n).as_str(),
                        ..Default::default()
                    },
                );
            });
        });
        group.bench_with_input(BenchmarkId::new("transform_squares", n), n, |b, n| {
            b.iter(|| {
                // 执行渲染
                render_scene(
                    TransformSquareScene(*n),
                    &AppOptions {
                        output_dir: format!("./output-bench/transform_squares/{}", n).as_str(),
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
