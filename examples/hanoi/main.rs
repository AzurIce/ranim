use log::LevelFilter;
use ranim::{
    animation::transform::TransformAnim,
    color::{HueDirection, palettes::manim},
    components::Anchor,
    glam::dvec3,
    items::vitem::geometry::Rectangle,
    prelude::*,
    timeline::{TimelineFunc, TimelinesFunc},
    utils::rate_functions::{ease_in_quad, ease_out_quad, linear},
};

fn solve_hanoi(
    n: usize,
    idx_src: usize,
    idx_dst: usize,
    idx_tmp: usize,
    move_disk: &mut impl FnMut(usize, usize),
) {
    if n == 1 {
        move_disk(idx_src, idx_dst);
    } else {
        solve_hanoi(n - 1, idx_src, idx_tmp, idx_dst, move_disk);
        move_disk(idx_src, idx_dst);
        solve_hanoi(n - 1, idx_tmp, idx_dst, idx_src, move_disk);
    }
}

#[scene]
struct HanoiScene(pub usize);

impl SceneConstructor for HanoiScene {
    fn construct(self, r: &mut RanimScene, _r_cam: ItemId<CameraFrame>) {
        let n = self.0;
        let total_sec = 10.0;
        let rod_width = 0.4;
        let rod_height = 5.0;
        let rod_section_width = 4.0;

        let _rods = [-1, 0, 1]
            .into_iter()
            .map(|i| {
                Rectangle::new(rod_width, rod_height).with(|rect| {
                    rect.set_color(manim::GREY_C).put_anchor_on(
                        Anchor::edge(0, -1, 0),
                        dvec3(i as f64 * rod_section_width, -4.0, 0.0),
                    );
                })
            })
            .map(|rect| r.init_timeline(rect).with(|timeline| timeline.show()).id())
            .collect::<Vec<_>>();

        let min_disk_width = rod_width * 1.7;
        let max_disk_width = rod_section_width * 0.8;
        let disk_height = (rod_height * 0.8) / n as f64;
        let _disks = (0..n)
            .map(|i| {
                let factor = i as f64 / (n - 1) as f64;
                let disk_width =
                    min_disk_width + (max_disk_width - min_disk_width) * (1.0 - factor);
                r.init_timeline(Rectangle::new(disk_width, disk_height).with(|rect| {
                    let color =
                        manim::RED_D.lerp(manim::BLUE_D, factor as f32, HueDirection::Increasing);
                    rect.stroke_width = 0.0;
                    rect.set_color(color).put_anchor_on(
                        Anchor::edge(0, -1, 0),
                        dvec3(-rod_section_width, -4.0 + disk_height * i as f64, 0.0),
                    );
                }))
                .with(|timeline| timeline.show())
                .id()
            })
            .collect::<Vec<_>>();

        let mut r_disks = [_disks, Vec::new(), Vec::new()];

        let anim_duration = total_sec / (2.0f64.powi(n as i32) - 1.0) / 3.0;
        let mut move_disk = |idx_src: usize, idx_dst: usize| {
            let top_disk_y = |idx: usize| r_disks[idx].len() as f64 * disk_height - 4.0;
            let top_src = top_disk_y(idx_src) - disk_height;
            let top_dst = top_disk_y(idx_dst);
            let r_disk = r_disks[idx_src].pop().unwrap();

            {
                let timeline = r.timeline_mut(r_disk);
                timeline.play_with(|disk| {
                    disk.transform(|data| {
                        data.shift(dvec3(0.0, 3.0 - top_src, 0.0));
                    })
                    .with_duration(anim_duration)
                    .with_rate_func(ease_in_quad)
                });
                timeline.play_with(|disk| {
                    disk.transform(|data| {
                        data.shift(dvec3(
                            (idx_dst as f64 - idx_src as f64) * rod_section_width,
                            0.0,
                            0.0,
                        ));
                    })
                    .with_duration(anim_duration)
                    .with_rate_func(linear)
                });
                timeline.play_with(|disk| {
                    disk.transform(|data| {
                        data.shift(dvec3(0.0, top_dst - 3.0, 0.0));
                    })
                    .with_duration(anim_duration)
                    .with_rate_func(ease_out_quad)
                });
            }
            r.timelines_mut().sync();
            r_disks[idx_dst].push(r_disk);
        };

        solve_hanoi(n, 0, 1, 2, &mut move_disk);
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
    run_scene_app(HanoiScene(10));
    #[cfg(not(feature = "app"))]
    {
        render_scene(
            HanoiScene(5),
            &AppOptions {
                output_filename: "output-5.mp4",
                ..Default::default()
            },
        );
        render_scene_at_sec(HanoiScene(5), 0.0, "preview-5.png", &AppOptions::default());
        render_scene(
            HanoiScene(10),
            &AppOptions {
                output_filename: "output-10.mp4",
                ..Default::default()
            },
        );
        render_scene_at_sec(
            HanoiScene(10),
            0.0,
            "preview-10.png",
            &AppOptions::default(),
        );
    }
}
