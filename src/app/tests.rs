use std::sync::{
    Arc, Mutex,
    atomic::{AtomicUsize, Ordering},
};

use super::*;
use crate::RanimScene;
use bevy_app::{App, Plugin};

fn scene(name: &str, constructor: fn(&mut RanimScene)) -> Scene {
    Scene::new(name, constructor)
}

#[derive(Resource)]
struct Configured(usize);

struct ConfiguredPlugin(usize);

impl Plugin for ConfiguredPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.insert_resource(Configured(self.0));
    }
}

struct ScenePlugin;

impl Plugin for ScenePlugin {
    fn build(&self, app: &mut bevy_app::App) {
        assert_eq!(app.world().resource::<Configured>().0, 42);
        app.add_scene(scene("intro", |_| {}));
    }
}

#[test]
fn configured_plugins_and_scenes_build_in_order() {
    let mut app = App::new();
    app.add_plugins((ConfiguredPlugin(42), ScenePlugin));

    assert!(app.is_plugin_added::<ConfiguredPlugin>());
    assert!(app.is_plugin_added::<ScenePlugin>());
    assert_eq!(app.world().resource::<Configured>().0, 42);
    assert_eq!(app.scenes()[0].name, "intro");
}

#[test]
#[should_panic(expected = "plugin was already added")]
fn duplicate_plugin_types_are_rejected() {
    let mut app = App::new();
    app.add_plugins(ConfiguredPlugin(1));
    app.add_plugins(ConfiguredPlugin(2));
}

#[test]
#[should_panic(expected = "scene `intro` is already registered")]
fn duplicate_scene_names_are_rejected() {
    App::new().add_scenes([scene("intro", |_| {}), scene("intro", |_| {})]);
}

#[test]
fn scene_selection_is_lazy_and_clones_descriptors() {
    static BUILDS: AtomicUsize = AtomicUsize::new(0);
    fn construct(_: &mut RanimScene) {
        BUILDS.fetch_add(1, Ordering::Relaxed);
    }

    let mut app = App::new();
    app.add_scenes([scene("intro", construct), scene("outro", |_| {})]);

    let selected = app.scene("intro").unwrap();
    assert_eq!(selected.name, "intro");
    assert_eq!(app.first_scene().unwrap().name, "intro");
    assert!(app.scene("missing").is_none());
    assert_eq!(BUILDS.load(Ordering::Relaxed), 0);

    assert_eq!(selected.build_scene().total_secs(), 0.0);
    assert_eq!(BUILDS.load(Ordering::Relaxed), 1);
    // Selection clones the descriptor, so building the selection again runs
    // the constructor again.
    assert_eq!(app.scene("intro").unwrap().build_scene().total_secs(), 0.0);
    assert_eq!(BUILDS.load(Ordering::Relaxed), 2);
}

#[test]
fn a_plugin_can_capture_its_configuration_in_a_scene_constructor() {
    struct CapturedPlugin(Arc<AtomicUsize>);

    impl Plugin for CapturedPlugin {
        fn build(&self, app: &mut bevy_app::App) {
            let builds = Arc::clone(&self.0);
            app.add_scene(Scene::new("captured", move |_: &mut RanimScene| {
                builds.fetch_add(1, Ordering::Relaxed);
            }));
        }
    }

    let builds = Arc::new(AtomicUsize::new(0));
    let mut app = App::new();
    app.add_plugins(CapturedPlugin(Arc::clone(&builds)));
    let scene = app.first_scene().unwrap();
    assert_eq!(builds.load(Ordering::Relaxed), 0);
    scene.build_scene();
    scene.build_scene();
    assert_eq!(builds.load(Ordering::Relaxed), 2);
}

#[test]
fn scene_descriptors_and_resources_drop_with_the_app() {
    #[derive(Resource)]
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

    let descriptor = app.scene("intro").unwrap();
    drop(app);
    assert_eq!(*dropped.lock().unwrap(), ["resource"]);
    drop(descriptor);
    assert_eq!(*dropped.lock().unwrap(), ["resource", "callback"]);
}
