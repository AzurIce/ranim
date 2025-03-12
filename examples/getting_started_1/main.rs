use ranim::{
    animation::fading::FadingAnimSchedule, color::palettes::manim, items::vitem::Square, prelude::*,
};

#[timeline]
fn getting_started_1(ranim: Ranim) {
    let Ranim(timeline, mut _camera) = ranim;

    let mut square = Square(300.0).build();
    square.set_color(manim::BLUE_C);

    timeline.forward(1.0);
    let mut square = timeline.insert(square);
    timeline.play(square.fade_in()); // Create an `AnimSchedule` and play it
    timeline.forward(1.0);
}

fn main() {
    render_timeline!(getting_started_1);
}
