use std::time::{Duration, Instant};

use env_logger::Env;
use glam::{dvec2, vec3, Vec3};
use log::info;
use palette::{rgb, Srgba};
use ranim::{
    animation::{fading::Fading, transform::Transform, Animation, AnimationConfig},
    mobject::{
        geometry::{Arc, Polygon},
        Mobject, TransformAnchor,
    },
    scene::Scene,
    RanimContext,
};

async fn run() {
    env_logger::Builder::from_env(Env::default().default_filter_or("ranim=info")).init();

    let mut ctx = RanimContext::new();

    let mut scene = Scene::new(&ctx.wgpu_ctx);
    let t = Instant::now();

    let polygon = Polygon::from_verticies(vec![
        dvec2(-100.0, 0.0),
        dvec2(20.0, 30.0),
        dvec2(0.0, 70.0),
        dvec2(50.0, 0.0),
    ]);
    let mut polygon = Mobject::from_pipeline_vertex(&ctx.wgpu_ctx, polygon);
    polygon.set_color(Srgba::from_u32::<rgb::channels::Rgba>(0xE65A4CFF).into());
    polygon
        .scale(vec3(2.0, 4.0, 1.0), TransformAnchor::origin())
        .rotate(
            std::f32::consts::PI / 4.0,
            Vec3::Z,
            TransformAnchor::origin(),
        );

    let polygon = scene
        .play(
            &mut ctx,
            Animation::new(
                polygon,
                Fading::In,
                AnimationConfig::default().run_time(Duration::from_secs(1)),
            ),
        )
        .unwrap();

    let arc = Arc {
        angle: std::f64::consts::PI / 2.0,
    };
    let mut arc = Mobject::from_pipeline_vertex(&ctx.wgpu_ctx, arc);
    arc.set_color(Srgba::from_u32::<rgb::channels::Rgba>(0x29ABCAFF).into());
    arc.scale(Vec3::splat(100.0), TransformAnchor::edge(-1, -1, 0));

    let arc = scene
        .play(
            &mut ctx,
            Animation::new(
                polygon,
                Transform::new(&arc),
                AnimationConfig::default().run_time(Duration::from_secs(2)),
            ),
        )
        .unwrap();
    scene.play(
        &mut ctx,
        Animation::new(arc, Fading::Out, AnimationConfig::default().remove()),
    );

    info!("Total Time: {:?}", t.elapsed());
}
fn main() {
    println!("Hello, world!");
    pollster::block_on(run())
}
