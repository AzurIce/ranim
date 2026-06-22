mod common;

use bevy::prelude::*;

const NUM_BARS: usize = 42;
const FRAME_HEIGHT: f64 = 8.0;
const FRAME_WIDTH: f64 = FRAME_HEIGHT * 16.0 / 9.0;
const WHITE: [f32; 4] = [1.0, 1.0, 1.0, 0.50];
const RED: [f32; 4] = [0.90, 0.20, 0.28, 0.58];
const BLUE: [f32; 4] = [0.20, 0.46, 0.95, 0.58];

fn main() {
    common::run_example(
        common::ExampleConfig::new("Ranim Bevy: Selection Sort", "selection_sort", 15.0),
        build_scene,
    );
}

fn build_scene(app: &mut App) {
    app.add_systems(Startup, setup)
        .add_systems(Update, common::animate_vitem_timelines);
}

fn setup(mut commands: Commands) {
    common::camera(
        &mut commands,
        Transform::from_xyz(0.0, 0.0, 11.0).looking_at(Vec3::ZERO, Vec3::Y),
    );

    let padded_width = FRAME_WIDTH * 0.90;
    let padded_height = FRAME_HEIGHT * 0.90;
    let width_unit = padded_width / NUM_BARS as f64;
    let height_unit = padded_height / NUM_BARS as f64;
    let base_x = -padded_width * 0.5 + width_unit * 0.5;
    let base_y = -padded_height * 0.5;
    let step_duration = 15.0 / (NUM_BARS * NUM_BARS) as f32;

    let mut heights = common::shuffled_heights(NUM_BARS, 114514);
    let mut bars = heights
        .iter()
        .enumerate()
        .map(|(idx, &height)| {
            let item = common::bottom_centered_rect(
                width_unit * 0.80,
                height as f64 * height_unit,
                WHITE,
                WHITE,
                0.0,
            );
            let transform = Transform::from_xyz(
                (base_x + width_unit * idx as f64) as f32,
                base_y as f32,
                0.0,
            );
            common::VItemTimelineBuilder::new(common::VItemAnimState::new(item, transform))
        })
        .collect::<Vec<_>>();

    let mut sync_time = 0.0f32;
    for i in 0..NUM_BARS - 1 {
        bars[i].wait_until(sync_time);
        bars[i].play(step_duration, common::RateFunc::Linear, |state| {
            state.set_fill(RED);
        });

        for j in i + 1..NUM_BARS {
            sync_time = sync_time.max(bars[i].cursor()).max(bars[j].cursor());
            bars[j].wait_until(sync_time);
            bars[j].play(step_duration, common::RateFunc::Linear, |state| {
                state.set_fill(RED);
            });
            sync_time += step_duration;

            if heights[i] > heights[j] {
                let [current, candidate] = bars.get_disjoint_mut([i, j]).unwrap();
                current.wait_until(sync_time);
                candidate.wait_until(sync_time);
                current.play(step_duration, common::RateFunc::Linear, |state| {
                    state.transform.translation.x += ((j - i) as f64 * width_unit) as f32;
                    state.set_fill(BLUE);
                });
                candidate.play(step_duration, common::RateFunc::Linear, |state| {
                    state.transform.translation.x -= ((j - i) as f64 * width_unit) as f32;
                    state.set_fill(RED);
                });

                sync_time += step_duration;
                heights.swap(i, j);
                bars.swap(i, j);
            }

            sync_time = sync_time.max(bars[j].cursor());
            bars[j].wait_until(sync_time);
            bars[j].play(step_duration, common::RateFunc::Linear, |state| {
                state.set_fill(WHITE);
            });
            sync_time += step_duration;
        }

        bars[i].wait_until(sync_time);
        bars[i].play(step_duration, common::RateFunc::Linear, |state| {
            state.set_fill(WHITE);
        });
        sync_time += step_duration;
    }

    for bar in bars {
        common::spawn_timeline(&mut commands, bar);
    }
}
