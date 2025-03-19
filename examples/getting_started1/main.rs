use ranim::{
    AppOptions,
    animation::fading::{FadingAnim, FadingAnimSchedule},
    build_and_render_timeline,
    color::palettes::manim,
    items::vitem::Square,
    prelude::*,
};

#[scene]
struct GettingStarted1Scene;

impl TimelineConstructor for GettingStarted1Scene {
    fn construct<'t: 'r, 'r>(
        self,
        timeline: &'t RanimTimeline,
        _camera: &'r mut Rabject<'t, CameraFrame>,
    ) {
        let mut square = Square(2.0).build();
        square.set_color(manim::BLUE_C);

        let mut square = timeline.insert(square);
        #[allow(deprecated)]
        timeline.play(square.fade_in().chain(|data| data.fade_out()));
    }
}

fn main() {
    build_and_render_timeline(GettingStarted1Scene, &AppOptions::default());
}
