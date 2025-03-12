use glam::{ivec3, vec2, vec3, Vec3};
use rand::{seq::SliceRandom, SeedableRng};
use ranim::{
    animation::transform::TransformAnimSchedule, color::palettes::manim, items::vitem::Rectangle,
    prelude::*, timeline::TimeMark, utils::rate_functions::linear,
};

#[timeline]
fn selective_sort_massive(ranim: Ranim) {
    let Ranim(timeline, _camera) = ranim;

    let frame_size = _camera.data.frame_size();
    let num = 100;
    let anim_step_duration = 0.0015;
    let padding = frame_size.x * 0.05;
    let gap = 10.0;
    let rect_width = (frame_size.x - 2.0 * padding - (num - 1) as f32 * gap) / num as f32;

    let max_height = frame_size.y - 2.0 * padding;
    let height_unit = max_height / num as f32;

    let mut rng = rand_chacha::ChaChaRng::seed_from_u64(114514);
    let mut heights = (1..=num)
        .map(|x| x as f32 * height_unit)
        .collect::<Vec<f32>>();
    heights.shuffle(&mut rng);

    let frame_bl = vec2(frame_size.x / -2.0, frame_size.y / -2.0);
    let mut rects = heights
        .iter()
        .enumerate()
        .map(|(i, &height)| {
            let mut rect = Rectangle(rect_width, height).build();
            let bottom_left = rect.get_bounding_box_point(ivec3(-1, -1, 0));
            let target_coord = frame_bl.extend(0.0)
                + vec3(padding, padding, 0.0)
                + Vec3::X * (rect_width + gap) * i as f32;
            rect.shift(target_coord - bottom_left)
                .set_color(manim::WHITE)
                .set_fill_opacity(0.5);
            timeline.insert(rect)
        })
        .collect::<Vec<_>>();

    let shift_right = Vec3::X * (gap + rect_width);
    for i in 0..num - 1 {
        timeline.play(
            rects[i]
                .transform(|data| {
                    data.set_color(manim::RED_C).set_fill_opacity(0.5);
                })
                .with_duration(anim_step_duration)
                .with_rate_func(linear)
                .apply(),
        );
        for j in i + 1..num {
            timeline.play(
                rects[j]
                    .transform(|data| {
                        data.set_color(manim::BLUE_C).set_fill_opacity(0.5);
                    })
                    .with_duration(anim_step_duration)
                    .with_rate_func(linear)
                    .apply(),
            );
            timeline.sync();

            if heights[i] > heights[j] {
                timeline.play(
                    rects[i]
                        .transform(|data| {
                            data.shift(shift_right * (j - i) as f32)
                                .set_color(manim::BLUE_C)
                                .set_fill_opacity(0.5);
                        })
                        .with_duration(anim_step_duration)
                        .with_rate_func(linear)
                        .apply(),
                );
                timeline.play(
                    rects[j]
                        .transform(|data| {
                            data.shift(-shift_right * (j - i) as f32)
                                .set_color(manim::RED_C)
                                .set_fill_opacity(0.5);
                        })
                        .with_duration(anim_step_duration)
                        .with_rate_func(linear)
                        .apply(),
                );
                timeline.sync();
                heights.swap(i, j);
                rects.swap(i, j);
            }
            timeline.play(
                rects[j]
                    .transform(|data| {
                        data.set_color(manim::WHITE).set_fill_opacity(0.5);
                    })
                    .with_duration(anim_step_duration)
                    .with_rate_func(linear)
                    .apply(),
            );
            timeline.sync();
        }
        timeline.play(
            rects[i]
                .transform(|data| {
                    data.set_color(manim::WHITE).set_fill_opacity(0.5);
                })
                .with_duration(anim_step_duration)
                .with_rate_func(linear)
                .apply(),
        );
        timeline.sync();
    }

    timeline.insert_time_mark(
        timeline.duration_secs() / 2.0,
        TimeMark::Capture("preview.png".to_string()),
    );
}

fn main() {
    render_timeline!(selective_sort_massive);
}
