//! Benchmarks comparing the old (flat, baked-points) and new (hierarchical,
//! placed-tree) `SvgItem` implementations on identical user-level code:
//! parse + construction, one animation frame (clone + pose + extract),
//! isolated pose operations, extraction alone, and AABB queries.
//!
//! Both branches compile this file unchanged; run it on each and compare.
//! The asset is the Ghostscript Tiger — the largest SVG in the repository.

use std::hint::black_box;

use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use ranim::{
    core::Extract,
    core::anchor::Aabb,
    core::traits::{Interpolatable, RotateTransform, ShiftTransform},
    glam::DVec3,
    items::vitem::{VItem, svg::SvgItem},
};

const TIGER: &str = include_str!("../../assets/Ghostscript_Tiger.svg");

fn tiger() -> SvgItem {
    SvgItem::new(TIGER)
}

fn svg_benches(c: &mut Criterion) {
    let mut group = c.benchmark_group("svg_bench");

    // Parse + construction. The old implementation bakes every group
    // transform into the control points and then rewrites all points twice
    // more (move-to-origin + Y flip); the new one builds the tree and keeps
    // those as a root pose.
    let probed = tiger();
    let points: usize = Vec::<VItem>::from(probed.clone())
        .iter()
        .map(|leaf| leaf.vpoints.len())
        .sum();
    drop(probed);
    group.throughput(Throughput::Elements(points as u64));
    group.bench_function("parse+construct (tiger)", |b| {
        b.iter(|| black_box(tiger()));
    });

    // One animation frame that moves the whole item: clone, pose, extract.
    group.bench_function("anim frame: clone + rotate + shift + extract", |b| {
        let svg = tiger();
        b.iter(|| {
            let mut frame = black_box(&svg).clone();
            frame.rotate_on_axis(DVec3::Z, 0.05);
            frame.shift(DVec3::X * 0.01);
            black_box(frame.extract())
        });
    });

    // The pose operation alone.
    group.bench_function("pose: rotate (tiger)", |b| {
        let mut svg = tiger();
        b.iter(|| {
            black_box(&mut svg).rotate_on_axis(DVec3::Z, 0.05);
        });
    });

    group.bench_function("pose: shift (tiger)", |b| {
        let mut svg = tiger();
        b.iter(|| {
            black_box(&mut svg).shift(DVec3::X * 0.01);
        });
    });

    // Extraction alone.
    group.bench_function("extract (tiger)", |b| {
        let svg = tiger();
        b.iter(|| black_box(black_box(&svg).extract()));
    });

    // Interpolating between two baked states of the item (state-change
    // style animation, on the flattened items).
    group.bench_function("state lerp (flattened, tiger)", |b| {
        let a = Vec::<VItem>::from(tiger());
        let mut dst = Vec::<VItem>::from(tiger());
        for leaf in &mut dst {
            leaf.shift(DVec3::X * 3.0);
        }
        b.iter(|| black_box(black_box(&a).lerp(&dst, 0.5)));
    });

    group.bench_function("aabb (tiger)", |b| {
        let svg = tiger();
        b.iter(|| black_box(black_box(&svg).aabb()));
    });

    group.finish();
}

criterion_group! {
    name = benches;
    config = Criterion::default();
    targets = svg_benches
}
criterion_main!(benches);
