mod common;

use bevy::prelude::*;

const NUM_BARS: usize = 48;
const FRAME_HEIGHT: f64 = 8.0;
const FRAME_WIDTH: f64 = FRAME_HEIGHT * 16.0 / 9.0;
const WHITE: [f32; 4] = [1.0, 1.0, 1.0, 0.50];
const BLUE: [f32; 4] = [0.22, 0.48, 0.96, 0.58];

fn main() {
    common::run_example(
        common::ExampleConfig::new("Ranim Bevy: Bubble Sort", "bubble_sort", 15.0),
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
    for i in (1..NUM_BARS).rev() {
        for j in 0..i {
            sync_time = sync_time.max(bars[j].cursor()).max(bars[j + 1].cursor());
            for bar in bars.get_disjoint_mut([j, j + 1]).unwrap() {
                bar.wait_until(sync_time);
                bar.play(step_duration, common::RateFunc::Linear, |state| {
                    state.set_fill(BLUE);
                });
            }

            sync_time += step_duration;
            if heights[j] > heights[j + 1] {
                let [left, right] = bars.get_disjoint_mut([j, j + 1]).unwrap();
                left.wait_until(sync_time);
                right.wait_until(sync_time);
                left.play(step_duration, common::RateFunc::Linear, |state| {
                    state.transform.translation.x += width_unit as f32;
                });
                right.play(step_duration, common::RateFunc::Linear, |state| {
                    state.transform.translation.x -= width_unit as f32;
                });

                sync_time += step_duration;
                heights.swap(j, j + 1);
                bars.swap(j, j + 1);
            }

            sync_time = sync_time.max(bars[j].cursor()).max(bars[j + 1].cursor());
            for bar in bars.get_disjoint_mut([j, j + 1]).unwrap() {
                bar.wait_until(sync_time);
                bar.play(step_duration, common::RateFunc::Linear, |state| {
                    state.set_fill(WHITE);
                });
            }
            sync_time += step_duration;
        }
    }

    for bar in bars {
        common::spawn_timeline(&mut commands, bar);
    }
}
