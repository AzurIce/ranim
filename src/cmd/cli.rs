//! The command line interface served by [`AppRunExt::run`].

use std::sync::Arc;

use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};
use tracing::{error, info};

use crate::{App, Output, SceneHandle};

use super::inspect::{self, InspectCommand};

/// Initialize tracing logging for a project binary.
///
/// Called automatically by [`AppRunExt::run`]. `RUST_LOG` overrides the
/// default `ranim=info` directives.
pub fn init_tracing() {
    use tracing::level_filters::LevelFilter;
    use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

    fn build_filter() -> EnvFilter {
        const DEFAULT_DIRECTIVES: &[(&str, LevelFilter)] = &[("ranim", LevelFilter::INFO)];
        let mut filter = EnvFilter::from_default_env();
        let env = std::env::var("RUST_LOG").unwrap_or_default();
        for (name, level) in DEFAULT_DIRECTIVES
            .iter()
            .filter(|(name, _)| !env.contains(name))
        {
            filter = filter.add_directive(format!("{name}={level}").parse().unwrap());
        }
        filter
    }

    let indicatif_layer = tracing_indicatif::IndicatifLayer::new();

    tracing_subscriber::registry()
        .with(fmt::layer().with_writer(indicatif_layer.get_stderr_writer()))
        .with(indicatif_layer)
        .with(build_filter())
        .init();
}

/// Select one scene of the project.
///
/// If `name` is omitted and the project contains exactly one scene, that scene
/// is returned. If the project contains multiple scenes, the caller must
/// choose one and a list of available scenes is included in the error.
pub fn select_scene(app: &Arc<App>, name: Option<&str>) -> Result<SceneHandle> {
    let scenes = app.scenes();
    if scenes.is_empty() {
        bail!("no scenes found in the project");
    }

    let available = scenes
        .iter()
        .map(|scene| scene.name.as_str())
        .collect::<Vec<_>>()
        .join(", ");

    match name {
        Some(name) => app.scene(name).with_context(|| {
            format!("failed to find scene `{name}`. Available scenes: {available}")
        }),
        None if scenes.len() == 1 => Ok(app.first_scene().unwrap()),
        None => {
            bail!("multiple scenes found, choose one explicitly. Available scenes: {available}")
        }
    }
}

/// Render every [`crate::Scene`] output declared with `#[output(...)]`.
pub fn output_command(app: &Arc<App>, scenes: &[String], buffer_count: usize) -> Result<()> {
    use super::render::render_scene;

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
pub fn render_command(app: &Arc<App>, scene_name: &str, buffer_count: usize) -> Result<()> {
    use super::render::render_scene_once;

    let scene = select_scene(app, Some(scene_name))?;

    let output = Output::default();
    info!(
        "Rendering scene {:?} once with default output: {}x{} {}fps {}",
        scene.name, output.width, output.height, output.fps, output.format
    );
    render_scene_once(&scene, &output, buffer_count);
    Ok(())
}

/// Preview a scene in the interactive preview app.
#[cfg(feature = "preview")]
pub fn preview_command(app: &Arc<App>, scene_name: Option<&str>) -> Result<()> {
    let scene = match scene_name {
        Some(scene) => app.scene(scene),
        None => app.first_scene(),
    }
    .with_context(|| {
        format!(
            "failed to find preview scene. Available scenes: {}",
            app.scenes()
                .iter()
                .map(|scene| scene.name.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        )
    })?;
    super::preview::preview_scene(scene);
    Ok(())
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Launch the interactive preview app (defaults to the first scene)
    Preview { scene: Option<String> },
    /// Render every `#[output(...)]` declared by the selected scenes
    Output {
        /// Optional scene names to render (if not provided, render all scenes)
        #[arg(num_args = 0..)]
        scenes: Vec<String>,

        /// Number of GPU readback buffers (higher = more parallelism, more VRAM)
        #[arg(long, default_value_t = 2)]
        buffer_count: usize,
    },
    /// Render a single scene once with default output settings (`1920x1080`, 60 fps, mp4)
    Render {
        /// Scene name to render
        scene: String,

        /// Number of GPU readback buffers (higher = more parallelism, more VRAM)
        #[arg(long, default_value_t = 2)]
        buffer_count: usize,
    },
    /// Inspect scenes, animation trees, or evaluated frames without rendering
    Inspect {
        #[command(subcommand)]
        command: InspectCommand,
    },
}

#[derive(Parser, Debug)]
#[command(about = "A Ranim animation project")]
struct ProjectCli {
    #[command(subcommand)]
    command: Commands,
}

impl ProjectCli {
    fn run(self, app: Arc<App>) -> Result<()> {
        match self.command {
            Commands::Preview { scene } => {
                #[cfg(feature = "preview")]
                {
                    preview_command(&app, scene.as_deref())
                }
                #[cfg(not(feature = "preview"))]
                {
                    let _ = scene;
                    bail!("this binary was built without the `preview` feature")
                }
            }
            Commands::Output {
                scenes,
                buffer_count,
            } => output_command(&app, &scenes, buffer_count),
            Commands::Render {
                scene,
                buffer_count,
            } => render_command(&app, &scene, buffer_count),
            Commands::Inspect { command } => inspect::inspect_command(&app, command),
        }
    }
}

/// Command line entry point for a Ranim project binary.
///
/// A project binary assembles its [`App`] and calls
/// [`run`](AppRunExt::run) to serve the `preview`, `output`, `render`, and
/// `inspect` subcommands:
///
/// ```ignore
/// use ranim::prelude::*;
///
/// fn main() {
///     App::new().add_plugins(/* project plugins */()).run();
/// }
/// ```
pub trait AppRunExt {
    /// Parse the process arguments and run the requested command.
    ///
    /// The app's scenes and resources are moved into the running session.
    /// On a command failure the error is logged and the process exits with
    /// a non-zero status.
    fn run(&mut self);
}

impl AppRunExt for App {
    fn run(&mut self) {
        init_tracing();
        let result = ProjectCli::parse().run(Arc::new(std::mem::take(self)));
        if let Err(err) = result {
            error!("{err:#}");
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scene(name: &str) -> crate::Scene {
        crate::Scene::new(name, |_: &mut crate::RanimScene| {})
    }

    #[test]
    fn select_scene_resolves_omitted_and_named_scenes() {
        let mut app = App::new();
        app.add_scene(scene("only"));
        let app = Arc::new(app);
        assert_eq!(select_scene(&app, None).unwrap().name, "only");

        let mut app = App::new();
        app.add_scenes([scene("a"), scene("b")]);
        let app = Arc::new(app);
        let err = match select_scene(&app, None) {
            Ok(_) => panic!("expected an error for multiple scenes"),
            Err(err) => err,
        };
        assert!(err.to_string().contains("Available scenes: a, b"));

        let selected = select_scene(&app, Some("b")).unwrap();
        assert_eq!(selected.name, "b");
        assert_eq!(selected.build_scene().total_secs(), 0.0);
    }

    #[test]
    fn select_scene_reports_empty_and_missing_scenes() {
        let app = Arc::new(App::new());
        let Err(err) = select_scene(&app, None) else {
            panic!("expected an error for an empty project");
        };
        assert!(err.to_string().contains("no scenes found"));

        let mut app = App::new();
        app.add_scene(scene("available"));
        let app = Arc::new(app);
        let Err(err) = select_scene(&app, Some("missing")) else {
            panic!("expected an error for an unknown scene");
        };
        assert!(err.to_string().contains("failed to find scene `missing`"));
        assert!(err.to_string().contains("Available scenes: available"));
    }

    #[test]
    fn cli_parses_all_subcommands() {
        let parse = |args: &[&str]| ProjectCli::try_parse_from(args);

        let cli = parse(&["demo", "output"]).unwrap();
        assert!(matches!(
            cli.command,
            Commands::Output {
                scenes,
                buffer_count: 2
            } if scenes.is_empty()
        ));

        let cli = parse(&["demo", "render", "intro", "--buffer-count", "4"]).unwrap();
        assert!(matches!(
            cli.command,
            Commands::Render {
                scene,
                buffer_count: 4
            } if scene == "intro"
        ));

        let cli = parse(&["demo", "preview"]).unwrap();
        assert!(matches!(cli.command, Commands::Preview { scene: None }));

        let cli = parse(&["demo", "inspect", "scenes", "--format", "json"]).unwrap();
        assert!(matches!(
            cli.command,
            Commands::Inspect {
                command: InspectCommand::Scenes {
                    format: inspect::FormatArg::Json
                }
            }
        ));
    }
}
