use anyhow::{Result, bail};
use ranim::{
    Output, SceneHandle,
    cmd::{render_scene, render_scene_once},
};
use tracing::{error, info};

use crate::{cli::CliArgs, load_user_library, select_scene};

/// Render every `#[output(...)]` declared by the selected scenes.
pub fn output_command(args: &CliArgs, scenes: &[String], buffer_count: usize) -> Result<()> {
    let lib = load_user_library(args)?;
    let app = lib.app();
    let all_scenes = app.scenes();
    let scenes_to_render: Vec<SceneHandle> = all_scenes
        .iter()
        .filter(|scene| scenes.is_empty() || scenes.iter().any(|name| name == &scene.name))
        .map(|scene| app.scene(&scene.name).unwrap())
        .collect();

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
        bail!("No scenes to render");
    }

    for scene in scenes_to_render {
        info!("Rendering scene: {}", scene.name);
        render_scene(&scene, buffer_count);
    }
    Ok(())
}

/// Render a single scene once with default output settings
/// (`1920x1080`, 60 fps, mp4). Unlike [`output_command`], this command does
/// not read any `#[output(...)]` declaration and does not process
/// `TimeMark::Capture`.
pub fn render_command(args: &CliArgs, scene_name: &str, buffer_count: usize) -> Result<()> {
    let lib = load_user_library(args)?;
    let scene = select_scene(&lib, Some(scene_name))?;

    let output = Output::default();
    info!(
        "Rendering scene {:?} once with default output: {}x{} {}fps {}",
        scene.name, output.width, output.height, output.fps, output.format
    );
    render_scene_once(&scene, &output, buffer_count);
    Ok(())
}
