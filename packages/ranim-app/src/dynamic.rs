//! Native loading for the Rust plugin entry point.

use std::{
    any::TypeId,
    fs::{self, OpenOptions},
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};

use libloading::Library;

use crate::{App, Plugin, PluginError};

/// A plugin loaded from a native dynamic library.
///
/// Install it with [`App::try_add_plugins`] just like a statically linked plugin.
/// The App and its scenes retain the library after this wrapper is consumed,
/// allowing scene constructors, resources, and evaluators to use plugin code.
/// Each load uses its own temporary copy, including repeated loads of one path.
pub struct DynamicPlugin {
    // Drop the trait object while its vtable and destructor are still loaded.
    inner: Box<dyn Plugin>,
    owner: Arc<LoadedLibrary>,
}

struct LoadedLibrary {
    inner: Option<Library>,
    temp_path: PathBuf,
}

impl DynamicPlugin {
    /// Load the `ranim_plugin` Rust ABI entry point from a dynamic library.
    ///
    /// The entry point constructs a plugin without constructing an App. Errors
    /// from the constructor are returned before any plugin is installed.
    ///
    /// # Safety
    ///
    /// The library and its dependencies must be trusted, built with matching
    /// Rust toolchains and Ranim versions, and use compatible build settings.
    /// The `ranim_plugin` symbol must have the Rust ABI signature
    /// `fn() -> Result<Box<dyn Plugin>, String>`. Its constructor must catch
    /// panics within the library and return an owned error string, as Ranim's
    /// `#[export_plugin]` attribute does. Implementations of [`Plugin::try_build`]
    /// must likewise prevent unwinding across the library boundary.
    /// Plugin destructors must not unwind across that boundary either. When
    /// taking plugin-owned callbacks or resource values out of the managed
    /// App/scene/evaluator lifetimes, retain their App or a scene handle until
    /// those values have also been destroyed.
    pub unsafe fn load(path: impl AsRef<Path>) -> Result<Self, PluginError> {
        let path = path.as_ref();
        let load_error = |message| PluginError::Load {
            path: path.to_owned(),
            message,
        };
        let mut loaded =
            LoadedLibrary::copy(path).map_err(|error| load_error(error.to_string()))?;
        // SAFETY: The caller guarantees this library and its initialization code
        // meet the Rust ABI and trust requirements documented above.
        loaded.inner = Some(
            unsafe { Library::new(&loaded.temp_path) }
                .map_err(|error| load_error(error.to_string()))?,
        );
        let library = loaded.inner.as_ref().expect("library was just opened");
        // SAFETY: The caller guarantees the exported symbol's signature and
        // constructor panic handling match the plugin protocol.
        let constructor =
            unsafe { library.get::<fn() -> Result<Box<dyn Plugin>, String>>(b"ranim_plugin") }
                .map_err(|error| {
                    load_error(format!(
                        "missing `ranim_plugin` entry point; mark a plugin factory \
                 with `#[ranim::export_plugin]`: {error}"
                    ))
                })?;
        let inner = constructor().map_err(|message| PluginError::Constructor {
            path: path.to_owned(),
            message,
        })?;
        Ok(Self {
            inner,
            owner: Arc::new(loaded),
        })
    }
}

impl Plugin for DynamicPlugin {
    fn build(&self, app: &mut App) {
        self.try_build(app)
            .unwrap_or_else(|message| panic!("{message}"));
    }

    fn name(&self) -> &str {
        self.inner.name()
    }

    fn plugin_type_id(&self) -> TypeId {
        self.inner.plugin_type_id()
    }

    fn try_build(&self, app: &mut App) -> Result<(), String> {
        // Retain before running plugin code: even a failed build may leave
        // resources, scene callbacks, or dependent plugins in the App.
        app.keep_alive(Arc::clone(&self.owner));
        self.inner.try_build(app)
    }
}

impl LoadedLibrary {
    fn copy(path: &Path) -> std::io::Result<Self> {
        static NEXT_COPY: AtomicU64 = AtomicU64::new(0);
        let name = path.file_name().unwrap_or_default().to_string_lossy();
        let temp_dir = std::env::temp_dir();
        loop {
            let temp_path = temp_dir.join(format!(
                "ranim-plugin-{}-{}-{name}",
                std::process::id(),
                NEXT_COPY.fetch_add(1, Ordering::Relaxed)
            ));
            match OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&temp_path)
            {
                Ok(file) => drop(file),
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(error) => return Err(error),
            }
            let loaded = Self {
                inner: None,
                temp_path,
            };
            fs::copy(path, &loaded.temp_path)?;
            return Ok(loaded);
        }
    }
}

impl Drop for LoadedLibrary {
    fn drop(&mut self) {
        // Windows requires unloading the library before its file can be removed.
        drop(self.inner.take());
        let _ = fs::remove_file(&self.temp_path);
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;
    use crate::{Scene, SceneConstructor};

    struct RecordedPlugin(Arc<Mutex<Vec<&'static str>>>);

    impl Plugin for RecordedPlugin {
        fn build(&self, app: &mut App) {
            let record = Arc::clone(&self.0);
            app.add_scene(Scene::new(
                "dynamic",
                move |_: &mut ranim_core::RanimScene| {
                    record.lock().unwrap().push("construct");
                },
            ));
        }
    }

    impl Drop for RecordedPlugin {
        fn drop(&mut self) {
            self.0.lock().unwrap().push("plugin");
        }
    }

    #[test]
    fn wrapper_preserves_concrete_identity_and_retains_library_for_built_scene() {
        let record = Arc::new(Mutex::new(Vec::new()));
        let source = std::env::current_exe().unwrap();
        let owner = Arc::new(LoadedLibrary::copy(&source).unwrap());
        let temp_path = owner.temp_path.clone();
        let weak_owner = Arc::downgrade(&owner);
        let plugin = DynamicPlugin {
            inner: Box::new(RecordedPlugin(Arc::clone(&record))),
            owner,
        };
        let mut app = App::new();
        app.add_plugins(plugin);
        assert!(app.is_plugin_added::<RecordedPlugin>());
        assert!(!app.is_plugin_added::<DynamicPlugin>());
        assert_eq!(*record.lock().unwrap(), ["plugin"]);
        let scene = app.scenes()[0].clone();
        let built = scene.build_scene();
        drop(app);
        drop(scene);
        assert!(temp_path.exists());
        assert!(weak_owner.upgrade().is_some());
        drop(built);
        assert!(!temp_path.exists());
        assert!(weak_owner.upgrade().is_none());
    }

    #[test]
    fn failed_library_load_cleans_up_its_temporary_copy() {
        let source = LoadedLibrary::copy(&std::env::current_exe().unwrap()).unwrap();
        let source_name = source.temp_path.file_name().unwrap().to_owned();
        let before = fs::read_dir(std::env::temp_dir())
            .unwrap()
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .collect::<std::collections::HashSet<_>>();
        // SAFETY: Loading this test executable is expected to be rejected by the
        // OS loader; no plugin constructor or foreign Rust code will execute.
        let result = unsafe { DynamicPlugin::load(&source.temp_path) };
        assert!(matches!(result, Err(PluginError::Load { .. })));
        let leaked = fs::read_dir(std::env::temp_dir())
            .unwrap()
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| !before.contains(path))
            .any(|path| {
                path.file_name()
                    .unwrap()
                    .to_string_lossy()
                    .ends_with(source_name.to_string_lossy().as_ref())
            });
        assert!(!leaked);
    }
}
