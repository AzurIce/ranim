use anyhow::{Context, Result, bail};
use ranim::{
    Output, Scene,
    cmd::{render_scene, render_scene_once},
};
use tracing::{error, info};

use crate::{
    RanimUserLibrary, RanimUserLibraryBuilder, Target,
    cli::CliArgs,
    workspace::{Workspace, get_target_package},
};

/// Build the user dylib, load it and collect its scenes.
///
/// The returned [`RanimUserLibrary`] must be kept alive for as long as the
/// scenes are used: `Scene::constructor` is a function pointer into the
/// dylib, so dropping the library unmaps the code it points to.
fn load_scenes(args: &CliArgs) -> Result<(RanimUserLibrary, Vec<Scene>)> {
    let workspace = Workspace::current()?;

    let (kid, package_name) = get_target_package(&workspace, args)?;
    let target = Target::from(args.target.clone());
    tracing::debug!(?kid, %package_name, ?target, "resolved build target");

    let current_dir = std::env::current_dir().context("Failed to get current directory")?;
    let mut builder = RanimUserLibraryBuilder::new(
        workspace.clone(),
        package_name.clone(),
        target,
        args.clone(),
        current_dir.clone(),
    );

    builder.start_build();
    let lib = builder
        .res_rx
        .recv_blocking()
        .context("Build worker exited without reporting")??;

    let scenes = lib.scenes().collect();
    Ok((lib, scenes))
}

/// Render every `#[output(...)]` declared by the selected scenes.
pub fn output_command(args: &CliArgs, scenes: &[String], buffer_count: usize) -> Result<()> {
    let (_lib, all_scenes) = load_scenes(args)?;
    let scenes_to_render: Vec<&Scene> = if scenes.is_empty() {
        all_scenes.iter().collect()
    } else {
        all_scenes
            .iter()
            .filter(|scene| scenes.iter().any(|s| s == &scene.name))
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
        bail!("No scenes to render");
    }

    for scene in scenes_to_render {
        info!("Rendering scene: {}", scene.name);
        render_scene(scene, buffer_count);
    }
    Ok(())
}

/// Render a single scene once with default output settings
/// (`1920x1080`, 60 fps, mp4). Unlike [`output_command`], this command does
/// not read any `#[output(...)]` declaration and does not process
/// `TimeMark::Capture`.
pub fn render_command(args: &CliArgs, scene_name: &str, buffer_count: usize) -> Result<()> {
    let (_lib, all_scenes) = load_scenes(args)?;

    let Some(scene) = all_scenes.iter().find(|scene| scene.name == scene_name) else {
        error!("No matching scene found for: {scene_name:?}");
        error!(
            "Available scenes: {:?}",
            all_scenes.iter().map(|s| &s.name).collect::<Vec<_>>()
        );
        bail!("No scene to render");
    };

    let output = Output::default();
    info!(
        "Rendering scene {:?} once with default output: {}x{} {}fps {}",
        scene.name, output.width, output.height, output.fps, output.format
    );
    render_scene_once(
        scene.constructor,
        scene.name.clone(),
        &scene.config,
        &output,
        buffer_count,
    );
    Ok(())
}
