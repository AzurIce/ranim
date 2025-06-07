use ranim::{
    AppOptions,
    animation::{creation::WritingAnim, transform::TransformAnim},
    color::palettes::manim,
    items::vitem::{
        VItem,
        geometry::{Circle, Square},
    },
    prelude::*,
    render_scene,
};

#[scene]
struct GettingStarted1Scene;

impl SceneConstructor for GettingStarted1Scene {
    fn construct(self, r: &mut RanimScene, _r_cam: TimelineId<CameraFrame>) {
        // A Square with size 2.0 and color blue
        let square = Square::new(2.0).with(|square| {
            square.set_color(manim::BLUE_C);
        });

        let circle = Circle::new(2.0).with(|circle| {
            circle.set_color(manim::RED_C);
        });

        // In order to do more low-level opeerations,
        // sometimes we need to convert the item to a low-level item.
        let r_vitem = r.init_timeline(VItem::from(square)).id();
        {
            let timeline = r.timeline_mut(&r_vitem);
            timeline.play_with(|vitem| vitem.transform_to(VItem::from(circle.clone())));
            timeline.play_with(|vitem| vitem.unwrite());
        }
    }
}

fn main() {
    render_scene(GettingStarted1Scene, &AppOptions::default());
}
