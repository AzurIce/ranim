use std::f64::consts::{PI, TAU};

use ranim::{
    color::palettes::manim,
    glam::{DMat4, DVec3},
    items::mesh::{Sphere, Surface},
    prelude::*,
    utils::rate_functions::linear,
};
use ranim_anims::fading::FadingAnim;
use ranim_core::animation::Eval;
use ranim_items::vitem::{VItem, text::TextItem};

// Custom animation: orbital motion around the origin in the XY plane
struct OrbitMotion {
    src: Surface,
    orbit_radius: f64,
    initial_angle: f64,
    total_angle: f64,
}

impl Eval<Surface> for OrbitMotion {
    fn eval_alpha(&self, alpha: f64) -> Surface {
        let angle = self.initial_angle + self.total_angle * alpha;
        let x = self.orbit_radius * angle.cos();
        let y = self.orbit_radius * angle.sin();
        let mut result = self.src.clone();
        result.transform = DMat4::from_translation(DVec3::new(x, y, 0.0));
        result
    }
}

// Custom animation: orbital motion for text labels (simple version)
struct LabelOrbitMotion {
    src: Vec<VItem>,
    orbit_radius: f64,
    initial_angle: f64,
    total_angle: f64,
    z_offset: f64,
}

impl Eval<Vec<VItem>> for LabelOrbitMotion {
    fn eval_alpha(&self, alpha: f64) -> Vec<VItem> {
        let angle = self.initial_angle + self.total_angle * alpha;
        let x = self.orbit_radius * angle.cos();
        let y = self.orbit_radius * angle.sin();
        let mut result = self.src.clone();
        result.move_to(DVec3::new(x, y, self.z_offset));
        result
    }
}

// Planet data structure
struct PlanetData {
    name: &'static str,
    radius: f64,
    orbit_radius: f64,
    color: ranim::color::AlphaColor<ranim::color::Srgb>,
    period: f64,
    initial_angle: f64,
    has_atmosphere: bool, // Whether the planet has a visible atmosphere
}

const PLANETS: &[PlanetData] = &[
    PlanetData {
        name: "Mercury",
        radius: 0.3,
        orbit_radius: 4.0,
        color: manim::GREY_C,
        period: 2.0,
        initial_angle: 0.0,
        has_atmosphere: false, // No atmosphere
    },
    PlanetData {
        name: "Venus",
        radius: 0.5,
        orbit_radius: 6.0,
        color: manim::GOLD_E,
        period: 3.0,
        initial_angle: PI / 4.0,
        has_atmosphere: true, // Thick atmosphere
    },
    PlanetData {
        name: "Earth",
        radius: 0.5,
        orbit_radius: 8.0,
        color: manim::BLUE_C,
        period: 4.0,
        initial_angle: PI / 2.0,
        has_atmosphere: true, // Atmosphere
    },
    PlanetData {
        name: "Mars",
        radius: 0.4,
        orbit_radius: 10.0,
        color: manim::RED_C,
        period: 5.0,
        initial_angle: PI,
        has_atmosphere: false, // Thin atmosphere (negligible)
    },
    PlanetData {
        name: "Jupiter",
        radius: 1.2,
        orbit_radius: 14.0,
        color: manim::ORANGE,
        period: 8.0,
        initial_angle: 3.0 * PI / 2.0,
        has_atmosphere: true, // Gas giant
    },
    PlanetData {
        name: "Saturn",
        radius: 1.0,
        orbit_radius: 17.0,
        color: manim::GOLD_C,
        period: 10.0,
        initial_angle: TAU / 3.0,
        has_atmosphere: true, // Gas giant
    },
];

#[scene]
#[output(dir = "./output/solar_system")]
fn solar_system(r: &mut RanimScene) {
    // Setup camera looking down from above (top-down view)
    let phi = 50.0f64.to_radians(); // Small angle from +Z axis (almost straight down)
    let theta = -PI / 2.0;
    let distance = 60.0;

    let mut cam = CameraFrame::from_spherical(phi, theta, distance);
    cam.fovy = 30.0f64.to_radians();
    let total_secs = 12.0;
    let mut content = AnimStack::new();

    let mut camera = AnimSequence::new();
    camera.push(cam.show()).hold_to(total_secs);
    content.push(camera);

    // Create the Sun at the origin (already visible at start)
    let sun = Surface::from(
        Sphere::new(2.0)
            .with_resolution((30, 15))
            .with_fill_color(manim::YELLOW_C.with_alpha(0.5)),
    );
    let mut sun_sequence = AnimSequence::new();
    sun_sequence.push(sun.show()).hold_to(total_secs);
    content.push(sun_sequence);

    // Orbit rings fade in (0-1s)
    for planet in PLANETS {
        let major_radius = planet.orbit_radius;
        let minor_radius = 0.05;
        let resolution = (128, 16);

        let mut orbit_torus = Surface::from_uv_func(
            |u, v| {
                let u_angle = u * TAU;
                let v_angle = v * TAU;
                DVec3::new(
                    (major_radius + minor_radius * v_angle.cos()) * u_angle.cos(),
                    (major_radius + minor_radius * v_angle.cos()) * u_angle.sin(),
                    minor_radius * v_angle.sin(),
                )
            },
            (0.0, 1.0),
            (0.0, 1.0),
            resolution,
        );

        let color = manim::GREY_C.with_alpha(0.3);
        orbit_torus.vertex_colors = vec![color; orbit_torus.vertices.len()];

        let mut orbit_sequence = AnimSequence::new();
        orbit_sequence
            .push(orbit_torus.fade_in().with_duration(1.0))
            .hold_to(total_secs);
        content.push(orbit_sequence);
    }

    // Timeline:
    // t=0s:   Sun + planets visible (static)
    // t=0-1s: Orbit rings fade in
    // t=1-2s: All text labels fade in
    // t=2-12s: Planets + labels start orbiting together

    // Create planets (visible at start, orbit starts at t=2s)
    for planet in PLANETS {
        let x = planet.orbit_radius * planet.initial_angle.cos();
        let y = planet.orbit_radius * planet.initial_angle.sin();

        if planet.has_atmosphere {
            // Planets with atmosphere: solid core + transparent atmosphere layer

            // Core (solid, opaque)
            let mut core = Surface::from(
                Sphere::new(planet.radius * 0.9)
                    .with_resolution((20, 10))
                    .with_fill_color(planet.color.with_alpha(1.0)),
            );
            core.transform = DMat4::from_translation(DVec3::new(x, y, 0.0));

            // Atmosphere (larger, semi-transparent)
            let mut atmosphere = Surface::from(
                Sphere::new(planet.radius)
                    .with_resolution((20, 10))
                    .with_fill_color(planet.color.with_alpha(0.3)),
            )
            .with_smooth_normals();
            atmosphere.transform = DMat4::from_translation(DVec3::new(x, y, 0.0));

            let orbit_total_angle = TAU * (10.0 / planet.period);

            let mut core_sequence = AnimSequence::new();
            core_sequence.push(core.show()).hold(2.0).push(
                OrbitMotion {
                    src: core.clone(),
                    orbit_radius: planet.orbit_radius,
                    initial_angle: planet.initial_angle,
                    total_angle: orbit_total_angle,
                }
                .into_animation_cell()
                .apply_to(&mut core)
                .with_duration(10.0)
                .with_rate_func(linear),
            );
            content.push(core_sequence);

            let mut atmosphere_sequence = AnimSequence::new();
            atmosphere_sequence.push(atmosphere.show()).hold(2.0).push(
                OrbitMotion {
                    src: atmosphere.clone(),
                    orbit_radius: planet.orbit_radius,
                    initial_angle: planet.initial_angle,
                    total_angle: orbit_total_angle,
                }
                .into_animation_cell()
                .apply_to(&mut atmosphere)
                .with_duration(10.0)
                .with_rate_func(linear),
            );
            content.push(atmosphere_sequence);
        } else {
            // Planets without atmosphere: single solid sphere
            let mut planet_surface = Surface::from(
                Sphere::new(planet.radius)
                    .with_resolution((20, 10))
                    .with_fill_color(planet.color.with_alpha(1.0)),
            )
            .with_smooth_normals();
            planet_surface.transform = DMat4::from_translation(DVec3::new(x, y, 0.0));

            let orbit_total_angle = TAU * (10.0 / planet.period);

            let mut planet_sequence = AnimSequence::new();
            planet_sequence.push(planet_surface.show()).hold(2.0).push(
                OrbitMotion {
                    src: planet_surface.clone(),
                    orbit_radius: planet.orbit_radius,
                    initial_angle: planet.initial_angle,
                    total_angle: orbit_total_angle,
                }
                .into_animation_cell()
                .apply_to(&mut planet_surface)
                .with_duration(10.0)
                .with_rate_func(linear),
            );
            content.push(planet_sequence);
        }

        // Create planet label
        let label_z_offset = planet.radius + 0.5;
        let label = TextItem::new(planet.name, 0.6).with(|item| {
            item.move_to(DVec3::new(x, y, label_z_offset))
                .with_origin(AabbPoint::CENTER, |x| {
                    x.rotate_on_x(30.0f64.to_radians()).discard()
                })
                .discard()
        });

        let mut label_vitems = Vec::<VItem>::from(label);

        let mut label_sequence = AnimSequence::new();
        label_sequence
            .forward(1.0)
            .push(label_vitems.clone().fade_in().with_duration(1.0));

        // Labels orbit together with planets (starts at t=2s, runs for 10s)
        let orbit_total_angle = TAU * (10.0 / planet.period);
        label_sequence.push(
            LabelOrbitMotion {
                src: label_vitems.clone(),
                orbit_radius: planet.orbit_radius,
                initial_angle: planet.initial_angle,
                total_angle: orbit_total_angle,
                z_offset: label_z_offset,
            }
            .into_animation_cell()
            .apply_to(&mut label_vitems)
            .with_duration(10.0)
            .with_rate_func(linear),
        );
        content.push(label_sequence);
    }

    r.play(content);

    // Capture preview at the midpoint
    r.insert_time_mark(6.0, TimeMark::Capture("preview.png".to_string()));
}
