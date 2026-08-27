//! Double pendulum — a canonical demonstration of deterministic chaos:
//! sensitive dependence on initial conditions.
//!
//! Three identical double pendulums are released from rest, with the second
//! arm's initial angle differing by only `EPSILON` (0.001 rad ≈ 0.057°)
//! between neighbours. For the first seconds the three evolve on top of each
//! other and are indistinguishable; then the tiny difference is amplified
//! exponentially and the trajectories diverge completely — deterministic
//! equations, unpredictable long-term behaviour.
//!
//! The state is integrated with RK4 (sub-stepped to at most 1/240 s) inside an
//! `Iterative` animation, following the same pattern as `examples/nbody`:
//! the closure advances the physical state by `sim_secs * delta_alpha`, and
//! the `Extract` impl projects rods, bobs and fading tip-trails into
//! `CoreItem`s once per frame (bobs stay semantic under `Translation`
//! wrappers).

use ranim::{
    color::{AlphaColor, Srgb, palettes::manim},
    core::Extract,
    core::animation::eval::iterative::Iterative,
    core::core_item::CoreItem,
    glam::DVec3,
    items::vitem::{
        VItem,
        geometry::{Circle, Line},
    },
    prelude::*,
};

const L1: f64 = 1.5;
const L2: f64 = 1.5;
const M1: f64 = 1.0;
const M2: f64 = 1.0;
const G: f64 = 9.8;

/// Number of pendulums simulated together.
const N_PENDULUMS: usize = 3;
/// Initial-angle difference between neighbours, in rad (0.001 rad ≈ 0.057°).
const EPSILON: f64 = 0.001;
/// Sample a trail point every 2nd sub-step (~120 Hz).
const TRAIL_SAMPLE_EVERY: usize = 2;
/// Trail length in samples (~1.25 s of tip history per pendulum).
const TRAIL_LEN: usize = 150;

const PALETTE: [AlphaColor<Srgb>; N_PENDULUMS] = [manim::BLUE_C, manim::YELLOW_C, manim::RED_C];

/// State of one planar double pendulum (point masses on massless rods).
#[derive(Clone, Copy)]
struct DoublePendulum {
    /// Angle of the first rod from straight down.
    theta1: f64,
    /// Angle of the second rod from straight down.
    theta2: f64,
    omega1: f64,
    omega2: f64,
}

impl DoublePendulum {
    /// Time derivatives `[dtheta1, dtheta2, domega1, domega2]` of the state.
    fn derivatives(&self) -> [f64; 4] {
        let &DoublePendulum {
            theta1: t1,
            theta2: t2,
            omega1: w1,
            omega2: w2,
        } = self;
        let d = t1 - t2;
        let den = 2.0 * M1 + M2 - M2 * (2.0 * d).cos();
        let a1 = (-G * (2.0 * M1 + M2) * t1.sin()
            - M2 * G * (t1 - 2.0 * t2).sin()
            - 2.0 * d.sin() * M2 * (w2 * w2 * L2 + w1 * w1 * L1 * d.cos()))
            / (L1 * den);
        let a2 = (2.0
            * d.sin()
            * (w1 * w1 * L1 * (M1 + M2) + G * (M1 + M2) * t1.cos() + w2 * w2 * L2 * M2 * d.cos()))
            / (L2 * den);
        [w1, w2, a1, a2]
    }

    /// Advance the state by `dt` with one classical RK4 step.
    fn rk4_step(&mut self, dt: f64) {
        let s = [self.theta1, self.theta2, self.omega1, self.omega2];
        let eval = |s: [f64; 4]| {
            DoublePendulum {
                theta1: s[0],
                theta2: s[1],
                omega1: s[2],
                omega2: s[3],
            }
            .derivatives()
        };
        let k1 = eval(s);
        let k2 = eval(std::array::from_fn::<f64, 4, _>(|i| {
            s[i] + k1[i] * dt / 2.0
        }));
        let k3 = eval(std::array::from_fn::<f64, 4, _>(|i| {
            s[i] + k2[i] * dt / 2.0
        }));
        let k4 = eval(std::array::from_fn::<f64, 4, _>(|i| s[i] + k3[i] * dt));
        let next = std::array::from_fn::<f64, 4, _>(|i| {
            s[i] + dt / 6.0 * (k1[i] + 2.0 * k2[i] + 2.0 * k3[i] + k4[i])
        });
        self.theta1 = next[0];
        self.theta2 = next[1];
        self.omega1 = next[2];
        self.omega2 = next[3];
    }

    /// Cartesian positions of the two bobs, hanging from `pivot`.
    fn positions(&self, pivot: DVec3) -> (DVec3, DVec3) {
        let p1 = pivot + L1 * DVec3::new(self.theta1.sin(), -self.theta1.cos(), 0.0);
        let p2 = p1 + L2 * DVec3::new(self.theta2.sin(), -self.theta2.cos(), 0.0);
        (p1, p2)
    }
}

/// The full scene state: `N_PENDULUMS` near-identical pendulums plus the
/// recent tip trajectories used for the fading trails.
#[derive(Clone)]
struct ChaosState {
    pendulums: [DoublePendulum; N_PENDULUMS],
    trails: [Vec<DVec3>; N_PENDULUMS],
    pivot: DVec3,
    step_count: usize,
}

impl ChaosState {
    fn new() -> Self {
        let pivot = DVec3::new(0.0, 0.6, 0.0);
        // Released from rest; only theta2 differs, by EPSILON per pendulum.
        let pendulums = std::array::from_fn(|i| DoublePendulum {
            theta1: 2.0,
            theta2: 2.5 + EPSILON * i as f64,
            omega1: 0.0,
            omega2: 0.0,
        });
        Self {
            pendulums,
            trails: std::array::from_fn(|_| Vec::new()),
            pivot,
            step_count: 0,
        }
    }

    /// Advance the simulation by `dt` seconds of physical time, sub-stepping
    /// RK4 so that each integration step is at most 1/240 s.
    fn step(&mut self, dt: f64) {
        let n = ((dt.abs() * 240.0).ceil() as usize).max(1);
        let h = dt / n as f64;
        for _ in 0..n {
            for p in &mut self.pendulums {
                p.rk4_step(h);
            }
            self.step_count += 1;
            if self.step_count.is_multiple_of(TRAIL_SAMPLE_EVERY) {
                for (i, p) in self.pendulums.iter().enumerate() {
                    let (_, tip) = p.positions(self.pivot);
                    let trail = &mut self.trails[i];
                    trail.push(tip);
                    if trail.len() > TRAIL_LEN {
                        trail.remove(0);
                    }
                }
            }
        }
    }
}

/// Build an open polyline `VItem` through `points` (quadratic vpoint triplets:
/// anchor, midpoint control, anchor, ...).
fn polyline(points: &[DVec3]) -> Option<VItem> {
    if points.len() < 2 {
        return None;
    }
    let mut vpoints = Vec::with_capacity(points.len() * 2 - 1);
    vpoints.push(points[0]);
    for w in points.windows(2) {
        vpoints.push((w[0] + w[1]) / 2.0);
        vpoints.push(w[1]);
    }
    Some(VItem::from_vpoints(vpoints))
}

impl Extract for ChaosState {
    type Target = CoreItem;

    fn extract_into(&self, buf: &mut Vec<CoreItem>) {
        // Trails first (drawn underneath): a dim polyline per pendulum.
        for (i, trail) in self.trails.iter().enumerate() {
            if let Some(mut line) = polyline(trail) {
                line.set_stroke_color(PALETTE[i]);
                line.set_stroke_opacity(0.35);
                line.set_stroke_width(0.04);
                line.extract_into(buf);
            }
        }
        // Rods and bobs.
        for (i, p) in self.pendulums.iter().enumerate() {
            let (p1, p2) = p.positions(self.pivot);

            let mut rod1 = VItem::from(Line::new(self.pivot, p1));
            rod1.set_stroke_color(PALETTE[i]);
            rod1.set_stroke_width(0.06);
            rod1.extract_into(buf);

            let mut rod2 = VItem::from(Line::new(p1, p2));
            rod2.set_stroke_color(PALETTE[i]);
            rod2.set_stroke_width(0.06);
            rod2.extract_into(buf);

            let mut bob1 = Circle::new(0.10).transformed(Translation(p1));
            bob1.set_fill_color(PALETTE[i]);
            bob1.set_stroke_opacity(0.0);
            bob1.extract_into(buf);

            let mut bob2 = Circle::new(0.14).transformed(Translation(p2));
            bob2.set_fill_color(PALETTE[i]);
            bob2.set_stroke_opacity(0.0);
            bob2.extract_into(buf);
        }
        // Pivot, drawn on top.
        let mut pivot_dot = Circle::new(0.06).transformed(Translation(self.pivot));
        pivot_dot.set_fill_color(manim::GREY_C);
        pivot_dot.set_stroke_opacity(0.0);
        pivot_dot.extract_into(buf);
    }
}

#[scene]
#[wasm_demo_doc]
#[output(dir = "./output/agents/double_pendulum")]
fn double_pendulum(r: &mut RanimScene) {
    let sim_secs = 24.0;
    r.play(CameraFrame::default().show().with_duration(sim_secs));
    r.play(
        Iterative::from_fn(
            ChaosState::new(),
            move |state: &mut ChaosState, _alpha: f64, delta_alpha: f64| {
                state.step(sim_secs * delta_alpha);
            },
        )
        .with_steps((sim_secs * 60.0) as usize)
        .with_duration(sim_secs),
    );
    r.insert_time_mark(sim_secs, TimeMark::Capture("preview.png".to_string()));
}
