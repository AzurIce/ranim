//! Iterative animation example: a damped spring (state stepped by a named
//! [`IterativeEval`], no ECS required).
//!
//! The spring's state (`x`, `v`) is owned by `Iterative` and advanced by
//! `SpringEval` via semi-implicit Euler; the `Extract` impl projects the state
//! into a `VItem` once per frame. Reset is structural — the adapter restores
//! the stored initial state, so `SceneEvaluator::seek` matches forward
//! advancement for free.
//!
//! The spring's logical duration is owned by its evaluator (`SpringEval`):
//! the segment integrates `sim_secs` worth of physics scaled from the progress
//! step `delta_alpha`, and the same field sets the playback duration.

use ranim::{
    color::palettes::manim,
    core::Extract,
    core::animation::eval::iterative::Iterative,
    core::core_item::CoreItem,
    items::vitem::{VItem, geometry::Rectangle},
    prelude::*,
};

/// The spring's state: displacement and velocity.
#[derive(Clone)]
struct SpringState {
    x: f64,
    v: f64,
}

impl Extract for SpringState {
    type Target = CoreItem;

    fn extract_into(&self, buf: &mut Vec<CoreItem>) {
        let mut ball = VItem::from(Rectangle::new(0.6, 0.6));
        ball.set_fill_color(manim::BLUE_C);
        ball.set_stroke_opacity(0.0);
        ball.move_to([self.x, 0.0, 0.0].into());
        ball.extract_into(buf);
    }
}

const K: f64 = 25.0;
const C: f64 = 1.0;

/// The spring segment's logical duration.
///
/// The simulation time step is `sim_secs * delta_alpha`; the animation plays
/// for the same `sim_secs`, so physical time and timeline time stay in sync.
struct SpringEval {
    sim_secs: f64,
}

impl IterativeEval for SpringEval {
    type Output = SpringState;

    fn step(&self, state: &mut Self::Output, _alpha: f64, delta_alpha: f64) {
        let dt = self.sim_secs * delta_alpha;
        let acc = -K * state.x - C * state.v;
        state.v += acc * dt;
        state.x += state.v * dt;
    }
}

/// Build the spring segment with one logical duration.
fn spring_animation(sim_secs: f64) -> impl Animation {
    Iterative::new(SpringState { x: 1.0, v: 0.0 }, SpringEval { sim_secs }).with_duration(sim_secs)
}

#[scene]
#[output(dir = "./output/iterative_spring")]
fn iterative_spring(r: &mut RanimScene) {
    let sim_secs = 4.0;
    r.play(CameraFrame::default().show().with_duration(sim_secs));
    r.play(spring_animation(sim_secs));
}
