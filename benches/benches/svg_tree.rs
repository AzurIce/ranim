//! SVG parsing, whole-value frame insertion, posing, interpolation, and AABB benchmarks.
//!
//! The asset is the Ghostscript Tiger — the largest SVG in the repository.

use std::hint::black_box;

use criterion::{BatchSize, Criterion, Throughput, criterion_group, criterion_main};
use ranim::{
    core::anchor::Aabb,
    core::frame::EvaluatedFrame,
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

    // Parse and construct canonical SVG geometry.
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

    // One animation frame that moves and stores the complete SVG item.
    group.bench_function("anim frame: clone + rotate + shift + frame push", |b| {
        let svg = tiger();
        let mut output = EvaluatedFrame::new();
        b.iter(|| {
            let mut frame = black_box(&svg).clone();
            frame.rotate_on_axis(DVec3::Z, 0.05);
            frame.shift(DVec3::X * 0.01);
            output.clear();
            output.push((0, 0), frame);
            black_box(&output);
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

    // Replace the frame with one whole SVG, preparing its clone outside timing.
    group.bench_function("replace frame with whole SVG (tiger)", |b| {
        let svg = tiger();
        let mut frame = EvaluatedFrame::new();
        b.iter_batched(
            || svg.clone(),
            |svg| {
                frame.clear();
                frame.push((0, 0), svg);
                black_box(&frame);
            },
            BatchSize::SmallInput,
        );
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
