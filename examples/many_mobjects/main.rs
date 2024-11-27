use std::time::{Duration, Instant};

use env_logger::Env;
use log::info;
use ranim::glam::vec3;
use ranim::palette::{rgb, Srgba};
use ranim::rabject::vmobject::Arc;
use ranim::rabject::Blueprint;
use ranim::{
    animation::{fading::Fading, Animation, AnimationConfig},
    scene::Scene,
    utils::SubpathWidth,
    RanimContext,
};

fn main() {
    #[cfg(debug_assertions)]
    env_logger::Builder::from_env(Env::default().default_filter_or("many_objects=trace")).init();
    #[cfg(not(debug_assertions))]
    env_logger::Builder::from_env(Env::default().default_filter_or("many_mobjects=info")).init();

    let mut ctx = RanimContext::new();

    let mut scene = Scene::new(&ctx);
    let t = Instant::now();

    let start_color: Srgba = Srgba::from_u32::<rgb::channels::Rgba>(0x29ABCAFF).into();
    let start_color = vec3(start_color.red, start_color.green, start_color.blue);
    let end_color: Srgba = Srgba::from_u32::<rgb::channels::Rgba>(0xE65A4CFF).into();
    let end_color = vec3(end_color.red, end_color.green, end_color.blue);
    let nrow = 10;
    let ncol = 10;
    let gap = 10.0;
    let padding = 30.0;
    let step_x =
        (scene.camera.frame.size.0 as f32 - padding * 2.0 - gap * (ncol - 1) as f32) / ncol as f32;
    let step_y =
        (scene.camera.frame.size.1 as f32 - padding * 2.0 - gap * (nrow - 1) as f32) / nrow as f32;

    let frame_start = vec3(
        scene.camera.frame.size.0 as f32,
        scene.camera.frame.size.1 as f32,
        0.0,
    ) / -2.0;
    for i in 0..nrow {
        let t = Instant::now();
        for j in 0..ncol {
            let angle = std::f32::consts::PI * j as f32 / (ncol - 1) as f32 * 360.0 / 180.0;
            let color = start_color.lerp(end_color, i as f32 / (nrow - 1) as f32);
            let offset = vec3(
                j as f32 * step_x + step_x / 2.0 + j as f32 * gap + padding,
                i as f32 * step_y + step_y / 2.0 + i as f32 * gap + padding,
                0.0,
            );
            let mut arc = Arc::new(angle)
                .with_radius(step_y / 2.0)
                .with_stroke_width(10.0 * j as f32)
                .build();

            arc.set_color(Srgba::from_components((color.x, color.y, color.z, 1.0)).into())
                .shift(frame_start + offset);
            // let _ = scene.insert_rabject(&mut ctx, &arc);
            // scene.wait(&mut ctx, Duration::from_secs_f32(0.02));
            scene.play(
                &mut ctx,
                Animation::new(
                    arc,
                    Fading::In,
                    AnimationConfig::default().run_time(Duration::from_secs_f32(0.02)),
                ),
            );
        }
        info!("row [{i}/{nrow}] cost: {:?}", t.elapsed());
    }

    info!(
        "Rendered {} frames({}s) in {:?}",
        scene.frame_count,
        scene.time,
        t.elapsed()
    );
}
