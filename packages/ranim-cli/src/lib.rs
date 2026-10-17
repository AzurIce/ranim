// #![warn(missing_docs)]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![allow(rustdoc::private_intra_doc_links)]
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/AzurIce/ranim/refs/heads/main/assets/ranim.svg",
    html_favicon_url = "https://raw.githubusercontent.com/AzurIce/ranim/refs/heads/main/assets/ranim.svg"
)]

use anyhow::{Context, Result};
use async_channel::{Receiver, Sender, bounded};
use ranim::{App, DynamicPlugin, SceneHandle};
use std::{
    fmt,
    path::{Path, PathBuf},
    process::Command,
    sync::Arc,
    thread::{self, JoinHandle},
};
use tracing::{error, info};

use crate::{
    cli::{Cli, CliArgs},
    workspace::{Workspace, get_target_package},
};

pub mod cli;
pub mod workspace;

#[derive(Debug)]
struct BuildCancelled;

impl fmt::Display for BuildCancelled {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("build cancelled")
    }
}

impl std::error::Error for BuildCancelled {}

#[derive(Clone)]
pub struct BuildProcess {
    workspace: Arc<Workspace>,
    package_name: String,
    target: Target,
    args: CliArgs,
    current_dir: PathBuf,

    res_tx: Sender<Result<RanimUserLibrary>>,
    cancel_rx: Receiver<()>,
}

impl BuildProcess {
    pub fn build(self) {
        if let Err(err) = cargo_build(
            &self.current_dir,
            &self.package_name,
            &self.target,
            &self.args,
            Some(self.cancel_rx),
        ) {
            if err.downcast_ref::<BuildCancelled>().is_some() {
                info!("Build cancelled");
                return;
            }
            error!("Failed to build package: {err:?}");
            self.res_tx
                .send_blocking(Err(anyhow::anyhow!("Failed to build package: {err:?}")))
                .unwrap();
        } else {
            let dylib_path = get_dylib_path(
                &self.workspace,
                &self.package_name,
                &self.target,
                &self.args.args,
            );
            info!("loading {dylib_path:?}...");

            self.res_tx
                .send_blocking(RanimUserLibrary::load(dylib_path))
                .unwrap();
        }
    }
}

pub struct RanimUserLibraryBuilder {
    pub res_rx: Receiver<Result<RanimUserLibrary>>,
    cancel_tx: Sender<()>,

    build_process: BuildProcess,
    building_handle: Option<JoinHandle<()>>,
}

impl RanimUserLibraryBuilder {
    pub fn new(
        workspace: Arc<Workspace>,
        package_name: String,
        target: Target,
        args: CliArgs,
        current_dir: PathBuf,
    ) -> Self {
        let (res_tx, res_rx) = bounded(1);
        let (cancel_tx, cancel_rx) = bounded(1);

        let build_process = BuildProcess {
            workspace,
            package_name,
            target,
            args,
            current_dir,
            res_tx,
            cancel_rx,
        };

        Self {
            res_rx,
            cancel_tx,
            build_process,
            building_handle: None,
        }
    }

    /// This will cancel the previous build
    pub fn start_build(&mut self) {
        info!("Start build");
        self.cancel_previous_build();
        let builder = self.build_process.clone();
        self.building_handle = Some(thread::spawn(move || builder.build()));
    }

    pub fn cancel_previous_build(&mut self) {
        if let Some(building_handle) = self.building_handle.take()
            && !building_handle.is_finished()
        {
            info!("Canceling previous build...");
            if let Err(err) = self.cancel_tx.try_send(())
                && err.is_closed()
            {
                panic!("Failed to cancel build: {err:?}");
            }
            building_handle.join().unwrap();
        }
    }
}

impl Drop for RanimUserLibraryBuilder {
    fn drop(&mut self) {
        self.cancel_previous_build();
    }
}

pub struct RanimUserLibrary {
    app: Arc<App>,
}

impl RanimUserLibrary {
    pub fn load(dylib_path: impl AsRef<Path>) -> Result<Self> {
        let dylib_path = dylib_path.as_ref();
        // The CLI and plugin are built with the same Rust toolchain and
        // dependency versions, as required by the plugin's Rust ABI.
        let plugin = unsafe { DynamicPlugin::load(dylib_path) }
            .with_context(|| format!("failed to load user plugin {}", dylib_path.display()))?;
        let mut app = App::new();
        app.try_add_plugins(plugin)
            .context("failed to initialize project plugin")?;
        Ok(Self { app: Arc::new(app) })
    }

    pub fn app(&self) -> &Arc<App> {
        &self.app
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub enum Target {
    #[default]
    Lib,
    Example(String),
}

impl From<cli::TargetArg> for Target {
    fn from(arg: cli::TargetArg) -> Self {
        if arg.lib {
            Target::Lib
        } else if let Some(example) = arg.example {
            Target::Example(example)
        } else {
            Self::default()
        }
    }
}

/// Resolve the cargo target and package that should be built from the CLI args.
///
/// This is the shared entry point for all commands that need to build a user
/// dylib: `-p/--package` takes precedence, `--example` is resolved to the
/// package that declares it, and the current directory is otherwise used to
/// find the workspace member.
pub fn resolve_build_target(
    args: &CliArgs,
) -> Result<(Arc<Workspace>, krates::Kid, String, Target)> {
    let workspace = Workspace::current()?;
    let (kid, package_name) = get_target_package(&workspace, args)?;
    let target = Target::from(args.target.clone());
    tracing::debug!(?kid, %package_name, ?target, "resolved build target");
    Ok((workspace, kid, package_name, target))
}

/// Build the selected user dylib and load it as a [`RanimUserLibrary`].
pub(crate) fn build_user_library(
    workspace: Arc<Workspace>,
    package_name: String,
    target: Target,
    args: &CliArgs,
) -> Result<RanimUserLibrary> {
    let current_dir = std::env::current_dir().context("Failed to get current directory")?;
    let mut builder =
        RanimUserLibraryBuilder::new(workspace, package_name, target, args.clone(), current_dir);

    builder.start_build();
    builder
        .res_rx
        .recv_blocking()
        .context("Build worker exited without reporting")?
}

/// Resolve the target, build its dylib and load it.
pub fn load_user_library(args: &CliArgs) -> Result<RanimUserLibrary> {
    let (workspace, _kid, package_name, target) = resolve_build_target(args)?;
    build_user_library(workspace, package_name, target, args)
}

/// Select one scene from a loaded user library.
///
/// If `name` is omitted and the library contains exactly one scene, that scene
/// is returned. If the library contains multiple scenes, the caller must
/// choose one and a list of available scenes is included in the error.
pub fn select_scene(lib: &RanimUserLibrary, name: Option<&str>) -> Result<SceneHandle> {
    let app = lib.app();
    let scenes = app.scenes();
    if scenes.is_empty() {
        anyhow::bail!("no scenes found in the user library");
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
        None => anyhow::bail!(
            "multiple scenes found, choose one explicitly. Available scenes: {available}"
        ),
    }
}

pub fn cargo_build(
    path: impl AsRef<Path>,
    package: &str,
    target: &Target,
    args: &CliArgs,
    cancel_rx: Option<Receiver<()>>,
) -> Result<()> {
    let path = path.as_ref();
    let mut cmd = Command::new("cargo");
    cmd.args(["build", "-p", package, "--color=always"])
        // .env("RUSTFLAGS", "-C prefer_dynamic")
        .current_dir(path);
    match target {
        Target::Lib => {
            cmd.arg("--lib");
        }
        Target::Example(x) => {
            cmd.args(["--example", x]);
        }
    }

    if !args.features.is_empty() {
        cmd.args(std::iter::once("--features").chain(args.features.iter().map(|s| s.as_str())));
    }

    cmd.args(&args.args);

    // Start an async task to wait for completion
    let mut child = match cmd.spawn() {
        Ok(child) => child,
        Err(e) => {
            anyhow::bail!("Failed to start cargo build: {}", e)
        }
    };

    loop {
        if cancel_rx
            .as_ref()
            .and_then(|rx| rx.try_recv().ok())
            .is_some()
        {
            child.kill().unwrap();
            child.wait().unwrap();

            return Err(BuildCancelled.into());
        }
        match child.try_wait() {
            Ok(res) => {
                if let Some(status) = res {
                    if status.success() {
                        info!("Build successful!");
                        return Ok(());
                    } else {
                        anyhow::bail!("Build failed with exit code: {:?}", status.code());
                    }
                }
            }
            Err(err) => {
                anyhow::bail!("build process error: {}", err);
            }
        }
    }
}

fn get_dylib_path(
    workspace: &Workspace,
    package_name: &str,
    target: &Target,
    args: &[String],
) -> PathBuf {
    // Construct the dylib path
    let mut target_dir = workspace
        .krates
        .workspace_root()
        .as_std_path()
        .join("target")
        .join(if args.contains(&"--release".to_string()) {
            "release"
        } else {
            "debug"
        });
    if let Target::Example(_) = target {
        target_dir = target_dir.join("examples");
    }

    let artifact_name = match target {
        Target::Lib => package_name,
        Target::Example(example) => example,
    };

    #[cfg(target_os = "windows")]
    let dylib_name = format!("{}.dll", artifact_name.replace("-", "_"));

    #[cfg(target_os = "macos")]
    let dylib_name = format!("lib{}.dylib", artifact_name.replace("-", "_"));

    #[cfg(target_os = "linux")]
    let dylib_name = format!("lib{}.so", artifact_name.replace("-", "_"));

    target_dir.join(dylib_name)
}

/// This should only be called once per process.
///
/// If you use [`main`], this is called automatically.
pub fn init_tracing() {
    use tracing::level_filters::LevelFilter;
    use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

    fn build_filter() -> EnvFilter {
        const DEFAULT_DIRECTIVES: &[(&str, LevelFilter)] = &[
            ("ranim_cli", LevelFilter::INFO),
            ("ranim", LevelFilter::INFO),
        ];
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

/// main
pub fn main() {
    use clap::Parser;
    init_tracing();
    if let Err(err) = Cli::parse().run() {
        error!("{err:#}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use ranim::Scene;

    use super::*;

    fn empty_constructor(_scene: &mut ranim::RanimScene) {}

    fn scene(name: &str) -> Scene {
        Scene::new(name, empty_constructor)
    }

    #[test]
    fn select_scene_resolves_omitted_and_named_scenes() {
        let mut app = App::new();
        app.add_scene(scene("only"));
        let single = RanimUserLibrary { app: Arc::new(app) };
        assert_eq!(select_scene(&single, None).unwrap().name, "only");

        let mut app = App::new();
        app.add_scenes([scene("a"), scene("b")]);
        let lib = RanimUserLibrary { app: Arc::new(app) };
        let err = match select_scene(&lib, None) {
            Ok(_) => panic!("expected an error for multiple scenes"),
            Err(err) => err,
        };
        assert!(err.to_string().contains("Available scenes: a, b"));

        let selected = select_scene(&lib, Some("b")).unwrap();
        assert_eq!(selected.name, "b");
        drop(lib);
        assert_eq!(selected.build_scene().total_secs(), 0.0);
    }

    #[test]
    fn select_scene_reports_empty_and_missing_scenes() {
        let mut lib = RanimUserLibrary {
            app: Arc::new(App::new()),
        };
        let Err(err) = select_scene(&lib, None) else {
            panic!("expected an error for an empty project");
        };
        assert!(err.to_string().contains("no scenes found"));

        Arc::get_mut(&mut lib.app)
            .unwrap()
            .add_scene(scene("available"));
        let Err(err) = select_scene(&lib, Some("missing")) else {
            panic!("expected an error for an unknown scene");
        };
        assert!(err.to_string().contains("failed to find scene `missing`"));
        assert!(err.to_string().contains("Available scenes: available"));
    }

    #[test]
    fn scene_handle_keeps_project_resources_alive() {
        use std::sync::atomic::{AtomicBool, Ordering};

        struct ResourceGuard(Arc<AtomicBool>);

        impl Drop for ResourceGuard {
            fn drop(&mut self) {
                self.0.store(true, Ordering::SeqCst);
            }
        }

        let dropped = Arc::new(AtomicBool::new(false));
        let mut app = App::new();
        app.add_scene(scene("test"))
            .insert_resource(ResourceGuard(Arc::clone(&dropped)));
        let lib = RanimUserLibrary { app: Arc::new(app) };
        let selected = select_scene(&lib, Some("test")).unwrap();

        drop(lib);
        assert!(!dropped.load(Ordering::SeqCst));
        assert_eq!(selected.build_scene().total_secs(), 0.0);
        drop(selected);
        assert!(dropped.load(Ordering::SeqCst));
    }
}
