//! Frame rendering benchmarks, including evaluation and render extraction.
//!
//! Low-level conversion moved from evaluation into extraction. Evaluating once
//! outside the measured loop would therefore compare different amounts of work
//! across renderer architectures, and does not model the playback/export path.
//!
//! - `frame_latency`: evaluate, extract, submit, and wait for GPU completion.
//! - `frame_cpu`: evaluate, extract, and submit; GPU completion is waited for
//!   outside each timed interval, so queued frames cannot accumulate.
//!
//! Neither metric is pure GPU time. Use GPU timestamp scopes for individual
//! passes. Scene construction, initialization, and readback are excluded here.

use std::time::{Duration, Instant};

use benches::test_scenes::{static_squares, transform_squares};
use criterion::{BenchmarkId, Criterion, SamplingMode, criterion_group, criterion_main};
use ranim::{
    SceneConstructor,
    items::{
        camera::CameraFramePlugin,
        vitem::{VItemPlugin, geometry::Square},
    },
    prelude::*,
};
use ranim_core::frame::EvaluatedFrame;
use ranim_render::{
    RenderAppExt, build_render_app,
    extract::ExtractPlugin,
    resource::RenderTextures,
    upload_probe::{self, UploadMode},
    utils::WgpuContext,
};

type PlacedSquare = Transformed<Square, Translation>;

fn frame_benchmark(c: &mut Criterion) {
    let ctx = pollster::block_on(WgpuContext::new());
    upload_probe::set_mode(UploadMode::Off);

    for cpu_only in [false, true] {
        let mut group = c.benchmark_group(if cpu_only {
            "frame_cpu"
        } else {
            "frame_latency"
        });
        group.sampling_mode(SamplingMode::Flat).sample_size(50);

        for morph in [false, true] {
            for n in [5, 10, 20, 40, 60] {
                let scene = (|r: &mut RanimScene| {
                    if morph {
                        transform_squares(r, n);
                    } else {
                        static_squares(r, n);
                    }
                })
                .build_scene();
                let mut store = EvaluatedFrame::new();
                let mut renderer = build_render_app(&ctx, 1920, 1080, 8);
                renderer.add_plugins((
                    CameraFramePlugin,
                    ExtractPlugin::<PlacedSquare>::default(),
                    VItemPlugin,
                ));
                renderer.finish();
                renderer.cleanup();
                renderer.set_gpu_timers_enabled(false);
                let mut render_textures = RenderTextures::new(&ctx, 1920, 1080);
                let clear_color = wgpu::Color {
                    r: 0.2,
                    g: 0.2,
                    b: 0.2,
                    a: 1.0,
                };

                // A deterministic changing playhead, including for static
                // content. Reset for every Criterion batch so both metrics
                // and revisions receive exactly the same time sequence.
                let alpha_at = |i: u64| 0.1 + 0.5 * (i % 120) as f64 / 120.0;
                for i in 0..20 {
                    scene.eval_at_alpha(alpha_at(i), &mut store);
                    renderer.render_frame(&mut render_textures, clear_color, &mut store);
                    ctx.device
                        .poll(wgpu::PollType::wait_indefinitely())
                        .unwrap();
                }

                group.bench_function(
                    BenchmarkId::new(if morph { "morph" } else { "static" }, n * n),
                    |b| {
                        b.iter_custom(|iterations| {
                            let mut measured = Duration::ZERO;
                            for i in 0..iterations {
                                let start = Instant::now();
                                scene.eval_at_alpha(alpha_at(i), &mut store);
                                renderer.render_frame(
                                    &mut render_textures,
                                    clear_color,
                                    &mut store,
                                );
                                if cpu_only {
                                    measured += start.elapsed();
                                }
                                ctx.device
                                    .poll(wgpu::PollType::wait_indefinitely())
                                    .unwrap();
                                if !cpu_only {
                                    measured += start.elapsed();
                                }
                            }
                            measured
                        });
                    },
                );
            }
        }
        group.finish();
    }
}

criterion_group!(benches, frame_benchmark);
criterion_main!(benches);
