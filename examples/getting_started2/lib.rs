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
    utils::rate_functions::{linear, smooth},
};

#[scene]
#[wasm_demo_doc]
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
    let mut rect_sequence = seq![rect.clone().show()];
    rect_sequence
        .hold(1.0)
        .push(VItem::from(rect).uncreate().with_rate_func(smooth));

    let mut item_sequence = seq![square.clone().show()];
    item_sequence
        .hold(1.0)
        .push(square.clone().create().with_rate_func(smooth))
        .push(
            square
                .clone()
                .morph_to(circle.clone())
                .with_rate_func(linear),
        )
        .push(circle.clone().unwrite().with_rate_func(smooth));

    let total_secs = item_sequence.cursor_sec().max(rect_sequence.cursor_sec());
    r.play(CameraFrame::default().show().with_duration(total_secs));
    r.play(stack![rect_sequence, item_sequence]);
    r.insert_time_mark(
        total_secs / 2.0,
        TimeMark::Capture("preview.png".to_string()),
    );
}
