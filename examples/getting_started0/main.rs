use ranim::{color::palettes::manim, items::vitem::Square, prelude::*};

#[scene]
struct GettingStarted0Scene;

impl TimelineConstructor for GettingStarted0Scene {
    fn construct<'t: 'r, 'r>(
        self,
        timeline: &'t RanimTimeline,
        _camera: &'r mut Rabject<'t, CameraFrame>,
    ) {
        let mut square = Square(2.0).build(); // An VItem of a square
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
}

fn main() {
    build_and_render_timeline(GettingStarted0Scene, &AppOptions::default());
}
