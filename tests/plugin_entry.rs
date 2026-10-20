//! Verify that a project plugin assembles scenes, outputs, and resources into
//! a host App through direct `add_plugins` installation.

use ranim::{App, OutputFormat, Plugin, prelude::*};
use std::sync::Arc;

#[scene(name = "registered", clear_color = "#112233")]
#[output(width = 320, height = 180, fps = 24, format = "webm", name = "clip")]
#[output(width = 640, height = 360, fps = 30, dir = "./second")]
fn registered_scene(_r: &mut RanimScene) {}

#[scene]
fn unregistered_scene(_r: &mut RanimScene) {}

struct ProjectPlugin(u32);

impl Plugin for ProjectPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(self.0)
            .add_scene(registered_scene::scene());
    }
}

#[test]
fn project_plugin_installs_only_explicit_scenes() {
    let mut project = App::new();
    project.insert_resource(String::from("host configuration"));
    assert!(project.scenes().is_empty());
    project.add_plugins(ProjectPlugin(42));
    let project = Arc::new(project);

    assert_eq!(project.resource::<String>(), "host configuration");
    assert_eq!(*project.resource::<u32>(), 42);
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
fn boxed_and_direct_plugins_share_the_same_registration_identity() {
    let mut project = App::new();
    project.add_plugins(ProjectPlugin(42));
    let boxed: Box<dyn Plugin> = Box::new(ProjectPlugin(42));
    let Err(error) = project.try_add_plugins(boxed) else {
        panic!("the same plugin was already registered directly");
    };
    assert!(error.to_string().contains("ProjectPlugin"));
    assert_eq!(project.scenes().len(), 1);
}

#[test]
fn scene_descriptors_can_exist_without_being_registered() {
    let scene = unregistered_scene::scene();
    assert_eq!(scene.outputs.len(), 1);
    assert_eq!(scene.outputs[0].format, OutputFormat::Mp4);
    assert_eq!(scene.outputs[0].width, 1920);
}
