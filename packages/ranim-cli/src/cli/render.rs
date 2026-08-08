use anyhow::{Context, Result, bail};
use ranim::{Scene, cmd::render_scene};
use tracing::{error, info};

use crate::{
    RanimUserLibraryBuilder, Target,
    cli::CliArgs,
    workspace::{Workspace, get_target_package},
};

pub fn render_command(args: &CliArgs, scenes: &[String], buffer_count: usize) -> Result<()> {
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

    let all_scenes: Vec<Scene> = lib.scenes().collect::<Vec<_>>();
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
