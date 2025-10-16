use std::{f64::consts::PI, hint::black_box};

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use ranim::{
    core::Extract,
    glam::{DVec3, dvec3},
    items::vitem::geometry::{Arc, Circle, Polygon, Rectangle, Square},
};

fn extract_bench(c: &mut Criterion) {
    let mut g = c.benchmark_group("extract");

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
            b.iter(|| black_box(x.extract()));
        });
    }
    g.bench_function("square", |b| {
        let x = Square::new(2.0);
        b.iter(|| black_box(x.extract()));
    });
    g.bench_function("rectangle", |b| {
        let x = Rectangle::new(2.0, 4.0);
        b.iter(|| black_box(x.extract()));
    });
    g.bench_function("circle", |b| {
        let x = Circle::new(2.0);
        b.iter(|| black_box(x.extract()));
    });
    g.bench_function("arc", |b| {
        let x = Arc::new(PI * 3.0 / 4.0, 2.0);
        b.iter(|| black_box(x.extract()));
    });
}

criterion_group!(benches, extract_bench);
criterion_main!(benches);
