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
//!     let mut app = App::new();
//!     app.add_scenes([hello::scene()]);
//!     run_cli(app.build());
//! }
//! ```
//!
//! Plugins organize larger projects and custom render configurations:
//!
//! ```
//! use ranim::{App, Plugin, Resource};
//!
//! /// Project resources must implement bevy's `Resource`.
//! #[derive(Resource)]
//! struct ProjectConfig(u32);
//!
//! struct ProjectPlugin;
//!
//! impl Plugin for ProjectPlugin {
//!     fn build(&self, app: &mut bevy_app::App) {
//!         app.insert_resource(ProjectConfig(42));
//!     }
//! }
//!
//! let mut app = App::new();
//! app.add_plugins(ProjectPlugin);
//! assert_eq!(app.world().resource::<ProjectConfig>().0, 42);
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

/// The WebAssembly host for project binaries
#[cfg(all(target_arch = "wasm32", feature = "render"))]
pub mod wasm;

pub use app::{
    AppExt, Output, OutputFormat, RanimApp, RanimAppExt, Scene, SceneConfig, SceneConstructor,
    Scenes,
};
pub use bevy_app::{App, Plugin};
pub use bevy_ecs::prelude::Resource;

pub use core::glam;
pub use ranim_core::{RanimScene, lagged, seq, stack};

/// The preludes
pub mod prelude {
    pub use crate::app::{AppExt, RanimAppExt};
    #[cfg(all(not(target_family = "wasm"), feature = "cmd-render"))]
    pub use crate::cmd::cli::run_cli;
    pub use crate::{App, Plugin};
    pub use ranim_core::prelude::*;
    pub use ranim_core::{lagged, seq, stack};
    #[cfg(feature = "items")]
    pub use ranim_items::camera::CameraFrame;
    pub use ranim_macros::{output, scene, wasm_demo_doc};
}
