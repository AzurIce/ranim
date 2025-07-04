use log::LevelFilter;
use ranim::{
    animation::fading::FadingAnim, color::palettes::manim, items::vitem::geometry::Square,
    prelude::*, timeline::TimelineFunc,
};

#[scene]
struct GettingStarted0Scene;

impl SceneConstructor for GettingStarted0Scene {
    fn construct(self, r: &mut RanimScene, _r_cam: ItemId<CameraFrame>) {
        // A Square with size 2.0 and color blue
        let square = Square::new(2.0).with(|square| {
            square.set_color(manim::BLUE_C);
        });

        let timeline = r.init_timeline(square);
        timeline.play_with(|square| square.fade_in());
        timeline.forward(1.0);
        timeline.hide();
        timeline.forward(1.0);
        timeline.show();
        timeline.forward(1.0);
        timeline.play_with(|square| square.fade_out());
    }
}

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
    run_scene_app(GettingStarted0Scene);
    #[cfg(not(feature = "app"))]
    render_scene(GettingStarted0Scene, &AppOptions::default());
}
