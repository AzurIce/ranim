mod scenes;

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    #[cfg(feature = "cmd-render")]
    {
        use ranim::prelude::*;
        let mut app = App::new();
        app.add_plugins(scenes::ScenesPlugin);
        run_cli(app.build());
    }
    #[cfg(not(feature = "cmd-render"))]
    compile_error!("native examples require the `cmd-render` feature (enabled by default)");
}

// The WASM module is driven by JavaScript through app().
#[cfg(target_arch = "wasm32")]
fn main() {}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn app() -> ranim::wasm::WasmApp {
    use ranim::prelude::*;

    let mut app = App::new();
    app.add_plugins(scenes::ScenesPlugin);
    ranim::wasm::WasmApp::new(app.build())
}
