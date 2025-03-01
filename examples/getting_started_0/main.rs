use ranim::{color::palettes::manim, items::vitem::Square, prelude::*};

#[timeline]
fn getting_started_0(timeline: &Timeline) {
    let mut square = Square(300.0).build(); // An VItem of a square
    square.set_color(manim::BLUE_C);

    timeline.forward(1.0);
    let mut _square = timeline.insert(square); // Create a "Rabject" in the timeline
    timeline.forward(1.0);
}

fn main() {
    render_timeline!(getting_started_0);
}
