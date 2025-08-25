use rand::{SeedableRng, seq::SliceRandom};
use ranim::{
    animation::transform::TransformAnim,
    color::palettes::manim,
    components::Anchor,
    glam::{DVec3, dvec2},
    items::vitem::geometry::Rectangle,
    prelude::*,
    utils::rate_functions::linear,
};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

// MARK: bubble_sort

pub fn bubble_sort(r: &mut RanimScene, num: usize) {
    let _r_cam = r.insert_and_show(CameraFrame::default());

    let frame_size = dvec2(8.0 * 16.0 / 9.0, 8.0);
    let padded_frame_size = frame_size * 0.9;

    let anim_step_duration = 15.0 / num.pow(2) as f64;

    let width_unit = padded_frame_size.x / num as f64;
    let height_unit = padded_frame_size.y / num as f64;

    let mut rng = rand_chacha::ChaChaRng::seed_from_u64(114514);
    let mut heights = (1..=num)
        .map(|x| x as f64 * height_unit)
        .collect::<Vec<f64>>();
    heights.shuffle(&mut rng);

    let padded_frame_bl = dvec2(padded_frame_size.x / -2.0, padded_frame_size.y / -2.0);
    let mut r_rects = heights
        .iter()
        .enumerate()
        .map(|(i, &height)| {
            let target_bc_coord =
                padded_frame_bl.extend(0.0) + DVec3::X * (width_unit * i as f64 + width_unit / 2.0);
            let rect = Rectangle::new(width_unit, height).with(|rect| {
                rect.stroke_width = 0.0;
                rect.set_fill_color(manim::WHITE.with_alpha(0.5))
                    .scale(DVec3::splat(0.8))
                    .put_anchor_on(Anchor::edge(0, -1, 0), target_bc_coord);
            });
            r.insert_and_show(rect)
        })
        .collect::<Vec<_>>();

    let anim_highlight = |rect: Rectangle| {
        rect.transform(|data| {
            data.set_fill_color(manim::BLUE_C.with_alpha(0.5));
        })
        .with_duration(anim_step_duration)
        .with_rate_func(linear)
    };
    let anim_unhighlight = |rect: Rectangle| {
        rect.transform(|data| {
            data.set_fill_color(manim::WHITE.with_alpha(0.5));
        })
        .with_duration(anim_step_duration)
        .with_rate_func(linear)
    };
    let shift_right = DVec3::X * width_unit;
    let swap_shift = [shift_right, -shift_right];
    let anim_swap = |timeline: &mut RanimScene, r_rectab: &[&ItemId<Rectangle>; 2]| {
        let timelines = timeline.timeline_mut(r_rectab);
        timelines
            .into_iter()
            .zip(swap_shift.iter())
            .for_each(|(timeline, shift)| {
                timeline.play_with(|rect| {
                    rect.transform(|data| {
                        data.shift(*shift);
                    })
                    .with_duration(anim_step_duration)
                    .with_rate_func(linear)
                });
            });
    };

    for i in (1..num).rev() {
        for j in 0..i {
            r.timeline_mut(&[&r_rects[j], &r_rects[j + 1]])
                .into_iter()
                .for_each(|timeline| {
                    timeline.play_with(anim_highlight);
                });
            if heights[j] > heights[j + 1] {
                anim_swap(r, &[&r_rects[j], &r_rects[j + 1]]);
                r.timelines_mut().sync();
                heights.swap(j, j + 1);
                r_rects.swap(j, j + 1);
            }
            r.timeline_mut(&[&r_rects[j], &r_rects[j + 1]])
                .into_iter()
                .for_each(|timeline| {
                    timeline.play_with(anim_unhighlight);
                });
            r.timelines_mut().sync();
        }
    }
}

#[scene]
#[preview]
#[output]
/// A bubble sort ranim example with input of 10.
///
/// <canvas id="ranim-app-bubble_sort_10" width="1280" height="720" style="width: 100%;"></canvas>
/// <script type="module">
///   const { run_bubble_sort_10 } = await ranim_examples;
///   run_bubble_sort_10();
/// </script>
pub fn bubble_sort_10(r: &mut RanimScene) {
    bubble_sort(r, 10);
}

#[scene]
#[preview]
#[output]
/// A bubble sort ranim example with input of 100.
///
/// <canvas id="ranim-app-bubble_sort_100" width="1280" height="720" style="width: 100%;"></canvas>
/// <script type="module">
///   const { run_bubble_sort_100 } = await ranim_examples;
///   run_bubble_sort_100();
/// </script>
pub fn bubble_sort_100(r: &mut RanimScene) {
    bubble_sort(r, 100);
}

#[cfg(any(feature = "app", target_arch = "wasm32"))]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub fn run_bubble_sort_10() {
    run_scene_app(
        bubble_sort_10_scene.constructor,
        bubble_sort_10_scene.name.to_string(),
    );
}

#[cfg(any(feature = "app", target_arch = "wasm32"))]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub fn run_bubble_sort_100() {
    run_scene_app(
        bubble_sort_100_scene.constructor,
        bubble_sort_100_scene.name.to_string(),
    );
}

// MARK: selective_sort

pub fn selective_sort(r: &mut RanimScene, num: usize) {
    let _r_cam = r.insert_and_show(CameraFrame::default());

    let frame_size = dvec2(8.0 * 16.0 / 9.0, 8.0);
    let padded_frame_size = frame_size * 0.9;

    let anim_step_duration = 15.0 / num.pow(2) as f64;

    let width_unit = padded_frame_size.x / num as f64;
    let height_unit = padded_frame_size.y / num as f64;

    let mut rng = rand_chacha::ChaChaRng::seed_from_u64(114514);
    let mut heights = (1..=num)
        .map(|x| x as f64 * height_unit)
        .collect::<Vec<f64>>();
    heights.shuffle(&mut rng);

    let padded_frame_bl = dvec2(padded_frame_size.x / -2.0, padded_frame_size.y / -2.0);
    let mut r_rects = heights
        .iter()
        .enumerate()
        .map(|(i, &height)| {
            let target_bc_coord =
                padded_frame_bl.extend(0.0) + DVec3::X * (width_unit * i as f64 + width_unit / 2.0);
            let rect = Rectangle::new(width_unit, height).with(|rect| {
                rect.fill_rgba = manim::WHITE.with_alpha(0.5);
                rect.scale(DVec3::splat(0.8))
                    .put_anchor_on(Anchor::edge(0, -1, 0), target_bc_coord);
            });
            r.insert_and_show(rect)
        })
        .collect::<Vec<_>>();

    let highlight = |rect: Rectangle| {
        rect.transform(|data| {
            data.set_color(manim::RED_C).set_fill_opacity(0.5);
        })
        .with_duration(anim_step_duration)
        .with_rate_func(linear)
    };
    let unhighlight = |rect: Rectangle| {
        rect.transform(|data| {
            data.set_color(manim::WHITE).set_fill_opacity(0.5);
        })
        .with_duration(anim_step_duration)
        .with_rate_func(linear)
    };

    let shift_right = DVec3::X * width_unit;
    for i in 0..num - 1 {
        r.timeline_mut(&r_rects[i]).play_with(highlight);
        for j in i + 1..num {
            r.timeline_mut(&r_rects[j]).play_with(highlight);
            r.timelines_mut().sync();

            if heights[i] > heights[j] {
                let dir = [shift_right, -shift_right];
                let color = [manim::BLUE_C, manim::RED_C];
                r.timeline_mut(&[&r_rects[i], &r_rects[j]])
                    .iter_mut()
                    .zip(dir)
                    .zip(color)
                    .for_each(|((timeline, dir), color)| {
                        timeline.play_with(|rect| {
                            rect.transform(|rect| {
                                rect.shift(dir * (j - i) as f64)
                                    .set_color(color)
                                    .set_fill_opacity(0.5);
                            })
                            .with_duration(anim_step_duration)
                            .with_rate_func(linear)
                        });
                    });
                heights.swap(i, j);
                r_rects.swap(i, j);
            }
            r.timeline_mut(&r_rects[j]).play_with(unhighlight);
            r.timelines_mut().sync();
        }
        r.timeline_mut(&r_rects[i]).play_with(unhighlight);
    }
}

#[scene]
#[preview]
#[output]
/// A selective sort ranim example with input of 10.
///
/// <canvas id="ranim-app-selective_sort_10" width="1280" height="720" style="width: 100%;"></canvas>
/// <script type="module">
///   const { run_selective_sort_10 } = await ranim_examples;
///   run_selective_sort_10();
/// </script>
pub fn selective_sort_10(r: &mut RanimScene) {
    selective_sort(r, 10);
}

#[scene]
#[preview]
#[output]
/// A selective sort ranim example with input of 100.
///
/// <canvas id="ranim-app-selective_sort_100" width="1280" height="720" style="width: 100%;"></canvas>
/// <script type="module">
///   const { run_selective_sort_100 } = await ranim_examples;
///   run_selective_sort_100();
/// </script>
pub fn selective_sort_100(r: &mut RanimScene) {
    selective_sort(r, 100);
}

#[cfg(any(feature = "app", target_arch = "wasm32"))]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub fn run_selective_sort_10() {
    run_scene_app(
        selective_sort_10_scene.constructor,
        selective_sort_10_scene.name.to_string(),
    );
}

#[cfg(any(feature = "app", target_arch = "wasm32"))]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub fn run_selective_sort_100() {
    run_scene_app(
        selective_sort_100_scene.constructor,
        selective_sort_100_scene.name.to_string(),
    );
}
