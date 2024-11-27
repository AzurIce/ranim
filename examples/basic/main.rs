use std::time::{Duration, Instant};

use env_logger::Env;
use log::info;
use ranim::glam::{vec2, Vec3};
use ranim::palette::{rgb, Srgba};
use ranim::rabject::vmobject::TransformAnchor;
use ranim::rabject::Blueprint;
use ranim::{
    animation::{fading::Fading, transform::Transform, Animation, AnimationConfig},
    rabject::vmobject::{Arc, Polygon},
    scene::Scene,
    utils::SubpathWidth,
    RanimContext,
};

fn main() {
    #[cfg(debug_assertions)]
    env_logger::Builder::from_env(Env::default().default_filter_or("basic=trace")).init();
    #[cfg(not(debug_assertions))]
    env_logger::Builder::from_env(Env::default().default_filter_or("basic=info,ranim=trace")).init();

    let mut ctx = RanimContext::new();

    let mut scene = Scene::new(&ctx);
    let t = Instant::now();
    info!("running...");

    let mut polygon = Polygon::new(vec![
        vec2(-100.0, -300.0),
        vec2(0.0, 0.0),
        vec2(200.0, 300.0),
        vec2(0.0, 700.0),
        vec2(500.0, 0.0),
    ])
    .with_width(20.0)
    .build();
    polygon
        .set_color(Srgba::from_u32::<rgb::channels::Rgba>(0xE65A4CFF).into())
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
    // scene.insert_rabject(&mut ctx, &polygon);
    // scene.insert_rabject(&mut ctx, &polygon);

    let mut arc = Arc::new(std::f32::consts::PI / 2.0)
        .with_radius(100.0)
        .with_stroke_width(SubpathWidth::Middle(20.0))
        .build();
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
    // scene.render_to_image(&mut ctx, "output.png");
    scene.play(
        &mut ctx,
        Animation::new(arc, Fading::Out, AnimationConfig::default().remove()),
    );

    info!("Rendered {} frames in {:?}", scene.frame_count, t.elapsed());
}
