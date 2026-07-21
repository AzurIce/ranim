use ranim::{
    anims::{creation::WritingAnim, morph::MorphAnim},
    color::palettes::manim,
    items::vitem::{
        VItem,
        geometry::{Circle, Square},
    },
    prelude::*,
    utils::rate_functions::smooth,
};

// ANCHOR: construct
#[scene]
#[output(dir = "./output/getting_started1")]
fn getting_started1(r: &mut RanimScene) {
    // A Square with size 2.0 and color blue
    let square = Square::new(2.0).with(|square| {
        square.set_color(manim::BLUE_C);
    });

    let circle = Circle::new(2.0).with(|circle| {
        circle.set_color(manim::RED_C);
    });

    let content = seq![
        VItem::from(square)
            .morph_to(VItem::from(circle.clone()))
            .with_rate_func(smooth),
        VItem::from(circle).unwrite().with_rate_func(smooth),
    ];

    r.play(
        CameraFrame::default()
            .show()
            .with_duration(content.cursor_sec()),
    );
    r.play(content);
}
// ANCHOR_END: construct
