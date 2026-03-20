/// Things for render to video
#[cfg(all(not(target_family = "wasm"), feature = "render"))]
pub mod render;
#[cfg(all(not(target_family = "wasm"), feature = "render"))]
pub use render::{render_scene, render_scene_output, render_scene_output_with_progress};

/// Render a scene by name.
///
/// ```rust,ignore
/// render_scene!(fading);
/// render_scene!(ranim_020::code_structure);
/// ```
#[cfg(all(not(target_family = "wasm"), feature = "render"))]
#[macro_export]
macro_rules! render_scene {
    ($($scene:tt)::+) => {
        $crate::cmd::render_scene(&$($scene)::+::scene(), 2)
    };
}

/// The preview application
#[cfg(feature = "preview")]
#[allow(missing_docs)]
pub mod preview;
#[cfg(feature = "preview")]
pub use preview::{preview_constructor_with_name, preview_scene, preview_scene_with_name};

/// Preview a scene by name.
///
/// ```rust,ignore
/// ranim::preview_scene!(fading);
/// ranim::preview_scene!(ranim_020::code_structure);
/// ```
#[macro_export]
macro_rules! preview_scene {
    ($($scene:tt)::+) => {
        $crate::cmd::preview_scene(&$($scene)::+::scene())
    };
}
