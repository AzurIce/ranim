use app::hello_ranim;

fn main() {
    #[cfg(not(target_arch = "wasm32"))]
    pretty_env_logger::formatted_timed_builder()
        .filter(Some("ranim"), log::LevelFilter::Info)
        .init();

    ranim_app::preview_scene!(hello_ranim);
}
