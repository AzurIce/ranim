//! Cloth simulation: a particle-spring flag waving in the wind.
//!
//! Classic "iterative only" animation: spring forces with self-collision have
//! no closed form, and the state (particle positions/velocities) is inherently
//! carried across frames by `step`.
//!
//! The flag is a grid of particles pinned along its left edge. Structural,
//! shear and bend springs hold it together; a spatial-gradient, time-varying
//! wind makes it wave; self-collision repulsion keeps it from folding onto
//! itself. Integration is Verlet (semi-implicit, stable for spring nets).

use ranim::{
    color::palettes::manim,
    core::animation::{Evaluator, SegmentTime},
    glam::DVec3,
    items::vitem::VItem,
    prelude::*,
};

use std::f64::consts::TAU;

const ROWS: usize = 9;
const COLS: usize = 15;
const SPACING: f64 = 0.25;
const GRAVITY: f64 = 0.4;
const DAMPING: f64 = 0.99;
const K_STRUCTURAL: f64 = 200.0;
const K_SHEAR: f64 = 150.0;
const K_BEND: f64 = 100.0;
const REPULSION_CUTOFF: f64 = 0.4;
const REPULSION_K: f64 = 300.0;
// Wind: two time-varying components, amplified toward the flag tip.
const WIND_AMP1: f64 = 0.35;
const WIND_FREQ1: f64 = 0.18;
const WIND_AMP2: f64 = 0.10;
const WIND_FREQ2: f64 = 0.45;
const WIND_SPATIAL: f64 = 1.5;
const TOTAL_SECS: f64 = 20.0;

/// A particle-spring cloth grid.
struct FlagCloth {
    curr: Vec<DVec3>,
    prev: Vec<DVec3>,
    pinned: Vec<bool>,
    springs: Vec<(usize, usize, f64, f64)>, // (i, j, rest_len, stiffness)
    initial: Vec<DVec3>,
}

impl FlagCloth {
    fn new() -> Self {
        let width = (COLS - 1) as f64 * SPACING;
        let mut curr = Vec::with_capacity(ROWS * COLS);
        for r in 0..ROWS {
            for c in 0..COLS {
                curr.push(DVec3::new(
                    -width / 2.0 + c as f64 * SPACING,
                    2.5 - r as f64 * SPACING,
                    0.0,
                ));
            }
        }
        let pinned = (0..ROWS * COLS)
            .map(|i| i % COLS == 0) // left column pinned (flagpole)
            .collect();

        let mut springs = Vec::new();
        for r in 0..ROWS {
            for c in 0..COLS {
                let i = r * COLS + c;
                if c + 1 < COLS {
                    springs.push((i, i + 1, SPACING, K_STRUCTURAL));
                }
                if r + 1 < ROWS {
                    springs.push((i, i + COLS, SPACING, K_STRUCTURAL));
                }
                if c + 1 < COLS && r + 1 < ROWS {
                    springs.push((i, i + COLS + 1, SPACING * 2.0f64.sqrt(), K_SHEAR));
                }
                if c + 2 < COLS {
                    springs.push((i, i + 2, 2.0 * SPACING, K_BEND));
                }
                if r + 2 < ROWS {
                    springs.push((i, i + 2 * COLS, 2.0 * SPACING, K_BEND));
                }
            }
        }

        Self {
            initial: curr.clone(),
            prev: curr.clone(),
            curr,
            pinned,
            springs,
        }
    }
}

impl Evaluator for FlagCloth {
    type Output = Vec<VItem>;

    fn reset(&mut self) {
        self.curr = self.initial.clone();
        self.prev = self.initial.clone();
    }

    fn step(&mut self, time: &SegmentTime) {
        let dt2 = time.local_delta_secs * time.local_delta_secs;
        let n = self.curr.len();

        // Spring forces.
        let mut ax = vec![0.0f64; n];
        let mut ay = vec![0.0f64; n];
        for &(i, j, rest, k) in &self.springs {
            let d = self.curr[j] - self.curr[i];
            let dist = d.length();
            if dist < 1e-9 {
                continue;
            }
            let f = k * (dist - rest) / dist;
            ax[i] += f * d.x;
            ay[i] += f * d.y;
            ax[j] -= f * d.x;
            ay[j] -= f * d.y;
        }

        // Self-collision repulsion (keeps the cloth from folding onto itself).
        for i in 0..n {
            for j in (i + 1)..n {
                let d = self.curr[j] - self.curr[i];
                let dist = d.length();
                if dist < REPULSION_CUTOFF && dist > 1e-9 {
                    let f = REPULSION_K * (REPULSION_CUTOFF - dist) / dist;
                    ax[i] -= f * d.x;
                    ay[i] -= f * d.y;
                    ax[j] += f * d.x;
                    ay[j] += f * d.y;
                }
            }
        }

        // Wind (time-varying, amplified toward the tip) + gravity, Verlet step.
        let t = time.global_secs;
        let wind_base =
            WIND_AMP1 * (TAU * WIND_FREQ1 * t).sin() + WIND_AMP2 * (TAU * WIND_FREQ2 * t).sin();
        for i in 0..n {
            if self.pinned[i] {
                continue;
            }
            let c = i % COLS;
            let wind = wind_base * (1.0 + WIND_SPATIAL * c as f64 / (COLS - 1) as f64);
            let vx = (self.curr[i].x - self.prev[i].x) * DAMPING;
            let vy = (self.curr[i].y - self.prev[i].y) * DAMPING;
            self.prev[i] = self.curr[i];
            self.curr[i].x += vx + (ax[i] + wind) * dt2;
            self.curr[i].y += vy + (ay[i] - GRAVITY) * dt2;
        }
    }

    fn sample(&self, _time: &SegmentTime) -> Vec<VItem> {
        let color = manim::TEAL_C;
        let mut items = Vec::new();
        // Horizontal rows.
        for r in 0..ROWS {
            let points: Vec<DVec3> = (0..COLS).map(|c| self.curr[r * COLS + c]).collect();
            let mut line = VItem::from_vpoints(points);
            line.set_stroke_color(color);
            line.set_stroke_width(1.5);
            items.push(line);
        }
        // Vertical columns.
        for c in 0..COLS {
            let points: Vec<DVec3> = (0..ROWS).map(|r| self.curr[r * COLS + c]).collect();
            let mut line = VItem::from_vpoints(points);
            line.set_stroke_color(color);
            line.set_stroke_width(1.5);
            items.push(line);
        }
        items
    }
}

#[scene]
#[output(dir = "./output/cloth")]
fn cloth(r: &mut RanimScene) {
    r.play(CameraFrame::default().show().with_duration(TOTAL_SECS));
    r.play(FlagCloth::new().with_duration(TOTAL_SECS));
}
