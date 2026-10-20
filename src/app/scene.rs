//! Scene description types.
//!
//! These types describe *what* to render (scene metadata, output settings)
//! rather than *how* to animate (which lives in `ranim-core`).

use std::sync::Arc;

use ranim_core::{RanimScene, SealedRanimScene};

/// A scene descriptor bundling a constructor, config, and outputs.
#[derive(Clone)]
pub struct Scene {
    /// Scene name
    pub name: String,
    constructor: Arc<dyn SceneConstructor>,
    /// Scene config
    pub config: SceneConfig,
    /// Scene outputs
    pub outputs: Vec<Output>,
}

impl Scene {
    /// Describe a scene with default rendering configuration and output.
    ///
    /// The constructor may capture plugin configuration. It runs only when a
    /// selected scene is built, and may run again for another session.
    pub fn new(name: impl Into<String>, constructor: impl SceneConstructor + 'static) -> Self {
        Self {
            name: name.into(),
            constructor: Arc::new(constructor),
            config: SceneConfig::default(),
            outputs: vec![Output::default()],
        }
    }
}

impl SceneConstructor for Scene {
    fn construct(&self, r: &mut RanimScene) {
        self.constructor.construct(r);
    }
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
    /// The name of the video, uses scene's name by default.
    ///
    /// e.g. the output of name `my_video` will be outputed as `my_video_<width>x<height>_<fps>.mp4`.
    pub name: Option<String>,
    /// The basename template for the rendered output.
    ///
    /// Supported placeholders:
    /// - `{name}`: the scene/output name
    /// - `{width}`: output width in pixels
    /// - `{height}`: output height in pixels
    /// - `{fps}`: output frame rate
    ///
    /// The file extension is appended automatically based on the output format.
    /// Defaults to `{name}_{width}x{height}_{fps}`.
    pub name_template: Option<String>,
    /// The directory to save the output.
    ///
    /// Can be relative (resolved from cwd) or absolute.
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
            name: None,
            name_template: None,
            dir: "./output".to_string(),
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

impl SceneConstructor for Arc<dyn SceneConstructor> {
    fn construct(&self, r: &mut RanimScene) {
        self.as_ref().construct(r)
    }
}
