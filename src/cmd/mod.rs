/// The command line interface for project binaries
#[cfg(all(not(target_family = "wasm"), feature = "render"))]
pub mod cli;
#[cfg(all(not(target_family = "wasm"), feature = "render"))]
pub use cli::AppRunExt;

/// The render setup used when the project does not register one: the built-in
/// camera, vector-item, and mesh-item plugins.
#[cfg(feature = "render")]
pub(crate) fn default_render_setup() -> ranim_render::RenderSetup {
    ranim_render::RenderSetup::new(|app| {
        app.add_plugins(ranim_items::RanimItemsPlugin);
    })
}

/// Inspect scenes, animation trees, or evaluated frames without rendering
#[cfg(all(not(target_family = "wasm"), feature = "render"))]
pub mod inspect;

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
