use ranim::{
    animation::{
        creation::{CreationAnim, WritingAnim},
        transform::TransformAnim,
    }, color::palettes::manim, items::vitem::{geometry::{Circle, Square}, VItem}, prelude::*, render_scene, AppOptions
};

#[scene]
struct GettingStarted1Scene;

impl TimelineConstructor for GettingStarted1Scene {
    fn construct(self, timeline: &RanimTimeline, _camera: PinnedItem<CameraFrame>) {
        // A Square with size 2.0 and color blue
        let square = Square::new(2.0).with(|square| {
            square.fill_rgba = manim::BLUE_C;
            square.stroke_rgba = manim::BLUE_C;
        });

        let circle = Circle::new(2.0).with(|circle| {
            circle.fill_rgba = manim::RED_C;
            circle.stroke_rgba = manim::RED_C;
        });

        // In order to do more low-level opeerations,
        // sometimes we need to convert the item to a low-level item.
        {
            let square = timeline.play(VItem::from(square).create());
            timeline.play(square.transform_to(circle.clone()));
        }
        timeline.play(VItem::from(circle).unwrite());
    }
}

fn main() {
    render_scene(GettingStarted1Scene, &AppOptions::default());
}
