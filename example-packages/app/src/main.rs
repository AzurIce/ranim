use std::sync::Arc;

fn main() {
    #[cfg(target_arch = "wasm32")]
    {
        console_error_panic_hook::set_once();
        wasm_tracing::set_as_global_default();
    }

    #[cfg(not(target_arch = "wasm32"))]
    pretty_env_logger::formatted_timed_builder()
        .filter(Some("ranim"), log::LevelFilter::Info)
        .init();

    let mut app = ranim::App::new();
    app.add_plugins(app::plugin());
    let app = Arc::new(app);
    let scene = app.scene("hello_ranim").expect("hello_ranim is registered");
    ranim::cmd::preview_scene(scene);
}
