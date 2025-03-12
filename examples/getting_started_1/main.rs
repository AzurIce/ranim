use ranim::{
    animation::fading::{FadingAnim, FadingAnimSchedule},
    color::palettes::manim,
    items::vitem::Square,
    prelude::*,
};

#[timeline]
fn getting_started_1(ranim: Ranim) {
    let Ranim(timeline, mut _camera) = ranim;

    let mut square = Square(300.0).build();
    square.set_color(manim::BLUE_C);

    let mut square = timeline.insert(square);
    timeline.play(square.fade_in().chain(|data| data.fade_out()));
}

fn main() {
    render_timeline!(getting_started_1);
}
