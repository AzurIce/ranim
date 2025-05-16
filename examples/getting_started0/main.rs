use ranim::{
    animation::fading::FadingAnim, color::palettes::manim, items::vitem::geometry::Square,
    prelude::*,
};

#[scene]
struct GettingStarted0Scene;

impl TimelineConstructor for GettingStarted0Scene {
    fn construct(self, timeline: &RanimTimeline, _camera: PinnedItem<CameraFrame>) {
        // A Square with size 2.0 and color blue
        let square = Square::new(2.0).with(|square| {
            square.fill_rgba = manim::BLUE_C;
            square.stroke_rgba = manim::BLUE_C;
        });

        // Plays the animation
        timeline.play(square.clone().fade_in());
        timeline.play(square.fade_out());

        // The play method returns the result of the animation,
        // so it can also be written like this:
        // let square = timeline.play(square.fade_in());
        // timeline.play(square.fade_out());
    }
}

fn main() {
    render_scene(GettingStarted0Scene, &AppOptions::default());
}
