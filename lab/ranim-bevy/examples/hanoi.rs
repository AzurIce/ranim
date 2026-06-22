mod common;

use bevy::prelude::*;

const DISK_COUNT: usize = 7;
const TOTAL_SECONDS: f32 = 12.0;
const ROD_WIDTH: f64 = 0.28;
const ROD_HEIGHT: f64 = 5.0;
const ROD_SECTION_WIDTH: f64 = 4.0;
const BASE_Y: f64 = -3.4;

fn main() {
    common::run_example(
        common::ExampleConfig::new("Ranim Bevy: Hanoi", "hanoi", TOTAL_SECONDS),
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
    common::light(&mut commands);

    for idx in 0..3 {
        let rod = common::bottom_centered_rect(
            ROD_WIDTH,
            ROD_HEIGHT,
            [0.48, 0.50, 0.54, 1.0],
            [0.48, 0.50, 0.54, 1.0],
            0.0,
        );
        commands.spawn((
            ranim_bevy::RanimVItem::new(rod),
            Transform::from_xyz(rod_x(idx), BASE_Y as f32, -0.03),
        ));
    }

    let min_disk_width = ROD_WIDTH * 1.7;
    let max_disk_width = ROD_SECTION_WIDTH * 0.80;
    let disk_height = ROD_HEIGHT * 0.80 / DISK_COUNT as f64;
    let move_duration = TOTAL_SECONDS / (2.0_f32.powi(DISK_COUNT as i32) - 1.0) / 3.0;

    let mut disks = (0..DISK_COUNT)
        .map(|idx| {
            let factor = idx as f32 / (DISK_COUNT - 1) as f32;
            let disk_width =
                min_disk_width + (max_disk_width - min_disk_width) * (1.0 - factor as f64);
            let color = color_lerp([0.88, 0.20, 0.18, 1.0], [0.18, 0.38, 0.90, 1.0], factor);
            let item = common::bottom_centered_rect(disk_width, disk_height * 0.82, color, color, 0.0);
            let transform = Transform::from_xyz(
                rod_x(0),
                (BASE_Y + disk_height * idx as f64) as f32,
                idx as f32 * 0.002,
            );

            common::VItemTimelineBuilder::new(common::VItemAnimState::new(item, transform))
        })
        .collect::<Vec<_>>();
    let mut rods = [Vec::new(), Vec::new(), Vec::new()];
    rods[0] = (0..DISK_COUNT).collect::<Vec<_>>();

    let mut sync_time = 0.0f32;
    solve_hanoi(DISK_COUNT, 0, 1, 2, &mut |src, dst| {
        let top_src_y = BASE_Y + disk_height * (rods[src].len() as f64 - 1.0);
        let top_dst_y = BASE_Y + disk_height * rods[dst].len() as f64;
        let disk_idx = rods[src].pop().unwrap();
        let disk = &mut disks[disk_idx];

        disk.wait_until(sync_time);
        disk.play(move_duration, common::RateFunc::EaseInQuad, |state| {
            state.transform.translation.y += (3.0 - top_src_y) as f32;
        });
        disk.play(move_duration, common::RateFunc::Linear, |state| {
            state.transform.translation.x += (rod_x(dst) - rod_x(src)) as f32;
        });
        disk.play(move_duration, common::RateFunc::EaseOutQuad, |state| {
            state.transform.translation.y += (top_dst_y - 3.0) as f32;
        });

        sync_time = disk.cursor();
        rods[dst].push(disk_idx);
    });

    for disk in disks {
        common::spawn_timeline(&mut commands, disk);
    }
}

fn solve_hanoi(
    n: usize,
    src: usize,
    dst: usize,
    tmp: usize,
    move_disk: &mut impl FnMut(usize, usize),
) {
    if n == 1 {
        move_disk(src, dst);
    } else {
        solve_hanoi(n - 1, src, tmp, dst, move_disk);
        move_disk(src, dst);
        solve_hanoi(n - 1, tmp, dst, src, move_disk);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_moves_keep_rods_ordered() {
        let mut rods = [Vec::new(), Vec::new(), Vec::new()];
        rods[0] = (0..DISK_COUNT).collect::<Vec<_>>();

        solve_hanoi(DISK_COUNT, 0, 1, 2, &mut |src, dst| {
            let disk = rods[src].pop().expect("source rod should not be empty");
            if let Some(&top) = rods[dst].last() {
                assert!(disk > top, "larger disk index means smaller disk");
            }
            rods[dst].push(disk);
        });

        assert!(rods[0].is_empty());
        assert!(rods[2].is_empty());
        assert_eq!(rods[1], (0..DISK_COUNT).collect::<Vec<_>>());
    }
}

fn rod_x(idx: usize) -> f32 {
    (idx as f64 - 1.0) as f32 * ROD_SECTION_WIDTH as f32
}

fn color_lerp(from: [f32; 4], to: [f32; 4], t: f32) -> [f32; 4] {
    [
        from[0].lerp(to[0], t),
        from[1].lerp(to[1], t),
        from[2].lerp(to[2], t),
        from[3].lerp(to[3], t),
    ]
}
