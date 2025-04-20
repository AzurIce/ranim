use ranim::{color::palettes::manim, items::vitem::Square, prelude::*};

#[scene]
struct GettingStarted0Scene;

impl TimelineConstructor for GettingStarted0Scene {
    fn construct(
        self,
        timeline: &RanimTimeline,
        _camera: &mut Rabject<CameraFrame>,
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

        timeline.remove(square); // Currently is equal to `timeline.hide(&rabject)`, but takes the owner ship
        timeline.forward(0.5);
    }
}

fn main() {
    render_scene(GettingStarted0Scene, &AppOptions::default());
}
