//! Iterative animation example: a damped spring (state driven by
//! [`IterativeEval`], no ECS required).
//!
//! The spring displacement `x` is the segment's internal state, integrated by
//! `step` (semi-implicit Euler); `sample` projects the current state into a
//! `VItem`. `reset` guarantees deterministic replay (`SceneEvaluator::seek`
//! matches forward advancement).
//!
//! The spring's logical duration is a construction parameter (`sim_secs`):
//! `with_duration` only stretches playback, while the segment integrates
//! `sim_secs` worth of physics scaled from `DeltaTime::alpha`.

use ranim::{
    anims::iterative::{Iterative, IterativeEval},
    color::palettes::manim,
    core::time::{DeltaTime, Time},
    items::vitem::{VItem, geometry::Rectangle},
    prelude::*,
};

/// A damped spring: x'' = −k·x − c·x' (m = 1).
struct SpringBall {
    k: f64,
    c: f64,
    x: f64,
    v: f64,
    /// How many seconds of physics the whole segment simulates over.
    sim_secs: f64,
}

impl SpringBall {
    fn new(k: f64, c: f64, sim_secs: f64) -> Self {
        Self {
            k,
            c,
            x: 1.0,
            v: 0.0,
            sim_secs,
        }
    }
}

impl IterativeEval for SpringBall {
    type Output = VItem;

    fn reset(&mut self) {
        self.x = 1.0;
        self.v = 0.0;
    }

    fn step(&mut self, _time: &Time, delta_time: &DeltaTime) {
        let dt = self.sim_secs * delta_time.alpha;
        let acc = -self.k * self.x - self.c * self.v;
        self.v += acc * dt;
        self.x += self.v * dt;
    }

    fn sample(&self) -> VItem {
        let mut ball = VItem::from(Rectangle::new(0.6, 0.6));
        ball.set_fill_color(manim::BLUE_C);
        ball.set_stroke_opacity(0.0);
        ball.move_to([self.x, 0.0, 0.0].into());
        ball
    }
}

#[scene]
#[output(dir = "./output/iterative_spring")]
fn iterative_spring(r: &mut RanimScene) {
    r.play(CameraFrame::default().show().with_duration(4.0));
    r.play(Iterative(SpringBall::new(25.0, 1.0, 4.0)).with_duration(4.0));
}
