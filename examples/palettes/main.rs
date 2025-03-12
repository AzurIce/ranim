use env_logger::Env;
use glam::vec3;
use ranim::animation::creation::Color;
use ranim::color::palettes::manim::*;
use ranim::items::vitem::Rectangle;
use ranim::timeline::timeline;
use ranim::{prelude::*, render_timeline_frame};

#[timeline]
fn palettes(ranim: Ranim) {
    let Ranim(timeline, mut _camera) = ranim;
    let (width, height) = (1920, 1080);
    let (offset_x, offset_y) = (width as f32 / -2.0, height as f32 / -2.0);
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

    let padding = 100;

    let rows = colors.len();
    let h_step = (height - 2 * padding) / rows;

    let mut squares = Vec::with_capacity(colors.len() * 5);
    for (i, row) in colors.iter().enumerate() {
        let y = padding + i * h_step;
        let cols = row.len();
        let w_step = (width - 2 * padding) / cols;
        for (j, color) in row.iter().enumerate() {
            let x = padding + j * w_step;
            let mut square = Rectangle(w_step as f32, h_step as f32).build();
            square.shift(vec3(
                x as f32 + offset_x + w_step as f32 / 2.0,
                y as f32 + offset_y + h_step as f32 / 2.0,
                0.0,
            ));
            square.set_color(*color).set_stroke_width(0.0);

            squares.push(timeline.insert(square));
        }
    }
    timeline.forward(0.01);
}

fn main() {
    #[cfg(debug_assertions)]
    env_logger::Builder::from_env(Env::default().default_filter_or("palettes=trace")).init();
    #[cfg(not(debug_assertions))]
    env_logger::Builder::from_env(Env::default().default_filter_or("palettes=info, ranim=trace"))
        .init();

    render_timeline_frame!(palettes, 0.0, "preview.png");
    render_timeline_frame!(palettes, 0.0, "output.png");
}
