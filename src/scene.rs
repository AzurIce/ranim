//! Scene description types.
//!
//! These types describe *what* to render (scene metadata, output settings)
//! rather than *how* to animate (which lives in `ranim-core`).

use ranim_core::{RanimScene, SealedRanimScene};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

/// A scene descriptor bundling a constructor, config, and outputs.
#[doc(hidden)]
#[derive(Clone)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub struct Scene {
    /// Scene name
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen(skip))]
    pub name: String,
    /// Scene constructor
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen(skip))]
    pub constructor: fn(&mut RanimScene),
    /// Scene config
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen(skip))]
    pub config: SceneConfig,
    /// Scene outputs
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen(skip))]
    pub outputs: Vec<Output>,
}

/// Scene config
#[derive(Debug, Clone)]
pub struct SceneConfig {
    /// The clear color
    pub clear_color: String,
}

impl Default for SceneConfig {
    fn default() -> Self {
        Self {
            clear_color: "#333333ff".to_string(),
        }
    }
}

/// The output format of a scene
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    /// H.264 in MP4 container (default, opaque)
    #[default]
    Mp4,
    /// VP9 with alpha in WebM container (transparent)
    Webm,
    /// ProRes 4444 in MOV container (transparent)
    Mov,
    /// GIF (opaque, limited palette)
    Gif,
}

impl std::fmt::Display for OutputFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Mp4 => write!(f, "mp4"),
            Self::Webm => write!(f, "webm"),
            Self::Mov => write!(f, "mov"),
            Self::Gif => write!(f, "gif"),
        }
    }
}

/// The output of a scene
#[derive(Debug, Clone)]
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
    pub dir: String,
    /// The output video format.
    pub format: OutputFormat,
}

impl Default for Output {
    fn default() -> Self {
        Self {
            width: 1920,
            height: 1080,
            fps: 60,
            save_frames: false,
            dir: "./".to_string(),
            format: OutputFormat::default(),
        }
    }
}

// MARK: SceneConstructor
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
