use std::f64::consts::PI;

use ranim::{
    color::palettes::manim,
    glam::DVec3,
    items::mesh::Sphere,
    prelude::*,
    utils::rate_functions::linear,
};

#[scene]
#[output(dir = "tetrahedron_spheres")]
fn tetrahedron_spheres(r: &mut RanimScene) {
    let phi = 50.0 * PI / 180.0;
    let theta = 0.0;
    let distance = 6.0;

    let mut cam = CameraFrame::from_spherical(phi, theta, distance);
    let r_cam = r.insert(cam.clone());

    // Regular tetrahedron centered at the origin, edge length `a`.
    let a: f64 = 2.5;
    let sqrt6 = 6.0_f64.sqrt();
    let sqrt3 = 3.0_f64.sqrt();

    let vertices = [
        DVec3::new(0.0, 0.0, a * sqrt6 / 4.0),
        DVec3::new(0.0, a * sqrt3 / 3.0, -a * sqrt6 / 12.0),
        DVec3::new(-a / 2.0, -a * sqrt3 / 6.0, -a * sqrt6 / 12.0),
        DVec3::new(a / 2.0, -a * sqrt3 / 6.0, -a * sqrt6 / 12.0),
    ];

    let colors = [
        manim::BLUE_C,
        manim::RED_C,
        manim::GREEN_C,
        manim::YELLOW_C,
    ];

    // Radius chosen so that adjacent spheres overlap nicely.
    let radius = 0.6 * a;
    let resolution = (31, 16);

    for (vertex, color) in vertices.iter().zip(colors.iter()) {
        let sphere = Sphere::new(radius)
            .with_center(*vertex)
            .with_resolution(resolution)
            .with_fill_color(color.with_alpha(0.5));
        let _r_sphere = r.insert(sphere);
    }

    // Camera orbit: full revolution in 8 seconds
    r.timeline_mut(r_cam).play(
        cam.orbit(DVec3::ZERO, 2.0 * PI)
            .with_duration(8.0)
            .with_rate_func(linear),
    );

    r.insert_time_mark(
        r.timelines().max_total_secs(),
        TimeMark::Capture("preview.png".to_string()),
    );
}
