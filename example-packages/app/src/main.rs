use app::hello_ranim_scene;
use ranim::app::preview_scene;

fn main() {
    #[cfg(not(target_arch = "wasm32"))]
    pretty_env_logger::formatted_timed_builder()
        .filter(Some("ranim"), log::LevelFilter::Info)
        .init();

    preview_scene(hello_ranim_scene);
}
