//! N-body gravitational simulation — a canonical "iterative only" animation:
//! no closed-form solution, chaotic, and inherently stateful across frames.
//!
//! Three bodies mutually attract through gravity (velocity-Verlet integration).
//! Each body leaves a fading trail of recent positions. The scene uses a
//! default (linear) rate, so `local_delta_secs` equals the physical tick.

use ranim::{
    color::{AlphaColor, Srgb, palettes::manim},
    core::animation::{Evaluator, SegmentTime},
    glam::DVec3,
    items::vitem::{VItem, geometry::Circle},
    prelude::*,
};

const G: f64 = 2.0;
const BODY_COUNT: usize = 3;
const TRAIL_SAMPLE_EVERY: usize = 4; // sample a trail point every 4th step (~30 Hz)
const TRAIL_LEN: usize = 60; // ~2 s of trail per body
const TOTAL_SECS: f64 = 16.0;

#[derive(Clone, Copy)]
struct Body {
    pos: DVec3,
    vel: DVec3,
    mass: f64,
}

/// A three-body system with mutual gravity.
struct NBody {
    bodies: [Body; BODY_COUNT],
    initial: [Body; BODY_COUNT],
    trails: [Vec<DVec3>; BODY_COUNT],
    step_count: usize,
    colors: [AlphaColor<Srgb>; BODY_COUNT],
}

impl NBody {
    fn new() -> Self {
        // Three equal masses on a circle with tangential velocities plus a tiny
        // asymmetry. v0 ≈ 104% of the circular speed: the system stays
        // gravitationally bound (max radius ≈ 3.4 < frame half-height 4) while
        // evolving chaotically (the equilateral equilibrium is unstable).
        let radius = 2.0;
        let v0 = 0.6;
        let mut bodies = [Body {
            pos: DVec3::ZERO,
            vel: DVec3::ZERO,
            mass: 1.0,
        }; BODY_COUNT];
        for (i, body) in bodies.iter_mut().enumerate() {
            let angle = i as f64 * std::f64::consts::TAU / BODY_COUNT as f64;
            body.pos = DVec3::new(radius * angle.cos(), radius * angle.sin(), 0.0);
            let velocity = v0 * (1.0 + 0.02 * i as f64);
            body.vel = DVec3::new(
                -radius * velocity * angle.sin() / radius,
                radius * velocity * angle.cos() / radius,
                0.0,
            );
        }
        let colors = [manim::BLUE_C, manim::RED_C, manim::YELLOW_C];
        Self {
            initial: bodies,
            bodies,
            trails: [Vec::new(), Vec::new(), Vec::new()],
            step_count: 0,
            colors,
        }
    }

    fn accelerations(&self) -> [DVec3; BODY_COUNT] {
        let mut accs = [DVec3::ZERO; BODY_COUNT];
        for (i, acc) in accs.iter_mut().enumerate() {
            for j in 0..BODY_COUNT {
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
}

impl Evaluator for NBody {
    type Output = Vec<VItem>;

    fn reset(&mut self) {
        self.bodies = self.initial;
        for trail in &mut self.trails {
            trail.clear();
        }
        self.step_count = 0;
    }

    fn step(&mut self, time: &SegmentTime) {
        // Velocity Verlet (symplectic): conserves energy far better than
        // semi-implicit Euler for gravitational orbits.
        let dt = time.local_delta_secs;
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

    fn sample(&self, _time: &SegmentTime) -> Vec<VItem> {
        let mut items = Vec::new();
        // Trails: dim dots at recent positions.
        for (i, trail) in self.trails.iter().enumerate() {
            for &p in trail {
                let mut dot = VItem::from(Circle::new(0.02));
                dot.set_fill_color(self.colors[i]);
                dot.set_fill_opacity(0.15);
                dot.set_stroke_opacity(0.0);
                dot.move_to(p);
                items.push(dot);
            }
        }
        // Bodies.
        for (i, body) in self.bodies.iter().enumerate() {
            let mut ball = VItem::from(Circle::new(0.32));
            ball.set_fill_color(self.colors[i]);
            ball.set_stroke_opacity(0.0);
            ball.move_to(body.pos);
            items.push(ball);
        }
        items
    }
}

#[scene]
#[output(dir = "./output/nbody")]
fn nbody(r: &mut RanimScene) {
    r.play(CameraFrame::default().show().with_duration(TOTAL_SECS));
    r.play(NBody::new().with_duration(TOTAL_SECS));
}
