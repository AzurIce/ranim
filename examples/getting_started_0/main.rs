use ranim::{color::palettes::manim, items::vitem::Square, prelude::*};

#[timeline]
fn getting_started_0(ranim: Ranim) {
    let Ranim(timeline, mut _camera) = ranim;

    let mut square = Square(300.0).build(); // An VItem of a square
    square.set_color(manim::BLUE_C);

    timeline.forward(0.5);
    let square = timeline.insert(square); // Create a "Rabject" in the timeline
    timeline.forward(0.5); // By default the rabject timeline is at "show" state
    timeline.hide(&square);
    timeline.forward(0.5); // After called "hide", the forward will encode blank into timeline

    timeline.show(&square);
    timeline.forward(0.5);

    drop(square); // The drop is equal to `timeline.hide(&rabject)`
    timeline.forward(0.5);
}

fn main() {
    render_timeline!(getting_started_0);
}
