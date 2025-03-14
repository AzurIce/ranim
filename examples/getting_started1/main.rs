use ranim::{
    AppOptions,
    animation::fading::{FadingAnim, FadingAnimSchedule},
    color::palettes::manim,
    items::vitem::Square,
    prelude::*,
    render_timeline,
};

#[scene]
struct GettingStarted1Scene;

impl TimelineConstructor for GettingStarted1Scene {
    fn construct<'t: 'r, 'r>(
        self,
        timeline: &'t RanimTimeline,
        _camera: &'r mut Rabject<'t, CameraFrame>,
    ) {
        let mut square = Square(300.0).build();
        square.set_color(manim::BLUE_C);

        let mut square = timeline.insert(square);
        timeline.play(square.fade_in().chain(|data| data.fade_out()));
    }
}

fn main() {
    render_timeline(GettingStarted1Scene, &AppOptions::default());
}
