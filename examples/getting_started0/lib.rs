use ranim::{
    anims::fading::FadingAnim, color::palettes::manim, items::vitem::geometry::Square, prelude::*,
};

// ANCHOR: construct
#[scene]
#[output(dir = "./output/getting_started0")]
fn getting_started0(r: &mut RanimScene) {
    // A Square with size 2.0 and color blue
    let square = Square::new(2.0).with(|square| {
        square.set_color(manim::BLUE_C);
    });

    let mut content = AnimSequence::new();
    content
        .play(square.clone().fade_in())
        .hold(1.0)
        .play(square.hide())
        .forward(1.0)
        .play(square.show())
        .hold(1.0)
        .play(square.clone().fade_out());

    let mut camera = AnimSequence::new();
    camera
        .play(CameraFrame::default().show())
        .hold_to(content.cursor_sec());
    r.play(stack![camera, content]);
}
// ANCHOR_END: construct
