//! Ranim is an animation engine written in rust based on [`wgpu`], inspired by [3b1b/manim](https://github.com/3b1b/manim/) and [jkjkil4/JAnim](https://github.com/jkjkil4/JAnim).
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
#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![allow(rustdoc::private_intra_doc_links)]
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/AzurIce/ranim/refs/heads/main/assets/ranim.svg",
    html_favicon_url = "https://raw.githubusercontent.com/AzurIce/ranim/refs/heads/main/assets/ranim.svg"
)]
#![feature(downcast_unchecked)]

use timeline::{RanimScene, SealedRanimScene};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

/// Builtin anims
pub use ranim_anims as anims;
/// Builtin items
pub mod items;
/// Core
pub use ranim_core as core;
/// Color
pub use ranim_core::color;
/// The core structure to encode animations
pub mod timeline;

/// The preview app
#[cfg(all(feature = "app", feature = "render"))]
pub mod app;
/// Rendering stuff
#[cfg(feature = "render")]
pub mod render;
/// Utils
pub mod utils;

pub use glam;

// ANCHOR: SceneConstructor
/// A scene constructor
///
/// It can be a simple fn pointer of `fn(&mut RanimScene)`,
/// or any type implements `Fn(&mut RanimScene) + Send + Sync`.
pub trait SceneConstructor: Send + Sync {
    /// The construct logic
    fn construct(&self, r: &mut RanimScene);

    /// Use the constructor to build a [`SealedRanimScene`]
    fn build_scene(&self) -> SealedRanimScene {
        let mut scene = RanimScene::new();
        self.construct(&mut scene);
        scene.seal()
    }
}
// ANCHOR_END: SceneConstructor

impl<F: Fn(&mut RanimScene) + Send + Sync> SceneConstructor for F {
    fn construct(&self, r: &mut RanimScene) {
        self(r);
    }
}

// MARK: Dylib part
#[doc(hidden)]
#[derive(Clone)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub struct Scene {
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen(skip))]
    pub name: &'static str,
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen(skip))]
    pub constructor: fn(&mut RanimScene),
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen(skip))]
    pub config: SceneConfig,
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen(skip))]
    pub outputs: &'static [Output],
}

pub use inventory;

inventory::collect!(Scene);

#[doc(hidden)]
#[unsafe(no_mangle)]
pub extern "C" fn get_scene(idx: usize) -> *const Scene {
    inventory::iter::<Scene>().skip(idx).take(1).next().unwrap()
}

#[doc(hidden)]
#[unsafe(no_mangle)]
pub extern "C" fn scene_cnt() -> usize {
    inventory::iter::<Scene>().count()
}

#[cfg(target_arch = "wasm32")]
unsafe extern "C" {
    fn __wasm_call_ctors();
}

/// Return a scene with matched name
#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub fn find_scene(name: &str) -> Option<Scene> {
    inventory::iter::<Scene>().find(|s| s.name == name).cloned()
}

#[cfg(target_arch = "wasm32")]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen(start))]
fn wasm_start() {
    unsafe {
        __wasm_call_ctors();
    }
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));
    console_log::init().expect("Failed to initialize console_log");
}

/// Scene config
#[derive(Debug, Clone)]
pub struct SceneConfig {
    /// The height of the frame
    ///
    /// This will be the coordinate in the scene. The width is calculated by the aspect ratio from [`Output::width`] and [`Output::height`].
    pub frame_height: f64,
    /// The clear color
    pub clear_color: &'static str,
}

impl Default for SceneConfig {
    fn default() -> Self {
        Self {
            frame_height: 8.0,
            clear_color: "#333333ff",
        }
    }
}

/// The output of a scene
#[derive(Debug, Clone)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub struct Output {
    /// The width of the output texture in pixels.
    pub width: u32,
    /// The height of the output texture in pixels.
    pub height: u32,
    /// The frame rate of the output video.
    pub fps: u32,
    /// Whether to save the frames.
    pub save_frames: bool,
    /// The directory to save the output
    ///
    /// Related to the `output` folder, Or absolute.
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen(skip))]
    pub dir: &'static str,
}

impl Default for Output {
    fn default() -> Self {
        Self::DEFAULT
    }
}

impl Output {
    /// 1920x1080 60fps save_frames=false dir="./"
    pub const DEFAULT: Self = Self {
        width: 1920,
        height: 1080,
        fps: 60,
        save_frames: false,
        dir: "./",
    };
}

// MARK: Prelude
/// The preludes
pub mod prelude {
    #[cfg(feature = "app")]
    pub use crate::app::{preview_scene, run_app, run_scene_app};
    #[cfg(not(target_arch = "wasm32"))]
    pub use crate::render::app::{render_scene, render_scene_output};

    pub use ranim_core::prelude::*;
    pub use ranim_macros::{output, scene, wasm_demo_doc};

    pub use crate::items::ItemId;
    pub use crate::timeline::RanimScene;
}

#[cfg(feature = "profiling")]
// Since the timing information we get from WGPU may be several frames behind the CPU, we can't report these frames to
// the singleton returned by `puffin::GlobalProfiler::lock`. Instead, we need our own `puffin::GlobalProfiler` that we
// can be several frames behind puffin's main global profiler singleton.
pub(crate) static PUFFIN_GPU_PROFILER: std::sync::LazyLock<
    std::sync::Mutex<puffin::GlobalProfiler>,
> = std::sync::LazyLock::new(|| std::sync::Mutex::new(puffin::GlobalProfiler::default()));
