use ranim::{
    animation::{
        creation::WritingAnimSchedule, fading::FadingAnimSchedule, transform::TransformAnimSchedule,
    },
    color::palettes::manim,
    items::vitem::{Circle, Square},
    prelude::*,
};

#[timeline]
fn getting_started_3(ranim: Ranim) {
    let Ranim(timeline, mut _camera) = ranim;
    let mut square = Square(300.0).build();
    square.set_color(manim::BLUE_C);

    let mut square = timeline.insert(square);
    let mut circle = Circle(300.0).build();
    circle.set_color(manim::RED_C);

    timeline.play(square.transform_to(circle).apply()); // Use `apply` on an anim schedule to update rabject data
    timeline.play(square.unwrite()); // Do not use `apply` to keep the data in Rabject not changed
    timeline.play(square.write());
    timeline.play(square.fade_out());
}

fn main() {
    render_timeline!(getting_started_3);
}
