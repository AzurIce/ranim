use ranim::{
    anims::{creation::WritingAnim, morph::MorphAnim},
    color::palettes::manim,
    items::vitem::{
        VItem,
        geometry::{Circle, Square},
    },
    prelude::*,
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

    let mut content = AnimSequence::new();
    content
        .play(VItem::from(square).morph_to(VItem::from(circle.clone())))
        .play(VItem::from(circle).unwrite());

    let mut camera = AnimSequence::new();
    camera
        .play(CameraFrame::default().show())
        .hold_to(content.cursor_sec());
    r.play(stack![camera, content]);
}
// ANCHOR_END: construct
