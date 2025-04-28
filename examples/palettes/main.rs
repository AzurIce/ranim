use log::LevelFilter;
use ranim::color::palettes::manim::*;
use ranim::components::Anchor;
use ranim::glam::{dvec2, dvec3};
use ranim::items::group::Group;
use ranim::items::vitem::Rectangle;
use ranim::prelude::*;

#[scene]
struct PalettesScene;

impl TimelineConstructor for PalettesScene {
    fn construct(self, timeline: &RanimTimeline, _camera: &mut Rabject<CameraFrame>) {
        let frame_size = dvec2(8.0 * 16.0 / 9.0, 8.0);
        let padded_frame_size = frame_size * 0.9;

        let colors = vec![
            vec![BLUE_E, BLUE_D, BLUE_C, BLUE_B, BLUE_A],
            vec![TEAL_E, TEAL_D, TEAL_C, TEAL_B, TEAL_A],
            vec![GREEN_E, GREEN_D, GREEN_C, GREEN_B, GREEN_A],
            vec![YELLOW_E, YELLOW_D, YELLOW_C, YELLOW_B, YELLOW_A],
            vec![GOLD_E, GOLD_D, GOLD_C, GOLD_B, GOLD_A],
            vec![RED_E, RED_D, RED_C, RED_B, RED_A],
            vec![MAROON_E, MAROON_D, MAROON_C, MAROON_B, MAROON_A],
            vec![PURPLE_E, PURPLE_D, PURPLE_C, PURPLE_B, PURPLE_A],
            vec![GREY_E, GREY_D, GREY_C, GREY_B, GREY_A],
            vec![WHITE, BLACK, GREEN_SCREEN],
            vec![GREY_BROWN, LIGHT_BROWN, PINK, LIGHT_PINK, ORANGE],
        ];

        let padded_frame_start = dvec2(padded_frame_size.x / -2.0, padded_frame_size.y / -2.0);
        let h_step = padded_frame_size.y / colors.len() as f64;

        let squares = colors
            .iter()
            .enumerate()
            .flat_map(|(i, row)| {
                let y = i as f64 * h_step;
                let w_step = padded_frame_size.x / row.len() as f64;
                row.iter().enumerate().map(move |(j, color)| {
                    let x = j as f64 * w_step;
                    let mut square = Rectangle(w_step as f64, h_step as f64).build();
                    square
                        .put_anchor_on(
                            Anchor::edge(-1, -1, 0),
                            padded_frame_start.extend(0.0) + dvec3(x, y, 0.0),
                        )
                        .set_color(*color)
                        .set_stroke_width(0.0);
                    square
                })
            })
            .collect::<Group<_>>();
        let _squares = timeline.insert(squares);
        timeline.forward(0.01);
    }
}

fn main() {
    #[cfg(debug_assertions)]
    pretty_env_logger::formatted_timed_builder()
        .filter(Some("ranim"), LevelFilter::Trace)
        .init();
    #[cfg(not(debug_assertions))]
    pretty_env_logger::formatted_timed_builder()
        .filter(Some("ranim"), LevelFilter::Info)
        .init();

    let options = AppOptions::default();
    render_scene_at_sec(PalettesScene, 0.0, "preview.png", &options);
    render_scene_at_sec(PalettesScene, 0.0, "output.png", &options);
}
