use std::f64::consts::PI;

use ranim::{
    color::palettes::manim, glam::DVec3, items::mesh::Surface, prelude::*,
    utils::rate_functions::linear,
};

// --- Improved Perlin Noise (ported from Python) ---

#[rustfmt::skip]
const P: [usize; 256] = [
    151,160,137, 91, 90, 15,131, 13,201, 95, 96, 53,194,233,  7,225,
    140, 36,103, 30, 69,142,  8, 99, 37,240, 21, 10, 23,190,  6,148,
    247,120,234, 75,  0, 26,197, 62, 94,252,219,203,117, 35, 11, 32,
     57,177, 33, 88,237,149, 56, 87,174, 20,125,136,171,168, 68,175,
     74,165, 71,134,139, 48, 27,166, 77,146,158,231, 83,111,229,122,
     60,211,133,230,220,105, 92, 41, 55, 46,245, 40,244,102,143, 54,
     65, 25, 63,161,  1,216, 80, 73,209, 76,132,187,208, 89, 18,169,
    200,196,135,130,116,188,159, 86,164,100,109,198,173,186,  3, 64,
     52,217,226,250,124,123,  5,202, 38,147,118,126,255, 82, 85,212,
    207,206, 59,227, 47, 16, 58, 17,182,189, 28, 42,223,183,170,213,
    119,248,152,  2, 44,154,163, 70,221,153,101,155,167, 43,172,  9,
    129, 22, 39,253, 19, 98,108,110, 79,113,224,232,178,185,112,104,
    218,246, 97,228,251, 34,242,193,238,210,144, 12,191,179,162,241,
     81, 51,145,235,249, 14,239,107, 49,192,214, 31,181,199,106,157,
    184, 84,204,176,115,121, 50, 45,127,  4,150,254,138,236,205, 93,
    222,114, 67, 29, 24, 72,243,141,128,195, 78, 66,215, 61,156,180,
];

fn perm(i: usize) -> usize {
    P[i & 255]
}

fn fade(t: f64) -> f64 {
    t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
}

fn fade_d(t: f64) -> f64 {
    30.0 * t * t * (t * (t - 2.0) + 1.0)
}

fn grad(hash: usize, x: f64, y: f64) -> f64 {
    const SQRT5_INV: f64 = 0.4472135954999579; // 1/sqrt(5)
    const TABLE: [(f64, f64); 8] = [
        (1.0, 2.0),
        (1.0, -2.0),
        (-1.0, 2.0),
        (-1.0, -2.0),
        (2.0, 1.0),
        (2.0, -1.0),
        (-2.0, 1.0),
        (-2.0, -1.0),
    ];
    let (gx, gy) = TABLE[hash & 7];
    (x * gx + y * gy) * SQRT5_INV
}

fn noise(x: f64, y: f64) -> f64 {
    let xi = x.floor() as i64 as usize;
    let yi = y.floor() as i64 as usize;

    let xf = x - x.floor();
    let yf = y - y.floor();

    let u = fade(xf);
    let v = fade(yf);

    let a = perm(xi) + (yi & 255);
    let b = perm(xi.wrapping_add(1)) + (yi & 255);
    let p0 = grad(perm(a), xf, yf);
    let p1 = grad(perm(b), xf - 1.0, yf);
    let p2 = grad(perm(a + 1), xf, yf - 1.0);
    let p3 = grad(perm(b + 1), xf - 1.0, yf - 1.0);

    fn lerp(t: f64, a: f64, b: f64) -> f64 {
        a + t * (b - a)
    }

    lerp(v, lerp(u, p0, p1), lerp(u, p2, p3))
}

fn noise_with_derivative(x: f64, y: f64) -> (f64, f64, f64) {
    let xi = x.floor() as i64 as usize;
    let yi = y.floor() as i64 as usize;

    let xf = x - x.floor();
    let yf = y - y.floor();

    let u = fade(xf);
    let v = fade(yf);

    let a = perm(xi) + (yi & 255);
    let b = perm(xi.wrapping_add(1)) + (yi & 255);
    let p0 = grad(perm(a), xf, yf);
    let p1 = grad(perm(b), xf - 1.0, yf);
    let p2 = grad(perm(a + 1), xf, yf - 1.0);
    let p3 = grad(perm(b + 1), xf - 1.0, yf - 1.0);

    fn lerp(t: f64, a: f64, b: f64) -> f64 {
        a + t * (b - a)
    }

    let n = lerp(v, lerp(u, p0, p1), lerp(u, p2, p3));
    let dndx = ((p1 - p0) + (p3 - p2 - p1 + p0) * v) * fade_d(xf);
    let dndy = ((p2 - p0) + (p3 - p2 - p1 + p0) * u) * fade_d(yf);

    (n, dndx, dndy)
}

fn fractal_noise(x: f64, y: f64, octaves: u32, persistence: f64) -> f64 {
    let mut total = 0.0;
    let mut frequency = 1.0;
    let mut amplitude = 1.0;
    let mut max_value = 0.0;

    for _ in 0..octaves {
        total += noise(x * frequency, y * frequency) * amplitude;
        max_value += amplitude;
        amplitude *= persistence;
        frequency *= 2.0;
    }

    total / max_value
}

fn fractal_with_derivative_noise(x: f64, y: f64, octaves: u32, persistence: f64) -> f64 {
    let mut total = 0.0;
    let mut frequency = 1.0;
    let mut amplitude = 1.0;
    let mut max_value = 0.0;
    let mut dx = 0.0;
    let mut dy = 0.0;

    for _ in 0..octaves {
        let (n, dndx, dndy) = noise_with_derivative(x * frequency, y * frequency);
        dx += dndx * amplitude;
        dy += dndy * amplitude;
        let factor = n / (1.0 + dx * dx + dy * dy);
        total += amplitude * factor;
        max_value += amplitude;
        amplitude *= persistence;
        frequency *= 2.0;
    }

    total / max_value
}

// --- Constants (matching Python cg-report) ---

const GRID_SIZE: f64 = 32.0;
const LATTICE_CNT: f64 = 8.0;
const DEPTH: f64 = 3.0;
const RESOLUTION: u32 = 512;

const U_MIN: f64 = 0.0;
const U_MAX: f64 = 30.0;
const CENTER_OFFSET: f64 = 16.0;

// Height-based color scale matching the Python reference.
// Python: [(c, x * depth) for c, x in [...]]
// Colors map to the Z-coordinate of the vertex (raw height, no scaling).
#[allow(clippy::neg_multiply)]
fn terrain_colorscale() -> Vec<(ranim::color::AlphaColor<ranim::color::Srgb>, f64)> {
    vec![
        (manim::BLUE_E, -1.0 * DEPTH),
        (manim::BLUE_E, -0.9 * DEPTH),
        (manim::BLUE_C, -0.8 * DEPTH),
        (manim::GOLD_E, -0.7 * DEPTH),
        (manim::GREY_BROWN, -0.1 * DEPTH),
        (manim::GREY_BROWN, 0.1 * DEPTH),
        (manim::GREY_D, 0.25 * DEPTH),
        (manim::GREY_C, 0.6 * DEPTH),
        (manim::WHITE, 0.7 * DEPTH),
        (manim::WHITE, 1.0 * DEPTH),
    ]
}

// --- Helper to build a terrain scene ---

fn build_terrain_scene(r: &mut RanimScene, height_func: impl Fn(f64, f64) -> f64) {
    let phi = 70.0 * PI / 180.0;
    let theta = 30.0 * PI / 180.0;
    let distance = 22.0;

    let mut cam = CameraFrame::from_spherical(phi, theta, distance);
    cam.fovy = 50.0 * PI / 180.0;
    let r_cam = r.insert(cam.clone());

    let colorscale = terrain_colorscale();

    // Z-up: x = u - offset, y = v - offset, z = height
    let terrain = Surface::from_uv_func(
        |u, v| {
            let x = u - CENTER_OFFSET;
            let y = v - CENTER_OFFSET;
            let z = height_func(u, v);
            DVec3::new(x, y, z)
        },
        (U_MIN, U_MAX),
        (U_MIN, U_MAX),
        (RESOLUTION, RESOLUTION),
    )
    .with_fill_by_z(&colorscale);

    let _r_terrain = r.insert(terrain);

    r.timeline_mut(r_cam).play(
        cam.orbit(DVec3::ZERO, 2.0)
            .with_duration(5.0)
            .with_rate_func(linear),
    );
}

// --- Scenes ---

/// Basic Perlin noise terrain.
#[scene]
#[output(dir = "./output/perlin_terrain/perlin")]
fn perlin(r: &mut RanimScene) {
    // Python: get_noise(x, y, size) → noise(v, u) (note v, u swap)
    // height = noise_func(v, u, size) * depth = noise * depth
    build_terrain_scene(r, |u, v| {
        let nx = v / GRID_SIZE * LATTICE_CNT;
        let ny = u / GRID_SIZE * LATTICE_CNT;
        noise(nx, ny) * DEPTH
    });
    r.insert_time_mark(0.0, TimeMark::Capture("preview-perlin.png".to_string()));
}

/// Fractal Perlin noise terrain (octave stacking).
#[scene]
#[output(dir = "./output/perlin_terrain/fractal")]
fn fractal_perlin(r: &mut RanimScene) {
    // Python: get_fractal_noise(x, y, size) * depth → fractal_noise(v, u) * depth
    // height = noise_func(v, u, size) * depth = fractal_noise * depth * depth
    build_terrain_scene(r, |u, v| {
        let nx = v / GRID_SIZE * LATTICE_CNT;
        let ny = u / GRID_SIZE * LATTICE_CNT;
        fractal_noise(nx, ny, 8, 0.5) * DEPTH * DEPTH
    });
    r.insert_time_mark(
        0.0,
        TimeMark::Capture("preview-fractal-perlin.png".to_string()),
    );
}

/// Fractal Perlin noise with derivative-based erosion.
#[scene(name = "perlin_terrain")]
#[output(dir = "./output/perlin_terrain/erosion")]
fn fractal_erosion(r: &mut RanimScene) {
    // Same pattern as fractal_perlin but with derivative-based erosion
    build_terrain_scene(r, |u, v| {
        let nx = v / GRID_SIZE * LATTICE_CNT;
        let ny = u / GRID_SIZE * LATTICE_CNT;
        fractal_with_derivative_noise(nx, ny, 8, 0.5) * DEPTH * DEPTH
    });
    r.insert_time_mark(
        0.0,
        TimeMark::Capture("preview-fractal-erosion-perlin.png".to_string()),
    );
}
