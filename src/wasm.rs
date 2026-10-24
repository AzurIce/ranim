//! The WebAssembly host for a finalized Ranim project.
//!
//! A project compiled to WebAssembly exports a constructor returning
//! [`WasmApp`]. JavaScript creates a canvas preview, then drives its time:
//!
//! ```javascript
//! const project = app();
//! const preview = await project.createPreview(canvas, "hello");
//! preview.renderAt(1.5);
//! preview.dispose();
//! ```
//!
//! ```ignore
//! #[wasm_bindgen::prelude::wasm_bindgen]
//! pub fn app() -> ranim::wasm::WasmApp {
//!     use ranim::prelude::*;
//!     let mut app = App::new();
//!     app.add_plugins(scenes::ScenesPlugin);
//!     ranim::wasm::WasmApp::new(app.build())
//! }
//! ```

use wasm_bindgen::prelude::*;

use crate::{AppExt, RanimApp};

mod audio;
mod renderer;
mod timeline;
use renderer::CanvasRenderer;

/// A finalized Ranim project exported to JavaScript.
///
/// Creating a preview transfers the project into that session. Call the
/// project's exported `app()` constructor again for another independent preview.
#[wasm_bindgen]
pub struct WasmApp {
    app: Option<RanimApp>,
}

impl WasmApp {
    /// Wrap a finalized project.
    ///
    /// Not exported to JavaScript: a [`RanimApp`] cannot cross the boundary.
    /// Projects export their own constructor instead.
    pub fn new(app: RanimApp) -> Self {
        console_error_panic_hook::set_once();
        Self { app: Some(app) }
    }
}

#[wasm_bindgen]
impl WasmApp {
    /// The registered scene names, one per line.
    #[wasm_bindgen(js_name = sceneNames)]
    pub fn scene_names(&self) -> String {
        self.app
            .as_ref()
            .map(|app| {
                app.scenes()
                    .iter()
                    .map(|scene| scene.name.as_str())
                    .collect::<Vec<_>>()
                    .join("\n")
            })
            .unwrap_or_default()
    }

    /// Create a preview of a registered scene on an explicit canvas.
    ///
    /// Resolves after GPU and scene initialization. This does not start a clock
    /// or an event loop: the host calls [`WasmPreview::render_at`] to draw.
    /// A valid scene selection consumes the project, including if GPU setup fails.
    #[wasm_bindgen(js_name = createPreview)]
    pub async fn create_preview(
        &mut self,
        canvas: web_sys::HtmlCanvasElement,
        name: &str,
    ) -> Result<WasmPreview, JsValue> {
        let app = self
            .app
            .as_ref()
            .ok_or_else(|| JsValue::from_str("this WasmApp already drives a preview"))?;
        let Some(scene) = app.scene(name) else {
            return Err(JsValue::from_str(&format!(
                "scene `{name}` is not registered"
            )));
        };
        let app = self.app.take().expect("the project is available");
        let renderer = CanvasRenderer::new(app, scene, canvas).await?;
        Ok(WasmPreview {
            renderer: Some(renderer),
        })
    }
}

/// A canvas renderer with its own scene evaluator and GPU resources.
///
/// Playback controls and scheduling belong to the host. Neither egui nor
/// eframe participates in this session.
#[wasm_bindgen]
pub struct WasmPreview {
    renderer: Option<CanvasRenderer>,
}

#[wasm_bindgen]
impl WasmPreview {
    /// Scene duration in seconds.
    #[wasm_bindgen(getter)]
    pub fn duration(&self) -> Result<f64, JsValue> {
        Ok(self.renderer()?.duration())
    }

    /// Snapshot the animation tree used by the native timeline.
    ///
    /// Returns `{ duration, roots }`. Each node has `name`, `kind` (`anim`,
    /// `children`, or `audio`), `range` in parent-local seconds,
    /// `contentDuration`, `enabled`, `simStep`, and `children`.
    /// The snapshot contains ordinary JavaScript objects, independent of the
    /// renderer's lifetime. Fetch once when a scene is ready, not each frame.
    pub fn timeline(&self) -> Result<JsValue, JsValue> {
        let renderer = self.renderer()?;
        timeline::snapshot(renderer.duration(), renderer.animation_infos())
    }

    /// Copy the scene's baked mix into JavaScript-owned PCM channel arrays.
    ///
    /// Returns `null` for scenes without audio, otherwise `{ sampleRate,
    /// channels }`, where each channel is a `Float32Array`. Fetch once per
    /// session; the snapshot remains valid after disposal. Playback and its
    /// audio clock belong to the host.
    pub fn audio(&self) -> Result<JsValue, JsValue> {
        audio::snapshot(self.renderer()?.audio())
    }

    /// Evaluate and present the requested time, clamped to the scene duration.
    /// Returns false if the surface is temporarily unavailable; retry next frame.
    #[wasm_bindgen(js_name = renderAt)]
    pub fn render_at(&mut self, seconds: f64) -> Result<bool, JsValue> {
        self.renderer_mut()?.render_at(seconds)
    }

    /// Resize the drawing buffer, in physical pixels. Render again afterwards.
    pub fn resize(&mut self, width: u32, height: u32) -> Result<(), JsValue> {
        self.renderer_mut()?.resize(width, height)
    }

    /// Release the scene, surface, and GPU device. Safe to call more than once.
    pub fn dispose(&mut self) {
        self.renderer.take();
    }
}

impl WasmPreview {
    fn renderer(&self) -> Result<&CanvasRenderer, JsValue> {
        self.renderer
            .as_ref()
            .ok_or_else(|| JsValue::from_str("this preview has been disposed"))
    }

    fn renderer_mut(&mut self) -> Result<&mut CanvasRenderer, JsValue> {
        self.renderer
            .as_mut()
            .ok_or_else(|| JsValue::from_str("this preview has been disposed"))
    }
}
