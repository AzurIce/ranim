use glam::{ivec3, vec2, vec3, Vec3};
use rand::{seq::SliceRandom, SeedableRng};
use ranim::{
    animation::transform::TransformAnimSchedule, color::palettes::manim, items::vitem::Rectangle,
    prelude::*, timeline::TimeMark, utils::rate_functions::linear,
};

#[timeline]
fn bubble_sort(ranim: Ranim) {
    let Ranim(timeline, _camera) = ranim;

    let frame_size = _camera.data.frame_size();
    let num = 10;
    let anim_step_duration = 0.15;
    let padding = frame_size.x * 0.1;
    let gap = 20.0;
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
    for i in (1..num).rev() {
        for j in 0..i {
            timeline.play(
                rects[j]
                    .transform(|data| {
                        data.set_fill_color(manim::BLUE_C.with_alpha(0.5));
                    })
                    .with_duration(anim_step_duration)
                    .with_rate_func(linear)
                    .apply(),
            );
            timeline.play(
                rects[j + 1]
                    .transform(|data| {
                        data.set_fill_color(manim::BLUE_C.with_alpha(0.5));
                    })
                    .with_duration(anim_step_duration)
                    .with_rate_func(linear)
                    .apply(),
            );
            timeline.sync();

            if heights[j] > heights[j + 1] {
                timeline.play(
                    rects[j]
                        .transform(|data| {
                            data.shift(shift_right)
                                .set_fill_color(manim::BLUE_C.with_alpha(0.5));
                        })
                        .with_duration(anim_step_duration)
                        .with_rate_func(linear)
                        .apply(),
                );
                timeline.play(
                    rects[j + 1]
                        .transform(|data| {
                            data.shift(-shift_right)
                                .set_fill_color(manim::BLUE_C.with_alpha(0.5));
                        })
                        .with_duration(anim_step_duration)
                        .with_rate_func(linear)
                        .apply(),
                );
                timeline.sync();
                heights.swap(j, j + 1);
                rects.swap(j, j + 1);
            }
            timeline.play(
                rects[j]
                    .transform(|data| {
                        data.set_fill_color(manim::WHITE.with_alpha(0.5));
                    })
                    .with_duration(anim_step_duration)
                    .with_rate_func(linear)
                    .apply(),
            );
            timeline.play(
                rects[j + 1]
                    .transform(|data| {
                        data.set_fill_color(manim::WHITE.with_alpha(0.5));
                    })
                    .with_duration(anim_step_duration)
                    .with_rate_func(linear)
                    .apply(),
            );
            timeline.sync();
        }
    }

    timeline.insert_time_mark(
        timeline.duration_secs() / 2.0,
        TimeMark::Capture("preview.png".to_string()),
    );
}

fn main() {
    render_timeline!(bubble_sort);
}
