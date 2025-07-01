use glam::{DVec3, dvec2};
use log::LevelFilter;
use rand::{SeedableRng, seq::SliceRandom};
use ranim::{
    animation::transform::TransformAnim,
    color::palettes::manim,
    components::Anchor,
    items::vitem::geometry::Rectangle,
    prelude::*,
    timeline::{TimeMark, TimelineFunc, TimelinesFunc},
    utils::rate_functions::linear,
};

#[scene]
struct SelectiveSortScene(pub usize);

impl SceneConstructor for SelectiveSortScene {
    fn construct(self, r: &mut RanimScene, _r_cam: TimelineId<CameraFrame>) {
        let num = self.0;

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
                let target_bc_coord = padded_frame_bl.extend(0.0)
                    + DVec3::X * (width_unit * i as f64 + width_unit / 2.0);
                let rect = Rectangle::new(width_unit, height).with(|rect| {
                    rect.fill_rgba = manim::WHITE.with_alpha(0.5);
                    rect.scale(DVec3::splat(0.8))
                        .put_anchor_on(Anchor::edge(0, -1, 0), target_bc_coord);
                });
                r.init_timeline(rect).with(|timeline| timeline.show()).id()
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
            r.timeline_mut(r_rects[i]).play_with(highlight);
            for j in i + 1..num {
                r.timeline_mut(r_rects[j]).play_with(highlight);
                r.timelines_mut().sync();

                if heights[i] > heights[j] {
                    let dir = [shift_right, -shift_right];
                    let color = [manim::BLUE_C, manim::RED_C];
                    r.timeline_mut(&[r_rects[i], r_rects[j]])
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
                r.timeline_mut(r_rects[j]).play_with(unhighlight);
                r.timelines_mut().sync();
            }
            r.timeline_mut(r_rects[i]).play_with(unhighlight);
        }

        r.insert_time_mark(
            r.timelines().max_total_secs() / 2.0,
            TimeMark::Capture(format!("preview-{num}.png")),
        );
    }
}

fn main() {
    #[cfg(not(target_arch = "wasm32"))]
    {
        #[cfg(debug_assertions)]
        pretty_env_logger::formatted_timed_builder()
            .filter(Some("ranim"), LevelFilter::Trace)
            .init();
        #[cfg(not(debug_assertions))]
        pretty_env_logger::formatted_timed_builder()
            .filter(Some("ranim"), LevelFilter::Info)
            .init();
    }

    #[cfg(feature = "app")]
    run_scene_app(SelectiveSortScene(100));
    #[cfg(not(feature = "app"))]
    {
        render_scene(
            SelectiveSortScene(10),
            &AppOptions {
                output_filename: "output-10.mp4",
                ..Default::default()
            },
        );
        render_scene(
            SelectiveSortScene(100),
            &AppOptions {
                output_filename: "output-100.mp4",
                ..Default::default()
            },
        );
    }
}
