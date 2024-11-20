use std::time::{Duration, Instant};

use env_logger::Env;
use log::info;
use ranim::glam::{vec2, Vec3};
use ranim::palette::{rgb, Srgba};
use ranim::{
    animation::{fading::Fading, transform::Transform, Animation, AnimationConfig},
    mobject::{
        geometry::{Arc, Polygon},
        ToMobject, TransformAnchor,
    },
    scene::Scene,
    utils::SubpathWidth,
    RanimContext,
};

fn main() {
    #[cfg(debug_assertions)]
    env_logger::Builder::from_env(Env::default().default_filter_or("basic=trace")).init();
    #[cfg(not(debug_assertions))]
    env_logger::Builder::from_env(Env::default().default_filter_or("basic=info")).init();

    let mut ctx = RanimContext::new();

    let mut scene = Scene::new(&ctx);
    let t = Instant::now();

    let mut polygon = Polygon::new(vec![
        vec2(-100.0, 0.0),
        vec2(20.0, 30.0),
        vec2(0.0, 70.0),
        vec2(50.0, 0.0),
    ])
    .with_width(SubpathWidth::Middle(20.0))
    .to_mobject();
    polygon.set_color(Srgba::from_u32::<rgb::channels::Rgba>(0xE65A4CFF).into());
    polygon.rotate(
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

    let mut arc = Arc::new(std::f32::consts::PI / 2.0)
        .with_radius(100.0)
        .with_stroke_width(SubpathWidth::Middle(20.0))
        .to_mobject();
    arc.set_color(Srgba::from_u32::<rgb::channels::Rgba>(0x29ABCAFF).into());

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

    info!("Rendered {} frames in {:?}", scene.frame_count, t.elapsed());
}
