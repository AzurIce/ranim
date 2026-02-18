#[cfg(target_arch = "wasm32")]
use ranim_app as _;

use glam::{DVec3, dvec2};
use rand::{SeedableRng, seq::SliceRandom};
use ranim::glam::{self, dvec3};
use ranim::{
    anims::morph::MorphAnim, color::palettes::manim, items::vitem::geometry::Rectangle, prelude::*,
    utils::rate_functions::linear,
};

fn selective_sort(r: &mut RanimScene, num: usize) {
    let _r_cam = r.insert(CameraFrame::default());

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
                    .move_anchor_to(AabbPoint(dvec3(0.0, -1.0, 0.0)), target_bc_coord);
            });
            (r.insert(rect.clone()), rect)
        })
        .collect::<Vec<_>>();

    let highlight = |rect: &mut Rectangle| {
        rect.morph(|data| {
            data.set_color(manim::RED_C).set_fill_opacity(0.5);
        })
        .with_duration(anim_step_duration)
        .with_rate_func(linear)
    };
    let unhighlight = |rect: &mut Rectangle| {
        rect.morph(|data| {
            data.set_color(manim::WHITE).set_fill_opacity(0.5);
        })
        .with_duration(anim_step_duration)
        .with_rate_func(linear)
    };

    let shift_right = DVec3::X * width_unit;
    for i in 0..num - 1 {
        r.timeline_mut(r_rects[i].0)
            .play(highlight(&mut r_rects[i].1));
        for j in i + 1..num {
            r.timeline_mut(r_rects[j].0)
                .play(highlight(&mut r_rects[j].1));
            r.timelines_mut().sync();

            if heights[i] > heights[j] {
                let dir = [shift_right, -shift_right];
                let color = [manim::BLUE_C, manim::RED_C];
                r.timeline_mut(r_rects.get_disjoint_mut([i, j]).unwrap())
                    .unwrap()
                    .into_iter()
                    .zip(dir)
                    .zip(color)
                    .for_each(|(((timeline, rect), dir), color)| {
                        timeline.play(
                            rect.morph(|rect| {
                                rect.shift(dir * (j - i) as f64)
                                    .set_color(color)
                                    .set_fill_opacity(0.5);
                            })
                            .with_duration(anim_step_duration)
                            .with_rate_func(linear),
                        );
                    });
                heights.swap(i, j);
                r_rects.swap(i, j);
            }
            r.timeline_mut(r_rects[j].0)
                .play(unhighlight(&mut r_rects[j].1));
            r.timelines_mut().sync();
        }
        r.timeline_mut(r_rects[i].0)
            .play(unhighlight(&mut r_rects[i].1));
    }

    r.insert_time_mark(
        r.timelines().max_total_secs() / 2.0,
        TimeMark::Capture(format!("preview-{num}.png")),
    );
}

#[scene]
#[output(dir = "selective_sort")]
fn selective_sort_10(r: &mut RanimScene) {
    selective_sort(r, 10);
}

#[scene(name = "selective_sort")]
#[output(dir = "selective_sort")]
fn selective_sort_100(r: &mut RanimScene) {
    selective_sort(r, 100);
}
