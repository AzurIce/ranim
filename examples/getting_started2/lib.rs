use ranim::{
    anims::{
        creation::{CreationAnim, WritingAnim},
        morph::MorphAnim,
    },
    color::palettes::manim,
    items::vitem::{
        VItem,
        geometry::{Circle, Rectangle, Square},
    },
    prelude::*,
    utils::rate_functions::linear,
};

#[scene]
#[output(dir = "./output/getting_started2")]
fn getting_started2(r: &mut RanimScene) {
    let rect = Rectangle::new(4.0, 9.0 / 4.0).with(|rect| {
        rect.set_stroke_color(manim::GREEN_C);
    });

    let square: VItem = Square::new(2.0)
        .with(|square| {
            square.set_color(manim::BLUE_C);
        })
        .into();
    let circle: VItem = Circle::new(2.0)
        .with(|circle| {
            circle.set_color(manim::RED_C);
        })
        .into();
    let mut rect_sequence = AnimSequence::new();
    rect_sequence
        .push(rect.clone().show())
        .hold(1.0)
        .push(VItem::from(rect).uncreate());

    let mut item_sequence = AnimSequence::new();
    item_sequence
        .push(square.clone().show())
        .hold(1.0)
        .push(square.clone().create())
        .push(
            square
                .clone()
                .morph_to(circle.clone())
                .with_rate_func(linear),
        )
        .push(circle.clone().unwrite());

    let total_secs = item_sequence.cursor_sec().max(rect_sequence.cursor_sec());
    let mut camera = AnimSequence::new();
    camera
        .push(CameraFrame::default().show())
        .hold_to(total_secs);
    r.play(camera);
    r.play(rect_sequence);
    r.play(item_sequence);
}
