use ranim::{
    AppOptions,
    animation::{
        creation::{CreationAnim, WritingAnim},
        transform::TransformAnim,
    },
    color::palettes::manim,
    items::vitem::{Circle, VItem, geometry::Square},
    prelude::*,
    render_scene,
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

        let circle = Circle(2.0).build().with(|circle| {
            circle.set_color(manim::RED_C);
        });

        // In order to do more low-level opeerations,
        // sometimes we need to convert the item to a low-level item.
        {
            let square = timeline.play(VItem::from(square).create());
            timeline.play(square.transform_to(circle.clone()));
        }
        timeline.play(circle.unwrite());
    }
}

fn main() {
    render_scene(GettingStarted1Scene, &AppOptions::default());
}
