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

pub use core::{glam, inventory};
pub use ranim_core::{Output, RanimScene, Scene, SceneConfig, SceneConstructor};

/// The preludes
pub mod prelude {
    pub use ranim_core::prelude::*;
    pub use ranim_macros::{output, scene, wasm_demo_doc};
}
