//! Ranim is an animation engine written in rust based on wgpu, inspired by [3b1b/manim](https://github.com/3b1b/manim/) and [jkjkil4/JAnim](https://github.com/jkjkil4/JAnim).
//!
//! A project binary registers its scenes and hands control to the command
//! line interface:
//!
//! ```ignore
//! use ranim::prelude::*;
//!
//! #[scene]
//! fn hello(r: &mut RanimScene) { /* ... */ }
//!
//! fn main() {
//!     // cargo run -- preview | output | render hello | inspect scenes
//!     App::new().add_scenes([hello::scene()]).run();
//! }
//! ```
//!
//! Plugins organize larger projects and custom render configurations:
//!
//! ```
//! use ranim::{App, Plugin};
//!
//! struct ProjectPlugin;
//!
//! impl Plugin for ProjectPlugin {
//!     fn build(&self, app: &mut App) {
//!         app.insert_resource(42_u32);
//!     }
//! }
//!
//! let mut app = App::new();
//! app.add_plugins(ProjectPlugin);
//! assert_eq!(*app.resource::<u32>(), 42);
//! ```
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

/// The project application: scenes, plugins, and resources
pub mod app;
/// Commands like preview and render
pub mod cmd;
pub use core::color;

/// Utils
pub mod utils {
    pub use ranim_core::utils::*;
}

pub use app::{
    App, Output, OutputFormat, Plugin, PluginError, Scene, SceneConfig, SceneConstructor,
    SceneHandle,
};

pub use core::glam;
pub use ranim_core::{RanimScene, lagged, seq, stack};

/// The preludes
pub mod prelude {
    pub use crate::app::{App, Plugin};
    #[cfg(all(not(target_family = "wasm"), feature = "render"))]
    pub use crate::cmd::AppRunExt;
    pub use ranim_core::prelude::*;
    pub use ranim_core::{lagged, seq, stack};
    pub use ranim_items::camera::CameraFrame;
    pub use ranim_macros::{output, scene, wasm_demo_doc};
}
