use ranim::{
    color::palettes::manim::*,
    glam::{dvec2, dvec3},
    items::vitem::geometry::Rectangle,
    prelude::*,
};

#[scene]
#[output(dir = "palettes")]
fn palettes(r: &mut RanimScene) {
    let _r_cam = r.insert(CameraFrame::default());
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
                Rectangle::new(w_step as f64, h_step as f64).with(|rect| {
                    rect.stroke_width = 0.0;

                    rect.set_color(*color).put_anchor_on(
                        Anchor::edge(-1, -1, 0),
                        padded_frame_start.extend(0.0) + dvec3(x, y, 0.0),
                    );
                })
            })
        })
        .collect::<Vec<_>>();
    r.insert(squares);
    r.insert_time_mark(0.0, TimeMark::Capture("preview.png".to_string()));
    r.timelines_mut().forward(0.01);
}
