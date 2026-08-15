use ranim::{
    anims::fading::FadingAnim, color::palettes::manim, items::vitem::geometry::Square, prelude::*,
    utils::rate_functions::smooth,
};

// ANCHOR: construct
#[scene]
#[wasm_demo_doc]
#[output(dir = "./output/getting_started0")]
fn getting_started0(r: &mut RanimScene) {
    // A Square with size 2.0 and color blue
    let square = Square::new(2.0).with(|square| {
        square.set_color(manim::BLUE_C);
    });

    let mut content = seq![square.clone().fade_in().with_rate_func(smooth)];
    content
        .hold(1.0)
        .push(square.hide())
        .forward(1.0)
        .push(square.show())
        .hold(1.0)
        .push(square.clone().fade_out().with_rate_func(smooth));

    r.play(
        CameraFrame::default()
            .show()
            .with_duration(content.cursor_sec()),
    );
    r.play(content);
}
// ANCHOR_END: construct
