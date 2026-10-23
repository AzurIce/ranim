//! WebAssembly entry; project assembly is shared with the native binary.

#![cfg(target_arch = "wasm32")]

mod scenes;

#[wasm_bindgen::prelude::wasm_bindgen]
pub fn app() -> ranim::wasm::WasmApp {
    use ranim::prelude::*;

    let mut app = App::new();
    app.add_plugins(scenes::ScenesPlugin);
    ranim::wasm::WasmApp::new(app.build())
}
