use benches::test_scenes::{static_squares, transform_squares};
use criterion::{BenchmarkId, Criterion, SamplingMode, criterion_group, criterion_main};
use ranim::{
    Output, Scene,
    cmd::render_scene_output,
    items::{RanimItemsPlugin, vitem::geometry::Square},
    prelude::*,
    render::{RenderSetup, extract::ExtractPlugin},
};

// 渲染性能测试函数
fn render_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("render");
    group.sampling_mode(SamplingMode::Linear).sample_size(10);

    // 测试不同规模的场景
    for n in [5, 10, 20, 40].iter() {
        let count = *n;
        let mut app = App::new();
        app.insert_resource(RenderSetup::new(|render_app| {
            render_app.add_plugins((
                RanimItemsPlugin,
                ExtractPlugin::<Transformed<Square, Translation>>::default(),
            ));
        }));
        app.add_scenes([
            Scene::new(format!("static_squares_{n}"), move |r: &mut RanimScene| {
                static_squares(r, count);
            }),
            Scene::new(
                format!("transform_squares_{n}"),
                move |r: &mut RanimScene| {
                    transform_squares(r, count);
                },
            ),
        ]);
        let setup = app.world().resource::<RenderSetup>().clone();
        let static_scene = app.scene(&format!("static_squares_{n}")).unwrap();
        let transform_scene = app.scene(&format!("transform_squares_{n}")).unwrap();
        group.bench_with_input(BenchmarkId::new("static_squares", n), n, |b, _| {
            b.iter(|| {
                // 执行渲染
                render_scene_output(
                    &static_scene,
                    &setup,
                    &Output {
                        dir: "./output/bench".to_string(),
                        ..Default::default()
                    },
                    2,
                );
            });
        });
        group.bench_with_input(BenchmarkId::new("transform_squares", n), n, |b, _| {
            b.iter(|| {
                // 执行渲染
                render_scene_output(
                    &transform_scene,
                    &setup,
                    &Output {
                        dir: "./output/bench".to_string(),
                        ..Default::default()
                    },
                    2,
                );
            });
        });
    }

    group.finish();
}

criterion_group!(benches, render_benchmark);
criterion_main!(benches);
