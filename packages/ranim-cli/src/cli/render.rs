use anyhow::{Context, Result, bail};
use ranim::{Scene, cmd::render_scene};
use tracing::{error, info};

use crate::{
    RanimUserLibraryBuilder, Target,
    cli::CliArgs,
    workspace::{Workspace, get_target_package},
};

/// Render scenes registered in a Python script (`ranim render script.py [names...]`).
#[cfg(feature = "python")]
fn render_python_script(script: &str, names: &[String], buffer_count: usize) -> Result<()> {
    use ranim::{Output, SceneConfig, cmd::render_scene_output};

    info!("Loading python script: {script}");
    ranimpy::init_python();
    let scenes = ranimpy::load_script(std::path::Path::new(script))?;

    let scenes: Vec<_> = if names.is_empty() {
        scenes.into_iter().collect()
    } else {
        scenes
            .into_iter()
            .filter(|scene| names.iter().any(|s| s == &scene.name))
            .collect()
    };

    if scenes.is_empty() {
        bail!("No scenes to render");
    }

    for scene in scenes {
        info!("Rendering scene: {}", scene.name);
        let name = scene.name.clone();
        let output = Output {
            dir: scene
                .output_dir
                .clone()
                .unwrap_or_else(|| format!("./output/{name}")),
            ..Default::default()
        };
        let mut config = SceneConfig::default();
        if let Some(clear_color) = &scene.clear_color {
            config.clear_color = clear_color.clone();
        }
        render_scene_output(scene, name, &config, &output, buffer_count);
    }
    Ok(())
}

/// If the first positional argument is a `.py` script, split it off from the
/// scene-name filter arguments.
fn split_python_script(scenes: &[String]) -> Option<(&str, &[String])> {
    let (script, names) = scenes.split_first()?;
    script
        .ends_with(".py")
        .then_some((script.as_str(), names))
}

pub fn render_command(args: &CliArgs, scenes: &[String], buffer_count: usize) -> Result<()> {
    if let Some((script, names)) = split_python_script(scenes) {
        #[cfg(feature = "python")]
        return render_python_script(script, names, buffer_count);
        #[cfg(not(feature = "python"))]
        {
            let _ = names;
            bail!(
                "Rendering python scripts requires the `python` feature of ranim-cli \
                 (e.g. `cargo run -p ranim-cli --features python -- render {script}`)"
            );
        }
    }

    info!("Loading workspace...");
    let workspace = Workspace::current().unwrap();

    // Get the target package
    info!("Getting target package...");
    let (_, package_name) = get_target_package(&workspace, args);
    info!("Target package name: {package_name}");

    // let target = args.target.clone().map(Target::from).unwrap_or_default();
    let target = Target::from(args.target.clone());
    info!("Target: {target:?}");

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
        .unwrap()
        .context("Failed on initial build")?;

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
