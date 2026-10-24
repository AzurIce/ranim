/// The command line interface for project binaries
#[cfg(all(not(target_family = "wasm"), feature = "cmd-render"))]
pub mod cli;

/// Inspect scenes, animation trees, or evaluated frames without rendering
#[cfg(all(not(target_family = "wasm"), feature = "cmd-render"))]
pub mod inspect;

/// Things for render to video
#[cfg(all(not(target_family = "wasm"), feature = "cmd-render"))]
pub mod render;
#[cfg(all(not(target_family = "wasm"), feature = "cmd-render"))]
pub use render::{
    RenderJob, render_scene, render_scene_job, render_scene_once, render_scene_output,
    render_scene_output_with_progress,
};

/// The preview application
#[cfg(all(not(target_family = "wasm"), feature = "cmd-preview"))]
#[allow(missing_docs)]
pub mod preview;
#[cfg(all(not(target_family = "wasm"), feature = "cmd-preview"))]
pub use preview::preview_scene;
