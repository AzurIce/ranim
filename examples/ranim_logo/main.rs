use std::time::Duration;

use bevy_color::{ColorToPacked, Srgba};
use glam::vec2;
use image::{ImageBuffer, Rgba};
use ranim::{
    animation::creation,
    prelude::*,
    rabject::rabject2d::vmobject::{geometry::Square, svg::Svg, VMobject},
    scene::{Scene, SceneBuilder},
    typst_svg, utils::get_texture_data,
};

fn main() {
    let mut scene = SceneBuilder::new("ranim_logo")
        .with_size((600, 600))
        .build();
    let mut canvas = scene.insert_new_canvas(600, 600);
    scene.center_canvas_in_frame(&canvas);

    let center = vec2(300.0, 300.0);

    let mut letter_r =
        Svg::from_svg(&typst_svg!(r##"#text(210pt, fill: rgb("#ffaa33"))[R]"##)).build();
    letter_r
        // .set_fill_color(Srgba::from_u8_array([0xff, 0xaa, 0x33, 0x00]))
        .set_fill_opacity(0.3);
    letter_r
        .set_stroke_width(4.0)
        // .set_stroke_color(Srgba::from_u8_array([0xff, 0xaa, 0x33, 0x00]))
        .set_stroke_opacity(1.0);
    letter_r.shift(center - letter_r.bounding_box().center());

    let mut square = Square::new(100.0).build();
    square
        .set_fill_color(Srgba::from_u8_array([0xff, 0xaa, 0x33, 0x00]))
        .set_fill_opacity(0.3);
    square
        .set_stroke_width(4.0)
        .set_stroke_color(Srgba::from_u8_array([0xff, 0xaa, 0x33, 0x00]))
        .set_stroke_opacity(1.0);
    square.shift(center - square.bounding_box().center());
    let square = scene.get_mut(&canvas).insert(square);

    scene.wait(Duration::from_secs_f32(0.2));
    scene.play_in_canvas(&canvas, letter_r, creation::write());
    scene.wait(Duration::from_secs_f32(0.2));

    scene.render_to_image("output.png");
    let texture_data = get_texture_data(&scene.ctx.wgpu_ctx, &scene.get(&canvas).camera.vello_texture);
    let buffer: ImageBuffer<Rgba<u8>, Vec<u8>> = ImageBuffer::from_raw(600, 600, texture_data).unwrap();
    buffer.save("vello_texture.png").unwrap();
    let texture_data = get_texture_data(&scene.ctx.wgpu_ctx, &scene.get(&canvas).camera.render_texture);
    let buffer: ImageBuffer<Rgba<u8>, Vec<u8>> = ImageBuffer::from_raw(600, 600, texture_data).unwrap();
    buffer.save("render_texture.png").unwrap();
}
