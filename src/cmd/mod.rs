/// Things for render to video
#[cfg(all(not(target_family = "wasm"), feature = "render"))]
pub mod render;
#[cfg(all(not(target_family = "wasm"), feature = "render"))]
pub use render::{
    RenderJob, render_scene, render_scene_job, render_scene_once, render_scene_output,
    render_scene_output_with_progress,
};

/// The preview application
#[cfg(feature = "preview")]
#[allow(missing_docs)]
pub mod preview;
#[cfg(feature = "preview")]
pub use preview::preview_scene;
