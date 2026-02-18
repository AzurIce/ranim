//! Ranim is an animation engine written in rust based on wgpu, inspired by [3b1b/manim](https://github.com/3b1b/manim/) and [jkjkil4/JAnim](https://github.com/jkjkil4/JAnim).
//!
//!
//! ## Coordinate System
//!
//! Ranim's coordinate system is right-handed coordinate:
//!
//! ```text
//!      +Y
//!      |
//!      |
//!      +----- +X
//!    /
//! +Z
//! ```
//!
#![warn(missing_docs)]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![allow(rustdoc::private_intra_doc_links)]
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/AzurIce/ranim/refs/heads/main/assets/ranim.svg",
    html_favicon_url = "https://raw.githubusercontent.com/AzurIce/ranim/refs/heads/main/assets/ranim.svg"
)]
#![feature(downcast_unchecked)]

#[cfg(feature = "anims")]
pub use ranim_anims as anims;
pub use ranim_core as core;
#[cfg(feature = "items")]
pub use ranim_items as items;
#[cfg(feature = "render")]
pub use ranim_render as render;

/// Commands like preview and render
pub mod cmd;
pub use core::color;

/// Utils
pub mod utils {
    pub use ranim_core::utils::*;
}

/// Scene types for dylib / inventory registration and runtime use.
mod link_magic;
pub use link_magic::*;

/// Scene description types (Scene, Output, OutputFormat, etc.)
mod scene;
pub use scene::*;

pub use core::glam;
pub use ranim_core::RanimScene;

/// Render a scene by name.
///
/// ```rust,ignore
/// render_scene!(fading);
/// ```
#[cfg(all(not(target_family = "wasm"), feature = "render"))]
#[macro_export]
macro_rules! render_scene {
    ($scene:ident) => {
        $crate::cmd::render_scene(&$scene::scene(), 2)
    };
}

/// The preludes
pub mod prelude {
    pub use ranim_core::prelude::*;
    pub use ranim_macros::{output, scene, wasm_demo_doc};
}
