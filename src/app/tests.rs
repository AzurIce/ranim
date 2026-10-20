use std::sync::{
    Mutex,
    atomic::{AtomicUsize, Ordering},
};

use super::*;

fn scene(name: &str, constructor: fn(&mut RanimScene)) -> Scene {
    Scene::new(name, constructor)
}

struct ConfiguredPlugin(usize);

impl Plugin for ConfiguredPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(self.0);
    }
}

struct ScenePlugin;

impl Plugin for ScenePlugin {
    fn build(&self, app: &mut App) {
        assert_eq!(*app.resource::<usize>(), 42);
        app.add_scene(scene("intro", |_| {}));
    }
}

#[test]
fn configured_plugins_and_nested_tuples_build_in_order() {
    let mut app = App::new();
    app.add_plugins((ConfiguredPlugin(42), (ScenePlugin, ())));

    assert!(app.is_plugin_added::<ConfiguredPlugin>());
    assert!(app.is_plugin_added::<ScenePlugin>());
    assert_eq!(*app.resource::<usize>(), 42);
    assert_eq!(app.scenes()[0].name, "intro");
}

#[test]
#[should_panic(expected = "is already added")]
fn duplicate_plugin_types_are_rejected_across_tuples() {
    let mut app = App::new();
    app.add_plugins((ConfiguredPlugin(1),));
    app.add_plugins(ConfiguredPlugin(2));
}

#[test]
#[should_panic(expected = "is already added")]
fn a_plugin_cannot_add_itself_recursively() {
    struct Recursive;
    impl Plugin for Recursive {
        fn build(&self, app: &mut App) {
            app.add_plugins(Recursive);
        }
    }
    App::new().add_plugins(Recursive);
}

#[test]
#[should_panic(expected = "scene `intro` is already registered")]
fn duplicate_scene_names_are_rejected() {
    App::new().add_scenes([scene("intro", |_| {}), scene("intro", |_| {})]);
}

#[test]
fn resources_can_be_replaced_mutated_and_removed() {
    let mut app = App::new();
    assert!(app.get_resource::<String>().is_none());
    assert!(app.get_resource_mut::<String>().is_none());
    app.insert_resource(String::from("old"));
    app.insert_resource(String::from("new"));
    app.resource_mut::<String>().push_str(" value");
    assert_eq!(app.resource::<String>(), "new value");
    assert_eq!(app.remove_resource::<String>(), Some("new value".into()));
    assert!(app.remove_resource::<String>().is_none());
}

#[test]
fn scene_selection_is_lazy_and_preserves_registration_order() {
    static BUILDS: AtomicUsize = AtomicUsize::new(0);
    fn construct(_: &mut RanimScene) {
        BUILDS.fetch_add(1, Ordering::Relaxed);
    }

    let mut app = App::new();
    app.add_scenes([scene("intro", construct), scene("outro", |_| {})]);
    let app = Arc::new(app);
    let selected = app.scene("intro").unwrap();
    assert_eq!(selected.scene().name, "intro");
    assert_eq!(app.first_scene().unwrap().name, "intro");
    assert!(app.scene("missing").is_none());
    assert_eq!(BUILDS.load(Ordering::Relaxed), 0);

    assert_eq!(selected.build_scene().total_secs(), 0.0);
    assert_eq!(BUILDS.load(Ordering::Relaxed), 1);
    let cloned = selected.clone();
    assert_eq!(cloned.build_scene().total_secs(), 0.0);
    assert_eq!(BUILDS.load(Ordering::Relaxed), 2);
    assert_eq!(cloned.app().scenes().len(), 2);
    assert!(Arc::new(App::default()).first_scene().is_none());
}

#[test]
fn a_plugin_can_capture_its_configuration_in_a_scene_constructor() {
    struct CapturedPlugin(Arc<AtomicUsize>);

    impl Plugin for CapturedPlugin {
        fn build(&self, app: &mut App) {
            let builds = Arc::clone(&self.0);
            app.add_scene(Scene::new("captured", move |_: &mut RanimScene| {
                builds.fetch_add(1, Ordering::Relaxed);
            }));
        }
    }

    let builds = Arc::new(AtomicUsize::new(0));
    let mut app = App::new();
    app.add_plugins(CapturedPlugin(Arc::clone(&builds)));
    let handle = Arc::new(app).first_scene().unwrap();
    assert_eq!(builds.load(Ordering::Relaxed), 0);
    handle.build_scene();
    handle.build_scene();
    assert_eq!(builds.load(Ordering::Relaxed), 2);
}

#[test]
fn boxed_plugins_preserve_their_concrete_identity() {
    let mut app = App::new();
    let configured: Box<dyn Plugin> = Box::new(ConfiguredPlugin(42));
    let scenes: Box<dyn Plugin> = Box::new(ScenePlugin);
    app.add_plugins((configured, scenes));
    assert!(app.is_plugin_added::<ConfiguredPlugin>());
    assert!(app.is_plugin_added::<ScenePlugin>());
    assert!(!app.is_plugin_added::<Box<dyn Plugin>>());
    assert!(matches!(
        app.try_add_plugins(ConfiguredPlugin(99)),
        Err(PluginError::AlreadyAdded { .. })
    ));
    let another: Box<dyn Plugin> = Box::new(ScenePlugin);
    assert!(matches!(
        app.try_add_plugins(another),
        Err(PluginError::AlreadyAdded { .. })
    ));
}

#[test]
fn failed_plugin_build_returns_an_owned_error_and_can_be_retried() {
    struct Failing;
    impl Plugin for Failing {
        fn build(&self, _: &mut App) {
            panic!("invalid plugin configuration");
        }
    }
    let mut app = App::new();
    let error = app.try_add_plugins(Failing).err().unwrap();
    assert!(matches!(error, PluginError::Build { ref message, .. }
        if message == "invalid plugin configuration"));
    assert!(!app.is_plugin_added::<Failing>());
    assert!(matches!(
        app.try_add_plugins(Failing),
        Err(PluginError::Build { .. })
    ));
}

#[test]
fn recursive_installation_is_reported_as_a_build_error() {
    struct Recursive;
    impl Plugin for Recursive {
        fn build(&self, app: &mut App) {
            app.add_plugins(Recursive);
        }
    }
    let mut app = App::new();
    let error = app.try_add_plugins(Recursive).err().unwrap();
    assert!(matches!(error, PluginError::Build { ref message, .. }
        if message.contains("is already added")));
}

#[test]
fn panic_payload_is_destroyed_before_its_error_is_returned() {
    struct Payload(Arc<AtomicUsize>);
    impl Drop for Payload {
        fn drop(&mut self) {
            self.0.fetch_add(1, Ordering::Relaxed);
        }
    }
    let drops = Arc::new(AtomicUsize::new(0));
    let result = catch_plugin_panic(|| std::panic::panic_any(Payload(Arc::clone(&drops))));
    assert!(matches!(result, Err(ref message)
        if message == "plugin panicked with a non-string payload"));
    assert_eq!(drops.load(Ordering::Relaxed), 1);
}

#[test]
fn scene_descriptors_and_resources_drop_with_the_app() {
    struct DropRecord(&'static str, Arc<Mutex<Vec<&'static str>>>);
    impl Drop for DropRecord {
        fn drop(&mut self) {
            self.1.lock().unwrap().push(self.0);
        }
    }

    let dropped = Arc::new(Mutex::new(Vec::new()));
    let callback = DropRecord("callback", Arc::clone(&dropped));
    let mut app = App::new();
    app.add_scene(Scene::new("intro", move |_: &mut RanimScene| {
        let _ = &callback;
    }))
    .insert_resource(DropRecord("resource", Arc::clone(&dropped)));

    let descriptor = app.scenes()[0].clone();
    drop(app);
    assert_eq!(*dropped.lock().unwrap(), ["resource"]);
    drop(descriptor);
    assert_eq!(*dropped.lock().unwrap(), ["resource", "callback"]);
}
