//! Verify that a project plugin assembles scenes, outputs, and resources into
//! the App through direct `add_plugins` installation.

use bevy_ecs::prelude::Resource;
use ranim::{App, AppExt, OutputFormat, Plugin, SceneConstructor, prelude::*};

#[scene(name = "registered", clear_color = "#112233")]
#[output(width = 320, height = 180, fps = 24, format = "webm", name = "clip")]
#[output(width = 640, height = 360, fps = 30, dir = "./second")]
fn registered_scene(_r: &mut RanimScene) {}

#[scene]
fn unregistered_scene(_r: &mut RanimScene) {}

#[derive(Resource)]
struct HostConfig(String);

#[derive(Resource)]
struct ProjectConfig(u32);

struct ProjectPlugin(u32);

impl Plugin for ProjectPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.insert_resource(ProjectConfig(self.0))
            .add_scene(registered_scene::scene());
    }
}

#[test]
fn project_plugin_installs_only_explicit_scenes() {
    let mut project = App::new();
    project.insert_resource(HostConfig(String::from("host configuration")));
    assert!(project.scenes().is_empty());
    project.add_plugins(ProjectPlugin(42));

    assert_eq!(
        project.world().resource::<HostConfig>().0,
        "host configuration"
    );
    assert_eq!(project.world().resource::<ProjectConfig>().0, 42);
    assert!(project.is_plugin_added::<ProjectPlugin>());
    assert_eq!(project.scenes().len(), 1);
    assert!(project.scene("unregistered_scene").is_none());

    let scene = project.scene("registered").unwrap();
    assert_eq!(scene.config.clear_color, "#112233");
    assert_eq!(scene.outputs.len(), 2);
    let output = &scene.outputs[0];
    assert_eq!((output.width, output.height, output.fps), (320, 180, 24));
    assert_eq!(output.format, OutputFormat::Webm);
    assert_eq!(output.name.as_deref(), Some("clip"));
    assert_eq!(scene.outputs[1].dir, "./second");
    assert_eq!(scene.build_scene().total_secs(), 0.0);
}

#[test]
fn duplicate_plugin_types_are_rejected() {
    let mut project = App::new();
    project.add_plugins(ProjectPlugin(42));
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        project.add_plugins(ProjectPlugin(99));
    }));
    assert!(result.is_err());
    assert_eq!(project.scenes().len(), 1);
}

#[test]
fn scene_descriptors_can_exist_without_being_registered() {
    let scene = unregistered_scene::scene();
    assert_eq!(scene.outputs.len(), 1);
    assert_eq!(scene.outputs[0].format, OutputFormat::Mp4);
    assert_eq!(scene.outputs[0].width, 1920);
}
