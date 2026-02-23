//! GPU rendering benchmark — isolates the pure render_store_with_pool cost.
//!
//! Measures:
//! - CPU-side submission time (buffer upload + command encoding + queue submit)
//! - Scales with VItem count to identify bottleneck (draw calls vs SDF vs upload)

use std::hint::black_box;

use benches::test_scenes::static_squares;
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, SamplingMode};
use ranim::{prelude::*, SceneConstructor};
use ranim_core::store::CoreItemStore;
use ranim_render::{resource::RenderPool, utils::WgpuContext, Renderer};

/// Pure GPU render benchmark: only measures render_store_with_pool + device.poll
fn gpu_render_benchmark(c: &mut Criterion) {
    let ctx = pollster::block_on(WgpuContext::new());

    let mut group = c.benchmark_group("gpu_render");
    group.sampling_mode(SamplingMode::Flat).sample_size(50);

    for n in [5, 10, 20, 40, 60].iter() {
        let vitem_count = n * n;

        // Build the scene and eval to get a CoreItemStore
        let scene = (|r: &mut RanimScene| static_squares(r, *n)).build_scene();
        let mut store = CoreItemStore::new();
        store.update(scene.eval_at_alpha(0.5));

        let mut renderer = Renderer::new(&ctx, 1920, 1080, 8);
        let mut render_textures = renderer.new_render_textures(&ctx);
        let mut pool = RenderPool::new();
        let clear_color = wgpu::Color {
            r: 0.2,
            g: 0.2,
            b: 0.2,
            a: 1.0,
        };

        // Warm up: render once to initialize all GPU resources
        renderer.render_store_with_pool(&ctx, &mut render_textures, clear_color, &store, &mut pool);
        pool.clean();
        ctx.device
            .poll(wgpu::PollType::wait_indefinitely())
            .unwrap();

        group.bench_with_input(
            BenchmarkId::new("submit", vitem_count),
            &vitem_count,
            |b, _| {
                b.iter(|| {
                    renderer.render_store_with_pool(
                        &ctx,
                        &mut render_textures,
                        clear_color,
                        &store,
                        &mut pool,
                    );
                    pool.clean();
                    // Wait for GPU to finish so we measure actual GPU time too
                    ctx.device
                        .poll(wgpu::PollType::wait_indefinitely())
                        .unwrap();
                    black_box(());
                });
            },
        );
    }

    group.finish();
}

/// Measures just the CPU-side submission cost (no GPU wait)
fn cpu_submit_benchmark(c: &mut Criterion) {
    let ctx = pollster::block_on(WgpuContext::new());

    let mut group = c.benchmark_group("cpu_submit");
    group.sampling_mode(SamplingMode::Flat).sample_size(50);

    for n in [5, 10, 20, 40, 60].iter() {
        let vitem_count = n * n;

        let scene = (|r: &mut RanimScene| static_squares(r, *n)).build_scene();
        let mut store = CoreItemStore::new();
        store.update(scene.eval_at_alpha(0.5));

        let mut renderer = Renderer::new(&ctx, 1920, 1080, 8);
        let mut render_textures = renderer.new_render_textures(&ctx);
        let mut pool = RenderPool::new();
        let clear_color = wgpu::Color {
            r: 0.2,
            g: 0.2,
            b: 0.2,
            a: 1.0,
        };

        // Warm up
        renderer.render_store_with_pool(&ctx, &mut render_textures, clear_color, &store, &mut pool);
        pool.clean();
        ctx.device
            .poll(wgpu::PollType::wait_indefinitely())
            .unwrap();

        group.bench_with_input(
            BenchmarkId::new("no_wait", vitem_count),
            &vitem_count,
            |b, _| {
                b.iter(|| {
                    renderer.render_store_with_pool(
                        &ctx,
                        &mut render_textures,
                        clear_color,
                        &store,
                        &mut pool,
                    );
                    pool.clean();
                    // Don't wait — measures pure CPU submission overhead
                    black_box(());
                });
            },
        );
    }

    group.finish();
}

/// Merged buffer path: GPU render benchmark (with GPU wait)
fn merged_gpu_render_benchmark(c: &mut Criterion) {
    let ctx = pollster::block_on(WgpuContext::new());

    let mut group = c.benchmark_group("merged_gpu_render");
    group.sampling_mode(SamplingMode::Flat).sample_size(50);

    for n in [5, 10, 20, 40, 60].iter() {
        let vitem_count = n * n;

        let scene = (|r: &mut RanimScene| static_squares(r, *n)).build_scene();
        let mut store = CoreItemStore::new();
        store.update(scene.eval_at_alpha(0.5));

        let mut renderer = Renderer::new(&ctx, 1920, 1080, 8);
        let mut render_textures = renderer.new_render_textures(&ctx);
        let clear_color = wgpu::Color {
            r: 0.2,
            g: 0.2,
            b: 0.2,
            a: 1.0,
        };

        // Warm up
        renderer.render_store_merged(&ctx, &mut render_textures, clear_color, &store);
        ctx.device
            .poll(wgpu::PollType::wait_indefinitely())
            .unwrap();

        group.bench_with_input(
            BenchmarkId::new("submit", vitem_count),
            &vitem_count,
            |b, _| {
                b.iter(|| {
                    renderer.render_store_merged(&ctx, &mut render_textures, clear_color, &store);
                    ctx.device
                        .poll(wgpu::PollType::wait_indefinitely())
                        .unwrap();
                    black_box(());
                });
            },
        );
    }

    group.finish();
}

/// Merged buffer path: CPU-only submission benchmark (no GPU wait)
fn merged_cpu_submit_benchmark(c: &mut Criterion) {
    let ctx = pollster::block_on(WgpuContext::new());

    let mut group = c.benchmark_group("merged_cpu_submit");
    group.sampling_mode(SamplingMode::Flat).sample_size(50);

    for n in [5, 10, 20, 40, 60].iter() {
        let vitem_count = n * n;

        let scene = (|r: &mut RanimScene| static_squares(r, *n)).build_scene();
        let mut store = CoreItemStore::new();
        store.update(scene.eval_at_alpha(0.5));

        let mut renderer = Renderer::new(&ctx, 1920, 1080, 8);
        let mut render_textures = renderer.new_render_textures(&ctx);
        let clear_color = wgpu::Color {
            r: 0.2,
            g: 0.2,
            b: 0.2,
            a: 1.0,
        };

        // Warm up
        renderer.render_store_merged(&ctx, &mut render_textures, clear_color, &store);
        ctx.device
            .poll(wgpu::PollType::wait_indefinitely())
            .unwrap();

        group.bench_with_input(
            BenchmarkId::new("no_wait", vitem_count),
            &vitem_count,
            |b, _| {
                b.iter(|| {
                    renderer.render_store_merged(&ctx, &mut render_textures, clear_color, &store);
                    black_box(());
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    gpu_render_benchmark,
    cpu_submit_benchmark,
    merged_gpu_render_benchmark,
    merged_cpu_submit_benchmark
);
criterion_main!(benches);
