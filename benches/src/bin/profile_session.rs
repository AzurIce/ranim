//! samply profiling target: run one scene session path in a tight loop.
//!
//! Usage: profile_session [evaluator|player] [n] [rounds]

use std::hint::black_box;

use benches::test_scenes::transform_squares;
use ranim::{SceneConstructor, prelude::*};
use ranim_core::{EvaluatedFrame, SceneEvaluator, ScenePlayer, SceneSession, SealedRanimScene};

fn run<S: SceneSession>(scene: SealedRanimScene, frames: usize, rounds: usize) {
    let mut session = S::from_sealed(scene);
    let total = session.total_secs();
    let mut frame = EvaluatedFrame::new();
    for _ in 0..rounds {
        for i in 0..frames {
            frame.clear();
            session.sample_at(black_box(total * i as f64 / frames as f64), &mut frame);
            black_box(&frame);
        }
    }
}

fn main() {
    let mut args = std::env::args().skip(1);
    let mode = args.next().unwrap_or_else(|| "evaluator".into());
    let n: usize = args.next().and_then(|s| s.parse().ok()).unwrap_or(50);
    let rounds: usize = args.next().and_then(|s| s.parse().ok()).unwrap_or(50);

    let scene = (|r: &mut RanimScene| transform_squares(r, n)).build_scene();
    match mode.as_str() {
        "evaluator" => run::<SceneEvaluator>(scene, 60, rounds),
        "player" => run::<ScenePlayer>(scene, 60, rounds),
        other => eprintln!("unknown mode: {other}"),
    }
}
