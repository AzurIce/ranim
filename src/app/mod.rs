//! Project configuration and scene registration for Ranim.
//!
//! A project binary is a [`bevy_app::App`]: plugins register scenes and
//! resources, then [`RanimAppExt::build`] finalizes plugin construction into a
//! [`RanimApp`] that preview/render/inspect sessions consume.

use std::ops::{Deref, DerefMut};

use bevy_app::{App, PluginsState};
use bevy_ecs::prelude::*;

mod scene;
pub use scene::{Output, OutputFormat, Scene, SceneConfig, SceneConstructor};

/// Resolve the project's rendering configuration for every session host.
#[cfg(any(
    feature = "cmd-render",
    all(target_family = "wasm", feature = "render")
))]
pub(crate) fn render_setup_of(app: &App) -> ranim_render::RenderSetup {
    app.world()
        .get_resource::<ranim_render::RenderSetup>()
        .cloned()
        .unwrap_or_else(|| {
            ranim_render::RenderSetup::new(|_render_app| {
                #[cfg(feature = "items")]
                _render_app.add_plugins(ranim_items::RanimItemsPlugin);
            })
        })
}

/// The scenes registered in a project.
#[derive(Resource, Default)]
pub struct Scenes(Vec<Scene>);

/// A project with plugin construction finalized.
///
/// Produced by [`RanimAppExt::build`], which runs bevy's plugin `finish` and
/// `cleanup` exactly once. CLI and canvas/interactive preview hosts consume
/// this project and may install render sub-apps into it. Offline render workers
/// instead receive a selected [`Scene`] and its rendering configuration.
/// This wrapper is no longer a plugin-construction target.
pub struct RanimApp(App);

impl RanimApp {
    /// Consume the wrapper, returning the inner bevy app.
    pub fn into_inner(self) -> App {
        self.0
    }
}

impl Deref for RanimApp {
    type Target = App;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for RanimApp {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// Finalize a project into a [`RanimApp`].
pub trait RanimAppExt {
    /// Run plugin `finish` and `cleanup`, then hand the project to sessions.
    ///
    /// # Panics
    ///
    /// Panics if plugins are still being added (asynchronous plugin building
    /// has not completed).
    fn build(&mut self) -> RanimApp;
}

impl RanimAppExt for App {
    fn build(&mut self) -> RanimApp {
        assert_eq!(
            self.plugins_state(),
            PluginsState::Ready,
            "plugins must be ready before building a RanimApp"
        );
        self.finish();
        self.cleanup();
        RanimApp(std::mem::take(self))
    }
}

/// Scene registration and selection for a project [`App`].
pub trait AppExt {
    /// Register a scene without running its constructor.
    ///
    /// # Panics
    ///
    /// Panics if a scene with the same name is already registered.
    fn add_scene(&mut self, scene: Scene) -> &mut Self;

    /// Register scenes in iteration order.
    ///
    /// # Panics
    ///
    /// Panics if any scene name is already registered, including duplicate
    /// names within this collection.
    fn add_scenes(&mut self, scenes: impl IntoIterator<Item = Scene>) -> &mut Self {
        for scene in scenes {
            self.add_scene(scene);
        }
        self
    }

    /// Return the scene descriptions in registration order.
    fn scenes(&self) -> &[Scene];

    /// Select a scene by name, cloning its descriptor.
    fn scene(&self, name: &str) -> Option<Scene>;

    /// Select the first registered scene, if the project has any scenes.
    fn first_scene(&self) -> Option<Scene> {
        self.scenes().first().cloned()
    }
}

impl AppExt for bevy_app::App {
    fn add_scene(&mut self, scene: Scene) -> &mut Self {
        let mut scenes = self.world_mut().get_resource_or_init::<Scenes>();
        assert!(
            !scenes.0.iter().any(|existing| existing.name == scene.name),
            "scene `{}` is already registered",
            scene.name,
        );
        scenes.0.push(scene);
        self
    }

    fn scenes(&self) -> &[Scene] {
        self.world()
            .get_resource::<Scenes>()
            .map(|scenes| scenes.0.as_slice())
            .unwrap_or(&[])
    }

    fn scene(&self, name: &str) -> Option<Scene> {
        self.scenes()
            .iter()
            .find(|scene| scene.name == name)
            .cloned()
    }
}

#[cfg(test)]
mod tests;
