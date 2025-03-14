use ranim::{
    animation::{fading::FadingAnimSchedule, transform::TransformAnimSchedule},
    color::palettes::manim,
    items::vitem::{Circle, Square},
    prelude::*,
    utils::rate_functions::linear,
};

#[scene]
struct GettingStarted2Scene;

impl TimelineConstructor for GettingStarted2Scene {
    fn construct<'t: 'r, 'r>(
        self,
        timeline: &'t RanimTimeline,
        _camera: &'r mut Rabject<'t, CameraFrame>,
    ) {
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
}

fn main() {
    render_timeline(GettingStarted2Scene, &AppOptions::default());
}
