//! Project configuration and plugin composition for Ranim.
//!
//! An [`App`] collects scene descriptions and resources without constructing
//! scenes or initializing a renderer. [`Plugin`] implementations configure the
//! project, and [`SceneHandle`] keeps that configuration alive for a session.

#![warn(missing_docs)]

use std::{
    any::{Any, TypeId, type_name},
    collections::{HashMap, HashSet},
    ops::Deref,
    panic::{AssertUnwindSafe, catch_unwind},
    sync::Arc,
};

use ranim_core::{RanimScene, SealedRanimScene, lifetime::KeepAlive};

mod scene;
pub use scene::{Output, OutputFormat, Scene, SceneConfig, SceneConstructor};

#[cfg(all(feature = "dylib", not(target_family = "wasm")))]
mod dynamic;
#[cfg(all(feature = "dylib", not(target_family = "wasm")))]
pub use dynamic::DynamicPlugin;

/// A project containing scenes and the configuration installed by its plugins.
///
/// Build this value before sharing it through an [`Arc`] with preview or output
/// sessions. Adding scenes only stores their descriptions; constructors run
/// when a selected [`SceneHandle`] is built.
#[derive(Default)]
pub struct App {
    scenes: Vec<Scene>,
    plugins: HashSet<TypeId>,
    resources: HashMap<TypeId, Box<dyn Any + Send + Sync>>,
    // Rust drops fields in declaration order. Keep library guards last so that
    // resource destructors and other project-owned code remain loaded.
    keep_alive: KeepAlive,
}

impl App {
    /// Create an empty project.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a scene without running its constructor.
    ///
    /// # Panics
    ///
    /// Panics if a scene with the same name is already registered.
    pub fn add_scene(&mut self, mut scene: Scene) -> &mut Self {
        assert!(
            !self
                .scenes
                .iter()
                .any(|existing| existing.name == scene.name),
            "scene `{}` is already registered",
            scene.name,
        );
        for owner in self.keep_alive.iter() {
            scene.retain_owner(Arc::clone(owner));
        }
        self.scenes.push(scene);
        self
    }

    /// Register scenes in iteration order.
    ///
    /// # Panics
    ///
    /// Panics if any scene name is already registered, including duplicate
    /// names within this collection.
    pub fn add_scenes(&mut self, scenes: impl IntoIterator<Item = Scene>) -> &mut Self {
        for scene in scenes {
            self.add_scene(scene);
        }
        self
    }

    /// Return the scene descriptions in registration order.
    pub fn scenes(&self) -> &[Scene] {
        &self.scenes
    }

    /// Select a scene while keeping the project and its library guards alive.
    pub fn scene(self: &Arc<Self>, name: &str) -> Option<SceneHandle> {
        self.scenes
            .iter()
            .position(|scene| scene.name == name)
            .map(|index| SceneHandle {
                app: Arc::clone(self),
                index,
            })
    }

    /// Select the first registered scene, if the project has any scenes.
    pub fn first_scene(self: &Arc<Self>) -> Option<SceneHandle> {
        (!self.scenes.is_empty()).then(|| SceneHandle {
            app: Arc::clone(self),
            index: 0,
        })
    }

    /// Install a plugin or a tuple of plugins, executing each build immediately.
    ///
    /// Tuples may be nested. Plugins can add further plugins from their build
    /// method. Each concrete plugin type may be installed only once, regardless
    /// of its configuration, boxing, or dynamic library loading.
    ///
    /// # Panics
    ///
    /// Panics if a plugin type is already installed, is currently building, or
    /// fails to build. Use [`Self::try_add_plugins`] to handle these failures.
    pub fn add_plugins<M>(&mut self, plugins: impl Plugins<M>) -> &mut Self {
        self.try_add_plugins(plugins)
            .unwrap_or_else(|error| panic!("{error}"))
    }

    /// Install plugins, returning duplicate registration and build failures.
    ///
    /// A failure does not roll back scenes, resources, or dependencies already
    /// installed by this collection. A failed plugin can be attempted again.
    pub fn try_add_plugins<M>(
        &mut self,
        plugins: impl Plugins<M>,
    ) -> Result<&mut Self, PluginError> {
        plugins.add_to(self)?;
        Ok(self)
    }

    fn add_plugin<P: Plugin>(&mut self, plugin: P) -> Result<(), PluginError> {
        let identity = plugin.plugin_type_id();
        let name = plugin.name().to_owned();
        if !self.plugins.insert(identity) {
            return Err(PluginError::AlreadyAdded { plugin: name });
        }
        if let Err(message) = plugin.try_build(self) {
            self.plugins.remove(&identity);
            return Err(PluginError::Build {
                plugin: name,
                message,
            });
        }
        Ok(())
    }

    /// Return whether a plugin type has been added, including one building now.
    pub fn is_plugin_added<P: Plugin>(&self) -> bool {
        self.plugins.contains(&TypeId::of::<P>())
    }

    /// Insert a resource, replacing any previous resource of the same type.
    pub fn insert_resource<T: Send + Sync + 'static>(&mut self, resource: T) -> &mut Self {
        self.resources.insert(TypeId::of::<T>(), Box::new(resource));
        self
    }

    /// Get a resource, if one of this type has been inserted.
    pub fn get_resource<T: Send + Sync + 'static>(&self) -> Option<&T> {
        self.resources.get(&TypeId::of::<T>())?.downcast_ref()
    }

    /// Get a resource mutably, if one of this type has been inserted.
    pub fn get_resource_mut<T: Send + Sync + 'static>(&mut self) -> Option<&mut T> {
        self.resources.get_mut(&TypeId::of::<T>())?.downcast_mut()
    }

    /// Get a resource that must already exist.
    ///
    /// # Panics
    ///
    /// Panics if no resource of this type has been inserted.
    pub fn resource<T: Send + Sync + 'static>(&self) -> &T {
        self.get_resource()
            .unwrap_or_else(|| panic!("resource `{}` is not registered", type_name::<T>()))
    }

    /// Get a resource mutably that must already exist.
    ///
    /// # Panics
    ///
    /// Panics if no resource of this type has been inserted.
    pub fn resource_mut<T: Send + Sync + 'static>(&mut self) -> &mut T {
        self.get_resource_mut()
            .unwrap_or_else(|| panic!("resource `{}` is not registered", type_name::<T>()))
    }

    /// Remove a resource and return its value, if it exists.
    pub fn remove_resource<T: Send + Sync + 'static>(&mut self) -> Option<T> {
        self.resources.remove(&TypeId::of::<T>()).map(|resource| {
            *resource
                .downcast::<T>()
                .expect("resource type matches its key")
        })
    }

    /// Retain an owner until all project resources and scene handles are gone.
    ///
    /// Dynamic plugin loaders use this to retain their library while scene
    /// constructors, plugin resources, or resource destructors may call into it.
    /// Owners are dropped after all other fields of this App, in reverse
    /// registration order so nested plugin libraries remain loaded.
    pub fn keep_alive<T: Any + Send + Sync>(&mut self, owner: Arc<T>) -> &mut Self {
        let owner: Arc<dyn Any + Send + Sync> = owner;
        for scene in &mut self.scenes {
            scene.retain_owner(Arc::clone(&owner));
        }
        self.keep_alive.push(owner);
        self
    }
}

/// An error encountered while loading or installing a plugin.
#[derive(Debug)]
pub enum PluginError {
    /// This concrete plugin type is installed or is currently building.
    AlreadyAdded {
        /// The plugin's diagnostic name.
        plugin: String,
    },
    /// The plugin failed to configure the project.
    Build {
        /// The plugin's diagnostic name.
        plugin: String,
        /// An owned description of the failure.
        message: String,
    },
    /// A dynamic library could not be copied, opened, or queried for its entry.
    Load {
        /// The requested library path.
        path: std::path::PathBuf,
        /// An owned description of the failure.
        message: String,
    },
    /// The exported plugin constructor failed.
    Constructor {
        /// The requested library path.
        path: std::path::PathBuf,
        /// An owned description of the failure.
        message: String,
    },
}

impl std::fmt::Display for PluginError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AlreadyAdded { plugin } => write!(f, "plugin `{plugin}` is already added"),
            Self::Build { plugin, message } => {
                write!(f, "plugin `{plugin}` failed to build: {message}")
            }
            Self::Load { path, message } => {
                write!(
                    f,
                    "failed to load plugin library `{}`: {message}",
                    path.display()
                )
            }
            Self::Constructor { path, message } => write!(
                f,
                "plugin constructor in `{}` failed: {message}",
                path.display()
            ),
        }
    }
}

impl std::error::Error for PluginError {}

/// A configurable unit of project setup.
///
/// The plugin instance is consumed after installation. Store reusable
/// configuration and factories in [`App`] resources rather than on the plugin
/// itself. The same plugin can be registered directly or exported from a
/// dynamic library.
pub trait Plugin: Send + Sync + 'static {
    /// Register scenes, resources, or other plugins with the project.
    fn build(&self, app: &mut App);

    /// Return a diagnostic name for this plugin.
    fn name(&self) -> &str {
        type_name::<Self>()
    }

    /// Return the concrete plugin's identity, preserved by loading wrappers.
    fn plugin_type_id(&self) -> TypeId {
        TypeId::of::<Self>()
    }

    /// Configure the project, converting a panic into an owned error message.
    ///
    /// The default implementation catches panics inside the plugin's runtime,
    /// so dynamic plugins do not unwind through another Rust runtime. Overrides
    /// must preserve that boundary and return owned error strings.
    fn try_build(&self, app: &mut App) -> Result<(), String> {
        catch_plugin_panic(|| self.build(app))
    }
}

impl<P: Plugin + ?Sized> Plugin for Box<P> {
    fn build(&self, app: &mut App) {
        (**self).build(app);
    }

    fn name(&self) -> &str {
        (**self).name()
    }

    fn plugin_type_id(&self) -> TypeId {
        (**self).plugin_type_id()
    }

    fn try_build(&self, app: &mut App) -> Result<(), String> {
        (**self).try_build(app)
    }
}

/// Catch a plugin panic and destroy its payload in the originating runtime.
#[doc(hidden)]
pub fn catch_plugin_panic<T>(build: impl FnOnce() -> T) -> Result<T, String> {
    catch_unwind(AssertUnwindSafe(build)).map_err(|payload| {
        let message = if let Some(message) = payload.downcast_ref::<String>() {
            message.clone()
        } else if let Some(message) = payload.downcast_ref::<&str>() {
            (*message).to_owned()
        } else {
            "plugin panicked with a non-string payload".to_owned()
        };
        drop(payload);
        message
    })
}

/// A plugin or tuple accepted by [`App::add_plugins`].
///
/// Implement [`Plugin`] to define project setup. This trait is implemented
/// automatically for plugins and tuples of up to sixteen plugin collections.
pub trait Plugins<Marker>: sealed::Sealed<Marker> {
    /// Install every plugin in this collection.
    #[doc(hidden)]
    fn add_to(self, app: &mut App) -> Result<(), PluginError>;
}

mod sealed {
    pub trait Sealed<Marker> {}
    pub struct SinglePlugin;
}

impl<P: Plugin> sealed::Sealed<sealed::SinglePlugin> for P {}

impl<P: Plugin> Plugins<sealed::SinglePlugin> for P {
    fn add_to(self, app: &mut App) -> Result<(), PluginError> {
        app.add_plugin(self)
    }
}

impl sealed::Sealed<()> for () {}

impl Plugins<()> for () {
    fn add_to(self, _app: &mut App) -> Result<(), PluginError> {
        Ok(())
    }
}

macro_rules! impl_plugin_tuples {
    ($($plugin:ident : $marker:ident),+) => {
        impl<$($plugin, $marker),+> sealed::Sealed<($($marker,)+)> for ($($plugin,)+)
        where
            $($plugin: Plugins<$marker>,)+
        {}

        impl<$($plugin, $marker),+> Plugins<($($marker,)+)> for ($($plugin,)+)
        where
            $($plugin: Plugins<$marker>,)+
        {
            #[allow(non_snake_case)]
            fn add_to(self, app: &mut App) -> Result<(), PluginError> {
                let ($($plugin,)+) = self;
                $($plugin.add_to(app)?;)+
                Ok(())
            }
        }
    };
}

impl_plugin_tuples!(P0: M0);
impl_plugin_tuples!(P0: M0, P1: M1);
impl_plugin_tuples!(P0: M0, P1: M1, P2: M2);
impl_plugin_tuples!(P0: M0, P1: M1, P2: M2, P3: M3);
impl_plugin_tuples!(P0: M0, P1: M1, P2: M2, P3: M3, P4: M4);
impl_plugin_tuples!(P0: M0, P1: M1, P2: M2, P3: M3, P4: M4, P5: M5);
impl_plugin_tuples!(P0: M0, P1: M1, P2: M2, P3: M3, P4: M4, P5: M5, P6: M6);
impl_plugin_tuples!(P0: M0, P1: M1, P2: M2, P3: M3, P4: M4, P5: M5, P6: M6, P7: M7);
impl_plugin_tuples!(P0: M0, P1: M1, P2: M2, P3: M3, P4: M4, P5: M5, P6: M6, P7: M7, P8: M8);
impl_plugin_tuples!(P0: M0, P1: M1, P2: M2, P3: M3, P4: M4, P5: M5, P6: M6, P7: M7, P8: M8, P9: M9);
impl_plugin_tuples!(P0: M0, P1: M1, P2: M2, P3: M3, P4: M4, P5: M5, P6: M6, P7: M7, P8: M8, P9: M9, P10: M10);
impl_plugin_tuples!(P0: M0, P1: M1, P2: M2, P3: M3, P4: M4, P5: M5, P6: M6, P7: M7, P8: M8, P9: M9, P10: M10, P11: M11);
impl_plugin_tuples!(P0: M0, P1: M1, P2: M2, P3: M3, P4: M4, P5: M5, P6: M6, P7: M7, P8: M8, P9: M9, P10: M10, P11: M11, P12: M12);
impl_plugin_tuples!(P0: M0, P1: M1, P2: M2, P3: M3, P4: M4, P5: M5, P6: M6, P7: M7, P8: M8, P9: M9, P10: M10, P11: M11, P12: M12, P13: M13);
impl_plugin_tuples!(P0: M0, P1: M1, P2: M2, P3: M3, P4: M4, P5: M5, P6: M6, P7: M7, P8: M8, P9: M9, P10: M10, P11: M11, P12: M12, P13: M13, P14: M14);
impl_plugin_tuples!(P0: M0, P1: M1, P2: M2, P3: M3, P4: M4, P5: M5, P6: M6, P7: M7, P8: M8, P9: M9, P10: M10, P11: M11, P12: M12, P13: M13, P14: M14, P15: M15);

/// A selected scene retaining its project and any loaded library owners.
#[derive(Clone)]
pub struct SceneHandle {
    app: Arc<App>,
    index: usize,
}

impl SceneHandle {
    /// Return the selected scene description.
    pub fn scene(&self) -> &Scene {
        &self.app.scenes[self.index]
    }

    /// Return the project configuration used by this scene.
    pub fn app(&self) -> &App {
        &self.app
    }

    /// Construct and seal a fresh scene for a preview or output session.
    pub fn build_scene(&self) -> SealedRanimScene {
        SceneConstructor::build_scene(self)
    }
}

impl Deref for SceneHandle {
    type Target = Scene;

    fn deref(&self) -> &Self::Target {
        self.scene()
    }
}

impl SceneConstructor for SceneHandle {
    fn construct(&self, r: &mut RanimScene) {
        r.keep_alive(self.app.clone());
        self.scene().construct(r);
    }
}

#[cfg(test)]
mod tests;
