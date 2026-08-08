//! N-body gravitational simulation — a canonical "iterative only" animation:
//! no closed-form solution, chaotic, and inherently stateful across frames.
//!
//! `NBodyState::new(n)` places `n` equal masses on a regular n-gon with
//! tangential velocities (exact circular speed for the relative equilibrium,
//! plus a tiny asymmetry). The logical duration `SIM_SECS` is a scene constant:
//! `with_duration` only stretches playback. Each body leaves a fading trail of
//! recent positions; the `Extract` impl projects bodies and trails into
//! `VItem`s once per frame.
//!
//! Behavior by `n`: n = 3 stays inside the frame and wobbles chaotically for
//! the whole scene; n >= 4 destabilizes and ejects a body (a dramatic finale).

use ranim::{
    anims::iterative::Iterative,
    color::{AlphaColor, Srgb, palettes::manim},
    core::Extract,
    core::core_item::CoreItem,
    core::time::{DeltaTime, Time},
    glam::DVec3,
    items::vitem::{VItem, geometry::Circle},
    prelude::*,
};
use std::f64::consts::PI;

const G: f64 = 8.0;
const TRAIL_SAMPLE_EVERY: usize = 4; // sample a trail point every 4th step (~30 Hz)
const TRAIL_LEN: usize = 90; // ~3 s of trail per body
const SIM_SECS: f64 = 32.0;

const PALETTE: [AlphaColor<Srgb>; 6] = [
    manim::BLUE_C,
    manim::RED_C,
    manim::YELLOW_C,
    manim::GREEN_C,
    manim::PURPLE_C,
    manim::ORANGE,
];

#[derive(Clone, Copy)]
struct Body {
    pos: DVec3,
    vel: DVec3,
    mass: f64,
}

/// The N-body system's full state.
#[derive(Clone)]
struct NBodyState {
    bodies: Vec<Body>,
    trails: Vec<Vec<DVec3>>,
    step_count: usize,
    colors: Vec<AlphaColor<Srgb>>,
}

impl NBodyState {
    /// Place `n` equal masses on a regular n-gon.
    ///
    /// The circular speed is exact for the regular n-gon relative equilibrium:
    /// `v² = G·Σ csc(πj/n) / (4R)`. A 4% overspeed plus a tiny asymmetry keeps
    /// the system lively (and chaotic — the n-gon equilibrium is unstable).
    fn new(n: usize) -> Self {
        assert!(n >= 2, "nbody needs at least 2 bodies");
        let radius = 2.0;
        let csc_sum: f64 = (1..n).map(|j| 1.0 / (PI * j as f64 / n as f64).sin()).sum();
        let v_circ = (G * csc_sum / (4.0 * radius)).sqrt();
        let v0 = 1.04 * v_circ;

        let mut bodies = Vec::with_capacity(n);
        for i in 0..n {
            let angle = PI * 2.0 * i as f64 / n as f64;
            let pos = DVec3::new(radius * angle.cos(), radius * angle.sin(), 0.0);
            let velocity = v0 * (1.0 + 0.001 * i as f64);
            let vel = DVec3::new(-velocity * angle.sin(), velocity * angle.cos(), 0.0);
            bodies.push(Body {
                pos,
                vel,
                mass: 1.0,
            });
        }

        let colors = (0..n).map(|i| PALETTE[i % PALETTE.len()]).collect();
        Self {
            bodies,
            trails: vec![Vec::new(); n],
            step_count: 0,
            colors,
        }
    }

    fn accelerations(&self) -> Vec<DVec3> {
        let mut accs = vec![DVec3::ZERO; self.bodies.len()];
        for (i, acc) in accs.iter_mut().enumerate() {
            for j in 0..self.bodies.len() {
                if i == j {
                    continue;
                }
                let r = self.bodies[j].pos - self.bodies[i].pos;
                let d2 = r.length_squared();
                let d = d2.sqrt();
                *acc += r * (G * self.bodies[j].mass / (d2 * d));
            }
        }
        accs
    }

    fn step(&mut self, dt: f64) {
        // Velocity Verlet (symplectic): conserves energy far better than
        // semi-implicit Euler for gravitational orbits.
        let a0 = self.accelerations();
        for (i, body) in self.bodies.iter_mut().enumerate() {
            body.pos += body.vel * dt + a0[i] * (0.5 * dt * dt);
        }
        let a1 = self.accelerations();
        for (i, body) in self.bodies.iter_mut().enumerate() {
            body.vel += (a0[i] + a1[i]) * (0.5 * dt);
        }

        self.step_count += 1;
        if self.step_count.is_multiple_of(TRAIL_SAMPLE_EVERY) {
            for (i, trail) in self.trails.iter_mut().enumerate() {
                trail.push(self.bodies[i].pos);
                if trail.len() > TRAIL_LEN {
                    trail.remove(0);
                }
            }
        }
    }
}

impl Extract for NBodyState {
    type Target = CoreItem;

    fn extract_into(&self, buf: &mut Vec<CoreItem>) {
        // Trails: dim dots at recent positions.
        for (i, trail) in self.trails.iter().enumerate() {
            for &p in trail {
                let mut dot = VItem::from(Circle::new(0.02));
                dot.set_fill_color(self.colors[i]);
                dot.set_fill_opacity(0.15);
                dot.set_stroke_opacity(0.0);
                dot.move_to(p);
                dot.extract_into(buf);
            }
        }
        // Bodies.
        for (i, body) in self.bodies.iter().enumerate() {
            let mut ball = VItem::from(Circle::new(0.32));
            ball.set_fill_color(self.colors[i]);
            ball.set_stroke_opacity(0.0);
            ball.move_to(body.pos);
            ball.extract_into(buf);
        }
    }
}

#[scene]
#[output(dir = "./output/nbody")]
fn nbody(r: &mut RanimScene) {
    r.play(CameraFrame::default().show().with_duration(SIM_SECS));
    // n = 3: chaotic wobble that stays in frame; n >= 4: ejection finale.
    r.play(
        Iterative::from_fn(
            NBodyState::new(99),
            |state: &mut NBodyState, _time: &Time, delta_time: &DeltaTime| {
                state.step(SIM_SECS * delta_time.alpha);
            },
        )
        .with_duration(SIM_SECS),
    );
}
