//! Benchmark to measure the performance impact of `discard` in mesh_item.wgsl's fs_main.
//!
//! Run procedure:
//! 1. With `discard` commented out:  cargo bench --bench mesh_discard -- --save-baseline no_discard
//! 2. With `discard` enabled:        cargo bench --bench mesh_discard -- --save-baseline with_discard
//! 3. Compare: cargo bench --bench mesh_discard -- --load-baseline no_discard --baseline with_discard

use std::hint::black_box;

use criterion::{BenchmarkId, Criterion, SamplingMode, criterion_group, criterion_main};
use glam::{Mat4, Vec3, Vec4};
use ranim_core::{
    components::rgba::Rgba,
    core_item::{CoreItem, camera_frame::CameraFrame, mesh_item::MeshItem},
    store::CoreItemStore,
};
use ranim_render::{Renderer, resource::RenderPool, utils::WgpuContext};

fn create_sphere_mesh(color: Rgba, radius: f32, position: Vec3, segments: u32) -> MeshItem {
    let mut points = Vec::new();
    let mut indices = Vec::new();

    let lat_segments = segments;
    let lon_segments = segments;

    for lat in 0..=lat_segments {
        let theta = lat as f32 * std::f32::consts::PI / lat_segments as f32;
        let sin_theta = theta.sin();
        let cos_theta = theta.cos();

        for lon in 0..=lon_segments {
            let phi = lon as f32 * 2.0 * std::f32::consts::PI / lon_segments as f32;
            let sin_phi = phi.sin();
            let cos_phi = phi.cos();

            let x = sin_theta * cos_phi;
            let y = sin_theta * sin_phi;
            let z = cos_theta;

            points.push(Vec3::new(x * radius, y * radius, z * radius) + position);
        }
    }

    for lat in 0..lat_segments {
        for lon in 0..lon_segments {
            let first = lat * (lon_segments + 1) + lon;
            let second = first + lon_segments + 1;

            indices.push(first);
            indices.push(second);
            indices.push(first + 1);

            indices.push(second);
            indices.push(second + 1);
            indices.push(first + 1);
        }
    }

    let vertex_colors = vec![color; points.len()];
    let vertex_normals = points.iter().map(|p| (*p - position).normalize()).collect();

    MeshItem {
        points,
        triangle_indices: indices,
        transform: Mat4::IDENTITY,
        vertex_colors,
        vertex_normals,
    }
}

/// Benchmark with nested transparent spheres (the OIT scenario).
fn mesh_discard_benchmark(c: &mut Criterion) {
    let ctx = pollster::block_on(WgpuContext::new());

    let mut group = c.benchmark_group("mesh_discard");
    group.sampling_mode(SamplingMode::Flat).sample_size(100);

    // Test with different polygon counts to see scaling
    for segments in [20, 40, 80] {
        let outer_transparent = Rgba(Vec4::new(0.0, 0.0, 1.0, 0.3));
        let middle_opaque = Rgba(Vec4::new(1.0, 0.0, 0.0, 1.0));
        let inner_transparent = Rgba(Vec4::new(0.0, 1.0, 0.0, 0.5));

        let outer_sphere = create_sphere_mesh(outer_transparent, 2.0, Vec3::ZERO, segments);
        let middle_sphere = create_sphere_mesh(middle_opaque, 1.5, Vec3::ZERO, segments);
        let inner_sphere = create_sphere_mesh(inner_transparent, 1.0, Vec3::ZERO, segments);

        let tri_count = 3 * 2 * segments * segments; // per sphere

        let mut store = CoreItemStore::new();
        store.update(
            [
                ((0, 0), CoreItem::CameraFrame(CameraFrame::default())),
                ((1, 0), CoreItem::MeshItem(outer_sphere)),
                ((2, 0), CoreItem::MeshItem(middle_sphere)),
                ((3, 0), CoreItem::MeshItem(inner_sphere)),
            ]
            .into_iter(),
        );

        let width = 1920u32;
        let height = 1080u32;
        let mut renderer = Renderer::new(&ctx, width, height, 8);
        let mut render_textures = renderer.new_render_textures(&ctx);
        let mut pool = RenderPool::new();
        let clear_color = wgpu::Color {
            r: 0.1,
            g: 0.1,
            b: 0.1,
            a: 1.0,
        };

        // Warm up
        renderer.render_store_with_pool(&ctx, &mut render_textures, clear_color, &store, &mut pool);
        pool.clean();
        ctx.device
            .poll(wgpu::PollType::wait_indefinitely())
            .unwrap();

        group.bench_with_input(
            BenchmarkId::new("nested_spheres", tri_count),
            &tri_count,
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

/// Benchmark with many small transparent spheres spread out (high fragment overdraw).
fn mesh_discard_overdraw_benchmark(c: &mut Criterion) {
    let ctx = pollster::block_on(WgpuContext::new());

    let mut group = c.benchmark_group("mesh_discard_overdraw");
    group.sampling_mode(SamplingMode::Flat).sample_size(100);

    // Create a grid of overlapping transparent spheres
    for count in [9, 25] {
        let side = (count as f32).sqrt() as i32;
        let mut items: Vec<((usize, usize), CoreItem)> = vec![
            ((0, 0), CoreItem::CameraFrame(CameraFrame::default())),
        ];

        let segments = 30;
        for i in 0..side {
            for j in 0..side {
                let x = (i - side / 2) as f32 * 1.5;
                let y = (j - side / 2) as f32 * 1.5;
                let alpha = 0.3 + 0.05 * ((i + j) as f32);
                let color = Rgba(Vec4::new(
                    i as f32 / side as f32,
                    j as f32 / side as f32,
                    0.5,
                    alpha.min(0.9),
                ));
                let sphere = create_sphere_mesh(color, 1.0, Vec3::new(x, y, 0.0), segments);
                let idx = (i * side + j + 1) as usize;
                items.push(((idx, 0), CoreItem::MeshItem(sphere)));
            }
        }

        let mut store = CoreItemStore::new();
        store.update(items.into_iter());

        let width = 1920u32;
        let height = 1080u32;
        let mut renderer = Renderer::new(&ctx, width, height, 8);
        let mut render_textures = renderer.new_render_textures(&ctx);
        let mut pool = RenderPool::new();
        let clear_color = wgpu::Color {
            r: 0.1,
            g: 0.1,
            b: 0.1,
            a: 1.0,
        };

        // Warm up
        renderer.render_store_with_pool(&ctx, &mut render_textures, clear_color, &store, &mut pool);
        pool.clean();
        ctx.device
            .poll(wgpu::PollType::wait_indefinitely())
            .unwrap();

        group.bench_with_input(
            BenchmarkId::new("transparent_grid", count),
            &count,
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

criterion_group!(benches, mesh_discard_benchmark, mesh_discard_overdraw_benchmark);
criterion_main!(benches);
