use std::time::Instant;

use env_logger::Env;
use glam::{dvec2, vec3, Vec3};
use log::info;
use ranim::{
    camera::Camera, mobject::{geometry::Polygon, Mobject, TransformAnchor}, scene::Scene, RanimContext
};

async fn run() {
    env_logger::Builder::from_env(Env::default().default_filter_or("ranim=trace")).init();

    let mut ctx = RanimContext::new();

    let mut scene = Scene::new(&ctx.wgpu_ctx);
    // let size = (1280, 720);
    // let mut camera = Camera::new(&ctx.wgpu_ctx, size.0, size.1);
    // camera.frame.set_fovy(std::f32::consts::PI / 2.0);

    // let data = SimpleVertex::test_data();
    // let arc = Arc {angle: std::f64::consts::PI / 2.0 };
    // let data = arc.vertex_data();
    let polygon = Polygon::from_verticies(vec![
        dvec2(-100.0, 0.0),
        dvec2(20.0, 30.0),
        dvec2(0.0, 70.0),
        dvec2(50.0, 0.0),
    ]);
    let t = Instant::now();
    let mut mobject = Mobject::from_pipeline_vertex(&ctx.wgpu_ctx, polygon);
    scene.add_mobject(&mobject);
    scene.render_to_image(&mut ctx, "image1.png");

    mobject.shift(vec3(100.0, 0.0, 0.0));
    scene.add_mobject(&mobject);
    scene.render_to_image(&mut ctx, "image2.png");

    mobject.scale(vec3(2.0, 4.0, 1.0), TransformAnchor::origin());
    scene.add_mobject(&mobject);
    scene.render_to_image(&mut ctx, "image3.png");

    mobject.shift(vec3(-100.0, 0.0, 0.0));
    scene.add_mobject(&mobject);
    scene.render_to_image(&mut ctx, "image4.png");

    mobject.rotate(
        std::f32::consts::PI / 4.0,
        Vec3::Z,
        TransformAnchor::origin(),
    );
    scene.add_mobject(&mobject);
    scene.render_to_image(&mut ctx, "image5.png");

    mobject.shift(vec3(0.0, 50.0, 0.0));
    scene.add_mobject(&mobject);
    scene.render_to_image(&mut ctx, "image6.png");
    info!("Total Time: {:?}", t.elapsed());
}
fn main() {
    println!("Hello, world!");
    pollster::block_on(run())
}
