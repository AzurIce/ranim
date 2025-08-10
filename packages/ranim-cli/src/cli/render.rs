use log::{error, info};
use ranim::{Scene, render_scene};

use crate::{
    RanimUserLibraryBuilder,
    cli::Args,
    workspace::{Workspace, get_target_package},
};

pub fn render_command(args: &Args, scenes: &[String]) {
    info!("Loading workspace...");
    let workspace = Workspace::current().unwrap();

    // Get the target package
    info!("Getting target package...");
    let (_, package_name) = get_target_package(&workspace, args);
    info!("Target package name: {package_name}");

    let current_dir = std::env::current_dir().expect("Failed to get current directory");
    let mut builder = RanimUserLibraryBuilder::new(
        workspace.clone(),
        package_name.clone(),
        args.clone(),
        current_dir.clone(),
    );

    builder.start_build();
    let lib = builder
        .res_rx
        .recv_blocking()
        .unwrap()
        .expect("Failed on initial build");

    let all_scenes = lib.scenes();
    let scenes_to_render: Vec<&Scene> = if scenes.is_empty() {
        all_scenes.iter().collect()
    } else {
        all_scenes
            .iter()
            .filter(|scene| scenes.iter().any(|s| s == scene.name))
            .collect()
    };

    if scenes_to_render.is_empty() {
        if scenes.is_empty() {
            info!("No scenes found to render");
        } else {
            error!("No matching scenes found for: {scenes:?}");
            error!(
                "Available scenes: {:?}",
                all_scenes.iter().map(|s| &s.name).collect::<Vec<_>>()
            );
        }
        return;
    }

    for scene in scenes_to_render {
        info!("Rendering scene: {}", scene.name);
        render_scene(scene);
    }
}
