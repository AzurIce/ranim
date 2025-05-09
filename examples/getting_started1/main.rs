use ranim::{
    AppOptions, animation::fading::FadingAnimSchedule, color::palettes::manim,
    items::vitem::Square, prelude::*, render_scene,
};

#[scene]
struct GettingStarted1Scene;

impl TimelineConstructor for GettingStarted1Scene {
    fn construct(self, timeline: &RanimTimeline, _camera: &mut Rabject<CameraFrame>) {
        let mut square = Square(2.0).build();
        square.set_color(manim::BLUE_C);

        let mut square = timeline.insert(square);
        #[allow(deprecated)]
        timeline.play(square.fade_in());
        timeline.play(square.fade_out());
    }
}

fn main() {
    render_scene(GettingStarted1Scene, &AppOptions::default());
}
