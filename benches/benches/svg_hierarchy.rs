//! Hierarchical-tree-only benchmarks (meaningful only on the hierarchy
//! implementation): id addressing, O(1) root posing, pose-only lerp, leaf
//! walking, and glTF parsing.

use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use ranim::{
    core::Extract,
    core::anchor::Aabb,
    core::core_item::transformed::Transformed,
    core::traits::{Interpolatable, ShiftTransform},
    glam::{DAffine3, DVec3},
    items::{hierarchy::Node, mesh::gltf, vitem::svg::SvgItem},
};

const TIGER: &str = include_str!("../../assets/Ghostscript_Tiger.svg");
const GENERATOR: &[u8] = include_bytes!("../../assets/models/generator.glb");

fn tiger() -> Transformed<Node<ranim::items::vitem::VItem>, DAffine3> {
    SvgItem::new(TIGER).into_tree()
}

fn hierarchy_benches(c: &mut Criterion) {
    let mut group = c.benchmark_group("svg_hierarchy");

    let tree = tiger();

    // A placement id present in the tiger asset (paths carry their SVG ids).
    let probe_id = "path-100";

    group.bench_function("by_id hit (tiger)", |b| {
        b.iter(|| black_box(black_box(&tree).inner.by_id(probe_id)))
    });

    group.bench_function("by_id miss (tiger)", |b| {
        b.iter(|| black_box(black_box(&tree).inner.by_id("does-not-exist")))
    });

    // Root posing is O(1): one matrix composition, no point is touched.
    group.bench_function("root pose: shift (tiger)", |b| {
        let mut placed = tree.clone();
        b.iter(|| {
            black_box(&mut placed).shift(DVec3::X * 0.001);
        });
    });

    // Pose-only lerp between two placements of the same tree: per frame
    // this is one matrix lerp per placement, no control-point lerp.
    group.bench_function("pose lerp between two placements (tiger)", |b| {
        let start = Transformed::new(tree.inner.clone(), DAffine3::IDENTITY);
        let end = Transformed::new(
            tree.inner.clone(),
            DAffine3::from_rotation_translation(
                ranim::glam::DQuat::from_rotation_z(1.0),
                DVec3::X * 3.0,
            ),
        );
        b.iter(|| black_box(black_box(&start).lerp(&end, 0.5)));
    });

    group.bench_function("extract placed tree (tiger)", |b| {
        b.iter(|| black_box(black_box(&tree).extract()));
    });

    group.bench_function("aabb placed tree (tiger)", |b| {
        b.iter(|| black_box(black_box(&tree).aabb()));
    });

    group.finish();

    let mut gltf_group = c.benchmark_group("gltf_import");
    let bytes = GENERATOR.to_vec();
    gltf_group.bench_function("parse generator.glb (136 KB)", |b| {
        b.iter(|| {
            let parsed = gltf::gltf::Gltf::from_slice(black_box(&bytes)).expect("valid glb");
            let blob = parsed.blob.clone();
            black_box(gltf::node_tree_from_gltf(
                &parsed.document,
                |buffer| match buffer.source() {
                    gltf::gltf::buffer::Source::Bin => blob.as_deref(),
                    gltf::gltf::buffer::Source::Uri(_) => None,
                },
            ))
        });
    });
    gltf_group.finish();
}

criterion_group! {
    name = benches;
    config = Criterion::default();
    targets = hierarchy_benches
}
criterion_main!(benches);
