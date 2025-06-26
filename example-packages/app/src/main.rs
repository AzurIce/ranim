use app::HelloRanimScene;
use ranim::app::run_scene_app;

fn main() {
    #[cfg(not(target_arch = "wasm32"))]
    pretty_env_logger::formatted_timed_builder()
        .filter(Some("ranim"), log::LevelFilter::Info)
        .init();

    run_scene_app(HelloRanimScene);
}
