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

// MARK: wasm engine/plugin split (experimental)

// Two orthogonal build roles; both cfg gates include `target_family = "wasm"`
// so neither affects native builds. The roles are mutually exclusive: a host
// module owns the shared memory/table and serves scenes, a client module
// imports them. Whether the host also carries the preview app is decided
// independently via `preview`/`render`.
#[cfg(all(
    target_family = "wasm",
    feature = "scene-engine",
    feature = "scene-plugin"
))]
compile_error!(
    "features `scene-engine` (host role) and `scene-plugin` (client role) are mutually exclusive"
);

/// Experimental wasm engine/plugin split exports (engine/host side).
#[cfg(all(target_family = "wasm", feature = "scene-engine"))]
mod wasm_engine;

/// Client-side runtime for wasm scene plugins (`scene-plugin` feature).
#[cfg(all(target_family = "wasm", feature = "scene-plugin"))]
mod scene_plugin;

/// Scene description types (Scene, Output, OutputFormat, etc.)
mod scene;
pub use scene::*;

pub use core::glam;
pub use ranim_core::{RanimScene, lagged, seq, stack};

/// The preludes
pub mod prelude {
    pub use ranim_core::prelude::*;
    pub use ranim_core::{lagged, seq, stack};
    pub use ranim_macros::{output, scene, wasm_demo_doc};
}
