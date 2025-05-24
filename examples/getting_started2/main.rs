use ranim::{
    animation::{
        creation::{CreationAnim, WritingAnim},
        transform::TransformAnim,
    },
    color::palettes::manim,
    items::vitem::{
        VItem,
        geometry::{Circle, Rectangle, Square},
    },
    prelude::*,
    utils::rate_functions::linear,
};

#[scene]
struct GettingStarted2Scene;

impl TimelineConstructor for GettingStarted2Scene {
    fn construct(self, timeline: &RanimTimeline, _camera: PinnedItem<CameraFrame>) {
        let rect = Rectangle::new(4.0, 9.0 / 4.0).with(|rect| {
            rect.set_stroke_color(manim::GREEN_C);
        });

        // Use pin to keep the item static showed
        let rect = timeline.pin(rect);
        timeline.forward(1.0);

        let square = Square::new(2.0).with(|square| {
            square.set_color(manim::BLUE_C);
        });

        let circle = Circle::new(2.0).with(|circle| {
            circle.set_color(manim::RED_C);
        });
        {
            let square = timeline.play(VItem::from(square).create());
            timeline.play(square.transform_to(circle.clone()).with_rate_func(linear));
        }
        timeline.play(VItem::from(circle).unwrite());

        // Use unpin to remove the static showed item and turn it back to normal
        let rect = timeline.unpin(rect);
        timeline.play(VItem::from(rect).uncreate());
    }
}

fn main() {
    render_scene(GettingStarted2Scene, &AppOptions::default());
}
