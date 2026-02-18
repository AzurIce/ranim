//! Scene types for dylib / inventory registration and runtime use.
use crate::{Output, OutputFormat, Scene, SceneConfig};
use ranim_core::RanimScene;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

/// Static scene type for inventory registration
#[doc(hidden)]
pub struct StaticScene {
    /// Scene name
    pub name: &'static str,
    /// Scene constructor
    pub constructor: fn(&mut RanimScene),
    /// Scene config
    pub config: StaticSceneConfig,
    /// Scene outputs
    pub outputs: &'static [StaticOutput],
}

/// Static scene config for inventory registration
#[doc(hidden)]
pub struct StaticSceneConfig {
    /// The clear color
    pub clear_color: &'static str,
}

/// Static output for inventory registration
#[doc(hidden)]
pub struct StaticOutput {
    /// The width of the output texture in pixels.
    pub width: u32,
    /// The height of the output texture in pixels.
    pub height: u32,
    /// The frame rate of the output video.
    pub fps: u32,
    /// Whether to save the frames.
    pub save_frames: bool,
    /// The directory to save the output
    pub dir: &'static str,
    /// The output format
    pub format: OutputFormat,
}

impl StaticOutput {
    /// 1920x1080 60fps save_frames=false dir="./"
    pub const DEFAULT: Self = Self {
        width: 1920,
        height: 1080,
        fps: 60,
        save_frames: false,
        dir: "./",
        format: OutputFormat::Mp4,
    };
}

// MARK: inventory + FFI

pub use inventory;

inventory::collect!(StaticScene);

#[doc(hidden)]
#[unsafe(no_mangle)]
pub extern "C" fn get_scene(idx: usize) -> *const StaticScene {
    inventory::iter::<StaticScene>()
        .skip(idx)
        .take(1)
        .next()
        .unwrap()
}

#[doc(hidden)]
#[unsafe(no_mangle)]
pub extern "C" fn scene_cnt() -> usize {
    inventory::iter::<StaticScene>().count()
}

/// Return a scene with matched name
#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub fn find_scene(name: &str) -> Option<Scene> {
    inventory::iter::<StaticScene>()
        .find(|s| s.name == name)
        .map(Scene::from)
}

// MARK: From impls (Static -> Owned)

impl From<&StaticScene> for Scene {
    fn from(s: &StaticScene) -> Self {
        Self {
            name: s.name.to_string(),
            constructor: s.constructor,
            config: SceneConfig::from(&s.config),
            outputs: s.outputs.iter().map(Output::from).collect(),
        }
    }
}

impl From<&StaticSceneConfig> for SceneConfig {
    fn from(c: &StaticSceneConfig) -> Self {
        Self {
            clear_color: c.clear_color.to_string(),
        }
    }
}

impl From<&StaticOutput> for Output {
    fn from(o: &StaticOutput) -> Self {
        Self {
            width: o.width,
            height: o.height,
            fps: o.fps,
            save_frames: o.save_frames,
            dir: o.dir.to_string(),
            format: o.format,
        }
    }
}
