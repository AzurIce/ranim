use log::LevelFilter;
use ranim::{
    animation::{creation::WritingAnim, fading::FadingAnim, transform::TransformAnim},
    color::palettes::manim,
    items::vitem::{
        VItem,
        geometry::{Circle, Square},
    },
    prelude::*,
};

// ANCHOR: construct
#[scene]
struct GettingStarted1Scene;

impl SceneConstructor for GettingStarted1Scene {
    fn construct(self, r: &mut RanimScene, _r_cam: ItemId<CameraFrame>) {
        // A Square with size 2.0 and color blue
        let square = Square::new(2.0).with(|square| {
            square.set_color(manim::BLUE_C);
        });

        let r_square = r.insert(square);
        r.timeline_mut(&r_square).play_with(|s| s.fade_in());

        let circle = Circle::new(2.0).with(|circle| {
            circle.set_color(manim::RED_C);
        });

        // In order to do more low-level opeerations,
        // sometimes we need to convert the item to a low-level item.
        let r_vitem = r.map_with(r_square, VItem::from);
        r.timeline_mut(&r_vitem)
            .play_with(|vitem| vitem.transform_to(circle.into()))
            .play_with(|vitem| vitem.unwrite());
    }
}
// ANCHOR_END: construct

fn main() {
    #[cfg(not(target_arch = "wasm32"))]
    {
        #[cfg(debug_assertions)]
        pretty_env_logger::formatted_timed_builder()
            .filter(Some("ranim"), LevelFilter::Trace)
            .init();
        #[cfg(not(debug_assertions))]
        pretty_env_logger::formatted_timed_builder()
            .filter(Some("ranim"), LevelFilter::Info)
            .init();
    }

    #[cfg(feature = "app")]
    run_scene_app(GettingStarted1Scene);
    #[cfg(not(feature = "app"))]
    render_scene(GettingStarted1Scene, &AppOptions::default());
}
