use ranim::{
    animation::{fading::FadingAnimSchedule, transform::TransformAnimSchedule},
    color::palettes::manim,
    items::vitem::{Circle, Square},
    prelude::*, timeline::TimeMark,
};

#[timeline]
fn hello_ranim(ranim: Ranim) {
    let Ranim(timeline, mut _camera) = ranim;
    let mut square = Square(300.0).build();
    square.set_color(manim::BLUE_C);
    let mut square = timeline.insert(square);

    let mut circle = Circle(300.0).build();
    circle.set_color(manim::RED_C);

    timeline.insert_time_mark(0.5, TimeMark::Capture("preview.png".to_string()));
    timeline.play(square.fade_in()).sync();

    timeline.forward(1.0);
    timeline.play(square.transform_to(circle).apply()).sync();
    timeline.forward(1.0);

    timeline.play(square.fade_out()).sync();
}

fn main() {
    render_timeline!(hello_ranim);
}
