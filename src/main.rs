use env_logger::Env;
use glam::dvec2;
use ranim::{camera::Camera, mobject::Polygon, Renderable, WgpuBuffer, WgpuContext};

async fn run() {
    env_logger::Builder::from_env(Env::default().default_filter_or("ranim=trace")).init();

    let ctx = WgpuContext::new().await;

    let size = (1280, 720);
    let mut camera = Camera::new(&ctx, size.0, size.1);
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
    let data = polygon.vertex_data();
    // println!("{:?}", data);
    // context setup done

    let vertex_buffer = WgpuBuffer::new_init(&ctx, &data, wgpu::BufferUsages::VERTEX);

    let mut texture_data = vec![0; size.0 * size.1 * 4];
    camera.render(&ctx, &vertex_buffer, &mut texture_data);

    use image::{ImageBuffer, Rgba};
    let buffer =
        ImageBuffer::<Rgba<u8>, _>::from_raw(size.0 as u32, size.1 as u32, texture_data).unwrap();
    buffer.save("image.png").unwrap();
}
fn main() {
    println!("Hello, world!");
    pollster::block_on(run())
}
