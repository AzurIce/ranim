/// Things for render to video
#[cfg(all(not(target_family = "wasm"), feature = "render"))]
pub mod render;
#[cfg(all(not(target_family = "wasm"), feature = "render"))]
pub use render::{render_scene, render_scene_output};

/// Things for preview app
#[cfg(feature = "preview")]
pub mod preview {
    pub use ranim_app::*;
}
#[cfg(feature = "preview")]
pub use preview::preview_scene;
