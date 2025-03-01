use ranim::{
    animation::fading::FadingAnim, color::palettes::manim, items::vitem::Square, prelude::*,
};

#[timeline]
fn getting_started_1(timeline: &Timeline) {
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
