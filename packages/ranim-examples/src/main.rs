use std::sync::Arc;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
thread_local! {
    static APP: std::cell::OnceCell<Arc<ranim::App>> = const { std::cell::OnceCell::new() };
}

fn main() {
    #[cfg(target_arch = "wasm32")]
    {
        console_error_panic_hook::set_once();
        wasm_tracing::set_as_global_default();
    }

    let mut app = ranim::App::new();
    app.add_plugins(ranim_examples::plugin());
    let app = Arc::new(app);

    #[cfg(target_arch = "wasm32")]
    APP.with(|state| {
        assert!(
            state.set(app).is_ok(),
            "preview host is already initialized"
        );
    });

    #[cfg(not(target_arch = "wasm32"))]
    {
        let scene = match std::env::args().nth(1) {
            Some(name) => app
                .scene(&name)
                .unwrap_or_else(|| panic!("scene '{name}' is not registered")),
            None => app.first_scene().expect("at least one scene is registered"),
        };
        ranim::cmd::preview_scene(scene);
    }
}

/// Start a preview of a scene registered in this host's application.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn preview_scene(name: &str) -> Result<(), JsValue> {
    APP.with(|state| {
        let app = state
            .get()
            .ok_or_else(|| JsValue::from_str("preview host is not initialized"))?;
        let scene = app
            .scene(name)
            .ok_or_else(|| JsValue::from_str(&format!("scene '{name}' is not registered")))?;
        ranim::cmd::preview_scene(scene);
        Ok(())
    })
}
