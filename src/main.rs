use env_logger::Env;
use glam::{dvec2, vec3, Vec3};
use ranim::{
    camera::Camera,
    mobject::{geometry::Polygon, Mobject, TransformAnchor},
    RanimContext,
};

async fn run() {
    env_logger::Builder::from_env(Env::default().default_filter_or("ranim=trace")).init();

    let mut ctx = RanimContext::new();

    let size = (1280, 720);
    let mut camera = Camera::new(&ctx.wgpu_ctx, size.0, size.1);
    camera.frame.set_fovy(std::f32::consts::PI / 2.0);

    // let data = SimpleVertex::test_data();
    // let arc = Arc {angle: std::f64::consts::PI / 2.0 };
    // let data = arc.vertex_data();
    let polygon = Polygon::from_verticies(vec![
        dvec2(-100.0, 0.0),
        dvec2(20.0, 30.0),
        dvec2(0.0, 70.0),
        dvec2(50.0, 0.0),
    ]);
    let mut mobject = Mobject::from_pipeline_vertex(&ctx.wgpu_ctx, polygon);

    mobject.shift(vec3(100.0, 0.0, 0.0));
    mobject.scale(vec3(2.0, 4.0, 1.0), TransformAnchor::origin());
    mobject.shift(vec3(-100.0, 0.0, 0.0));
    mobject.rotate(
        std::f32::consts::PI / 4.0,
        Vec3::Z,
        TransformAnchor::origin(),
    );
    mobject.shift(vec3(0.0, 50.0, 0.0));

    let mut texture_data = vec![0; size.0 * size.1 * 4];
    camera.render(&mut ctx, &mut texture_data, &mut mobject);

    use image::{ImageBuffer, Rgba};
    let buffer =
        ImageBuffer::<Rgba<u8>, _>::from_raw(size.0 as u32, size.1 as u32, texture_data).unwrap();
    buffer.save("image.png").unwrap();
}
fn main() {
    println!("Hello, world!");
    pollster::block_on(run())
}
