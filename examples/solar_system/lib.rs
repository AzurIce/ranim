use std::f64::consts::{PI, TAU};

use ranim::{
    color::palettes::manim,
    glam::{DMat4, DVec3},
    items::mesh::{Sphere, Surface},
    prelude::*,
    utils::rate_functions::{linear, smooth},
};
use ranim_anims::pure::fading::FadingAnim;
use ranim_items::vitem::{VItem, text::TextItem};

const INTRO_SECS: f64 = 2.0;
const LABEL_DELAY_SECS: f64 = 1.0;
const LABEL_FADE_SECS: f64 = 1.0;
const ORBIT_SECS: f64 = 10.0;
const TOTAL_SECS: f64 = INTRO_SECS + ORBIT_SECS;

/// Something that can be positioned on an orbit.
///
/// The same orbital evaluator can therefore animate both mesh surfaces and
/// vector-item groups without erasing either output type.
trait OrbitItem: Clone {
    fn set_orbit_position(&mut self, position: DVec3);
}

impl OrbitItem for Surface {
    fn set_orbit_position(&mut self, position: DVec3) {
        self.transform = DMat4::from_translation(position);
    }
}

impl OrbitItem for Vec<VItem> {
    fn set_orbit_position(&mut self, position: DVec3) {
        self.move_to(position);
    }
}

/// A reusable orbital evaluator whose output keeps its concrete item type.
struct OrbitMotion<T> {
    src: T,
    orbit_radius: f64,
    initial_angle: f64,
    total_angle: f64,
    z: f64,
}

impl<T: OrbitItem> Eval for OrbitMotion<T> {
    type Output = T;

    fn eval_alpha(&self, alpha: f64) -> Self::Output {
        let angle = self.initial_angle + self.total_angle * alpha;
        let position = DVec3::new(
            self.orbit_radius * angle.cos(),
            self.orbit_radius * angle.sin(),
            self.z,
        );
        let mut result = self.src.clone();
        result.set_orbit_position(position);
        result
    }
}

struct PlanetData {
    name: &'static str,
    radius: f64,
    orbit_radius: f64,
    color: ranim::color::AlphaColor<ranim::color::Srgb>,
    period: f64,
    initial_angle: f64,
    has_atmosphere: bool,
}

const PLANETS: &[PlanetData] = &[
    PlanetData {
        name: "Mercury",
        radius: 0.3,
        orbit_radius: 4.0,
        color: manim::GREY_C,
        period: 2.0,
        initial_angle: 0.0,
        has_atmosphere: false,
    },
    PlanetData {
        name: "Venus",
        radius: 0.5,
        orbit_radius: 6.0,
        color: manim::GOLD_E,
        period: 3.0,
        initial_angle: PI / 4.0,
        has_atmosphere: true,
    },
    PlanetData {
        name: "Earth",
        radius: 0.5,
        orbit_radius: 8.0,
        color: manim::BLUE_C,
        period: 4.0,
        initial_angle: PI / 2.0,
        has_atmosphere: true,
    },
    PlanetData {
        name: "Mars",
        radius: 0.4,
        orbit_radius: 10.0,
        color: manim::RED_C,
        period: 5.0,
        initial_angle: PI,
        has_atmosphere: false,
    },
    PlanetData {
        name: "Jupiter",
        radius: 1.2,
        orbit_radius: 14.0,
        color: manim::ORANGE,
        period: 8.0,
        initial_angle: 3.0 * PI / 2.0,
        has_atmosphere: true,
    },
    PlanetData {
        name: "Saturn",
        radius: 1.0,
        orbit_radius: 17.0,
        color: manim::GOLD_C,
        period: 10.0,
        initial_angle: TAU / 3.0,
        has_atmosphere: true,
    },
];

/// Build the typed orbital evaluator reused by every planet layer and label.
fn orbit<T: OrbitItem>(item: &mut T, planet: &PlanetData, z: f64) -> OrbitMotion<T> {
    OrbitMotion {
        src: item.clone(),
        orbit_radius: planet.orbit_radius,
        initial_angle: planet.initial_angle,
        total_angle: TAU * (ORBIT_SECS / planet.period),
        z,
    }
    .apply_to(item)
}

/// One body layer: stay visible through the intro, then enter the shared orbit.
fn body_layer(mut surface: Surface, planet: &PlanetData) -> AnimSequence {
    seq![
        surface.show().with_duration(INTRO_SECS),
        orbit(&mut surface, planet, 0.0)
            .with_duration(ORBIT_SECS)
            .with_rate_func(linear),
    ]
}

/// A label has its own intro phrase, followed by the same reusable orbit phase.
fn planet_label(planet: &PlanetData, position: DVec3) -> AnimSequence {
    let label_z = planet.radius + 0.5;
    let label = TextItem::new(planet.name, 0.6).with(|item| {
        item.move_to(position.with_z(label_z))
            .with_origin(AabbPoint::CENTER, |item| {
                item.rotate_on_x(30.0f64.to_radians()).discard()
            })
            .discard()
    });
    let mut label = Vec::<VItem>::from(label);

    let mut sequence = AnimSequence::new();
    sequence
        .forward(LABEL_DELAY_SECS)
        .push(
            label
                .fade_in()
                .with_duration(LABEL_FADE_SECS)
                .with_rate_func(smooth),
        )
        .push(
            orbit(&mut label, planet, label_z)
                .with_duration(ORBIT_SECS)
                .with_rate_func(linear),
        );
    sequence
}

/// Build a planet as a nested stack of body layers and its label track.
///
/// Atmospheric planets reuse the same body-layer phrase twice; rocky planets
/// use it once. The outer stack keeps each planet as one semantic Timeline node.
fn planet_system(planet: &PlanetData) -> AnimStack {
    let position = DVec3::new(
        planet.orbit_radius * planet.initial_angle.cos(),
        planet.orbit_radius * planet.initial_angle.sin(),
        0.0,
    );
    let mut body_layers = AnimStack::new();

    if planet.has_atmosphere {
        let mut core = Surface::from(
            Sphere::new(planet.radius * 0.9)
                .with_resolution((20, 10))
                .with_fill_color(planet.color.with_alpha(1.0)),
        );
        core.transform = DMat4::from_translation(position);
        body_layers.push(body_layer(core, planet));

        let mut atmosphere = Surface::from(
            Sphere::new(planet.radius)
                .with_resolution((20, 10))
                .with_fill_color(planet.color.with_alpha(0.3)),
        )
        .with_smooth_normals();
        atmosphere.transform = DMat4::from_translation(position);
        body_layers.push(body_layer(atmosphere, planet));
    } else {
        let mut surface = Surface::from(
            Sphere::new(planet.radius)
                .with_resolution((20, 10))
                .with_fill_color(planet.color.with_alpha(1.0)),
        )
        .with_smooth_normals();
        surface.transform = DMat4::from_translation(position);
        body_layers.push(body_layer(surface, planet));
    }

    stack![body_layers, planet_label(planet, position)]
}

/// Fade one orbit ring in, then hold its final state for the full scene.
fn orbit_ring(planet: &PlanetData) -> AnimSequence {
    let major_radius = planet.orbit_radius;
    let minor_radius = 0.05;
    let mut ring = Surface::from_uv_func(
        move |u, v| {
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
        (128, 16),
    );
    let color = manim::GREY_C.with_alpha(0.3);
    ring.vertex_colors = vec![color; ring.vertices.len()];

    let mut sequence = seq![ring.fade_in().with_duration(1.0).with_rate_func(smooth),];
    sequence.hold_to(TOTAL_SECS);
    sequence
}

fn orbit_rings() -> AnimStack {
    let mut rings = AnimStack::new();
    for planet in PLANETS {
        rings.push(orbit_ring(planet));
    }
    rings
}

fn planetary_systems() -> AnimStack {
    let mut planets = AnimStack::new();
    for planet in PLANETS {
        planets.push(planet_system(planet));
    }
    planets
}

#[scene]
#[output(dir = "./output/solar_system")]
fn solar_system(r: &mut RanimScene) {
    let phi = 50.0f64.to_radians();
    let theta = -PI / 2.0;
    let distance = 60.0;
    let mut camera = CameraFrame::from_spherical(phi, theta, distance);
    camera.fovy = 30.0f64.to_radians();

    let sun = Surface::from(
        Sphere::new(2.0)
            .with_resolution((30, 15))
            .with_fill_color(manim::YELLOW_C.with_alpha(0.5)),
    );
    // Stack -> Stack -> Sequence: orbit rings and reusable planet systems
    // remain visible as semantic groups in the preview Timeline.
    r.play(camera.show().with_duration(TOTAL_SECS));
    r.play(stack![
        sun.show().with_duration(TOTAL_SECS),
        orbit_rings(),
        planetary_systems(),
    ]);
    r.insert_time_mark(6.0, TimeMark::Capture("preview.png".to_owned()));
}
