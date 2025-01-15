use std::time::Duration;

use glam::vec2;
use ranim::{
    animation::creation, color::palettes::manim::WHITE, prelude::*, rabject::rabject2d::vmobject::svg::Svg, scene::SceneBuilder, typst_svg, utils::get_texture_data
};

fn main() {
    let mut scene = SceneBuilder::new("ranim_logo")
        .with_size((600, 600))
        .build();
    scene.set_clear_color(WHITE);

    let canvas = scene.insert_new_canvas(600, 600);
    scene.center_canvas_in_frame(&canvas);

    let center = vec2(300.0, 300.0);

    let mut letter_r =
        Svg::from_svg(&typst_svg!(r##"
            #text(100pt, font: "KaTex_AMS", fill: rgb("#ffaa33"))[R]
            #text(80pt, fill: rgb("#ffaa33"), weight: "bold")[anim]
        "##)).build();
    // letter_r.set_fill_opacity(0.3);
    // letter_r.set_stroke_width(4.0).set_stroke_opacity(1.0);
    letter_r.shift(center - letter_r.bounding_box().center());

    scene.wait(Duration::from_secs_f32(0.2));
    scene.play_in_canvas(&canvas, letter_r, creation::write());
    scene.wait(Duration::from_secs_f32(0.2));

    scene.render_to_image("output.png");
}
