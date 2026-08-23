//! samply profiling target: run one render-feeding path in a tight loop.
//!
//! Usage: profile_extract_path [buffered|direct] [n] [rounds]

use std::hint::black_box;

use benches::test_scenes::transform_squares;
use ranim::{SceneConstructor, bevy_ecs::world::World, prelude::*};
use ranim_core::{EvaluatedFrame, SceneEvaluator, ScenePlayer, SealedRanimScene};
use ranim_render::world::{RenderFrame, reconcile, reconcile_logic};

const FRAMES: usize = 60;

fn run_buffered(scene: SealedRanimScene, rounds: usize) {
    let mut session = SceneEvaluator::new(scene);
    let total = session.total_secs();
    let mut render_world = World::new();
    let mut store = RenderFrame::new();
    let mut frame = EvaluatedFrame::new();
    for _ in 0..rounds {
        for i in 0..FRAMES {
            frame.clear();
            session.sample_at(black_box(total * i as f64 / FRAMES as f64), &mut frame);
            store.update(frame.drain(..));
            reconcile(black_box(&mut render_world), black_box(&store));
        }
    }
}

fn run_direct(scene: SealedRanimScene, rounds: usize) {
    let mut player = ScenePlayer::new(scene);
    let total = player.total_secs();
    let mut render_world = World::new();
    for _ in 0..rounds {
        for i in 0..FRAMES {
            player.materialize_at(black_box(total * i as f64 / FRAMES as f64));
            let mut logic = player.take_world();
            reconcile_logic(black_box(&mut render_world), black_box(&mut logic));
            player.put_world(logic);
        }
    }
}

fn main() {
    let mut args = std::env::args().skip(1);
    let mode = args.next().unwrap_or_else(|| "buffered".into());
    let n: usize = args.next().and_then(|s| s.parse().ok()).unwrap_or(50);
    let rounds: usize = args.next().and_then(|s| s.parse().ok()).unwrap_or(30);

    let scene = (|r: &mut RanimScene| transform_squares(r, n)).build_scene();
    match mode.as_str() {
        "buffered" => run_buffered(scene, rounds),
        "direct" => run_direct(scene, rounds),
        other => eprintln!("unknown mode: {other}"),
    }
}
