//! Ranim is an animation engine written in rust based on wgpu, inspired by [3b1b/manim](https://github.com/3b1b/manim/) and [jkjkil4/JAnim](https://github.com/jkjkil4/JAnim).
//!
//! Export a project's plugin factory for Rust hosts and native dynamic loading:
//!
//! ```
//! use ranim::{App, Plugin, export_plugin};
//!
//! struct ProjectPlugin;
//!
//! impl Plugin for ProjectPlugin {
//!     fn build(&self, app: &mut App) {
//!         app.insert_resource(42_u32);
//!     }
//! }
//!
//! #[export_plugin]
//! pub fn project_plugin() -> impl Plugin {
//!     ProjectPlugin
//! }
//!
//! let mut app = App::new();
//! app.add_plugins(project_plugin());
//! assert_eq!(*app.resource::<u32>(), 42);
//! ```
//!
//! The host constructs the App and chooses how to render or preview its scenes.
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
#![recursion_limit = "256"]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![allow(rustdoc::private_intra_doc_links)]
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/AzurIce/ranim/refs/heads/main/assets/ranim.svg",
    html_favicon_url = "https://raw.githubusercontent.com/AzurIce/ranim/refs/heads/main/assets/ranim.svg"
)]

extern crate self as ranim;

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

pub use ranim_macros::export_plugin;

#[cfg(all(feature = "dylib", not(target_family = "wasm")))]
pub use ranim_app::DynamicPlugin;
pub use ranim_app::{
    App, Output, OutputFormat, Plugin, PluginError, Scene, SceneConfig, SceneConstructor,
    SceneHandle,
};

pub use core::glam;
pub use ranim_core::{RanimScene, lagged, seq, stack};

/// Implementation dependencies used by exported macros.
#[doc(hidden)]
pub mod __private {
    pub use ranim_app::catch_plugin_panic;
}

/// The preludes
pub mod prelude {
    pub use ranim_app::{App, Plugin};
    pub use ranim_core::prelude::*;
    pub use ranim_core::{lagged, seq, stack};
    pub use ranim_macros::{export_plugin, output, scene, wasm_demo_doc};
}
