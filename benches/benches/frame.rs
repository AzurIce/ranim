//! Replacing a typed frame with one complete CPU input.

use std::{f64::consts::PI, hint::black_box};

use criterion::{BatchSize, Bencher, BenchmarkId, Criterion, criterion_group, criterion_main};
use ranim::{
    core::frame::EvaluatedFrame,
    glam::{DVec3, dvec3},
    items::vitem::geometry::{Arc, Circle, Polygon, Rectangle, Square},
};

fn bench_frame<T: Clone + Send + Sync + 'static>(b: &mut Bencher<'_>, value: T) {
    let mut frame = EvaluatedFrame::new();
    b.iter_batched(
        || value.clone(),
        |value| {
            frame.clear();
            frame.push((0, 0), value);
            black_box(&frame);
        },
        BatchSize::SmallInput,
    );
}

fn frame_bench(c: &mut Criterion) {
    let mut g = c.benchmark_group("frame_replace");

    // 测试不同规模的场景
    for n in [5, 10, 20, 40].iter() {
        g.bench_with_input(BenchmarkId::new("polygon", n), n, |b, n| {
            let points = (0..*n)
                .map(|i: i32| {
                    let angle = 2.0 * std::f64::consts::PI * (i as f64) / 16.0;
                    dvec3(angle.cos(), angle.sin(), 0.0)
                })
                .collect::<Vec<DVec3>>();
            let x = Polygon::new(points);
            bench_frame(b, x);
        });
    }
    g.bench_function("square", |b| {
        let x = Square::new(2.0);
        bench_frame(b, x);
    });
    g.bench_function("rectangle", |b| {
        let x = Rectangle::new(2.0, 4.0);
        bench_frame(b, x);
    });
    g.bench_function("circle", |b| {
        let x = Circle::new(2.0);
        bench_frame(b, x);
    });
    g.bench_function("arc", |b| {
        let x = Arc::new(PI * 3.0 / 4.0, 2.0);
        bench_frame(b, x);
    });
}

criterion_group!(benches, frame_bench);
criterion_main!(benches);
