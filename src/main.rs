use env_logger::Env;
use glam::dvec2;
use ranim::{
    camera::Camera,
    mobject::{geometry::Polygon, Mobject},
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
    let mobject = Mobject::from_pipeline_vertex(&ctx.wgpu_ctx, polygon);

    let mut texture_data = vec![0; size.0 * size.1 * 4];
    camera.render(&mut ctx, &mut texture_data, &mobject);

    use image::{ImageBuffer, Rgba};
    let buffer =
        ImageBuffer::<Rgba<u8>, _>::from_raw(size.0 as u32, size.1 as u32, texture_data).unwrap();
    buffer.save("image.png").unwrap();
}
fn main() {
    println!("Hello, world!");
    pollster::block_on(run())
}
