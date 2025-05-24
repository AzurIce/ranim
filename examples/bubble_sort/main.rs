use itertools::Itertools;
use rand::{SeedableRng, seq::SliceRandom};
use ranim::{
    animation::transform::TransformAnim,
    color::palettes::manim,
    components::Anchor,
    glam::{DVec3, dvec2},
    items::vitem::geometry::Rectangle,
    prelude::*,
    timeline::TimeMark,
    utils::rate_functions::linear,
};

#[scene]
struct BubbleSortScene(pub usize);

impl TimelineConstructor for BubbleSortScene {
    fn construct(self, timeline: &RanimTimeline, _camera: PinnedItem<CameraFrame>) {
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
        let mut rects = heights
            .iter()
            .enumerate()
            .map(|(i, &height)| {
                let target_bc_coord = padded_frame_bl.extend(0.0)
                    + DVec3::X * (width_unit * i as f64 + width_unit / 2.0);
                let rect = Rectangle::new(width_unit, height).with(|rect| {
                    rect.stroke_width = 0.0;
                    rect.set_fill_color(manim::WHITE.with_alpha(0.5))
                        .scale(DVec3::splat(0.8))
                        .put_anchor_on(Anchor::edge(0, -1, 0), target_bc_coord);
                });
                Some(timeline.pin(rect))
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
        let anim_swap = |rects: [Rectangle; 2]| {
            rects
                .into_iter()
                .zip(swap_shift.iter())
                .map(|(rect, shift)| {
                    rect.transform(|data| {
                        data.shift(*shift);
                    })
                    .with_duration(anim_step_duration)
                    .with_rate_func(linear)
                })
                .collect_array()
                .unwrap()
        };

        for i in (1..num).rev() {
            for j in 0..i {
                let rect_ab = [
                    timeline.unpin(rects[j].take().unwrap()),
                    timeline.unpin(rects[j + 1].take().unwrap()),
                ];

                let mut rect_ab = timeline.play(rect_ab.map(anim_highlight));
                if heights[j] > heights[j + 1] {
                    rect_ab = timeline.play(anim_swap(rect_ab));
                    timeline.sync();
                    heights.swap(j, j + 1);
                    rect_ab.swap(0, 1);
                }
                let [rect_a, rect_b] = timeline.play(rect_ab.map(anim_unhighlight));

                rects[j] = Some(timeline.pin(rect_a));
                rects[j + 1] = Some(timeline.pin(rect_b));
            }
        }

        timeline.insert_time_mark(
            timeline.cur_sec() / 2.0,
            TimeMark::Capture(format!("preview-{num}.png")),
        );
    }
}

fn main() {
    #[cfg(feature = "app")]
    run_scene_app(BubbleSortScene(100));
    #[cfg(not(feature = "app"))]
    {
        render_scene(
            BubbleSortScene(10),
            &AppOptions {
                output_filename: "output-10.mp4",
                ..Default::default()
            },
        );
        render_scene(
            BubbleSortScene(100),
            &AppOptions {
                output_filename: "output-100.mp4",
                ..Default::default()
            },
        );
    }
}
