use ranim::{
    animation::{fading::FadingAnim, transform::TransformAnim},
    color::palettes::manim,
    items::vitem::{Circle, Square},
    prelude::*,
    utils::rate_functions::linear,
};

#[timeline]
fn getting_started_2(ranim: Ranim) {
    let Ranim(timeline, mut _camera) = ranim;
    let mut square = Square(300.0).build();
    square.set_color(manim::BLUE_C);

    let mut square = timeline.insert(square);
    let mut circle = Circle(300.0).build();
    circle.set_color(manim::RED_C);

    timeline.play(
        square
            .transform_to(circle)
            .with_duration(2.0)
            .with_rate_func(linear),
    ); // Anim Schedule won't change the data in Rabject
    timeline.forward(1.0);
    timeline.play(square.fade_out()); // Anim is created based on the data in Rabject
}

fn main() {
    render_timeline!(getting_started_2);
}
