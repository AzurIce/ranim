use std::{
    path::{Path, PathBuf},
    process::Command,
    thread::{self, JoinHandle},
    time::Duration,
};

use anyhow::Result;
use krates::Kid;
use notify_debouncer_full::{DebouncedEvent, Debouncer};
use ranim::{
    app::{AppCmd, AppState},
};

use async_channel::{Receiver, Sender, bounded, unbounded};
use notify::RecursiveMode;
use tracing::{error, info, trace};

use crate::{cli::Args, workspace::Workspace, RanimUserLibrary};

fn watch_krate(
    workspace: &Workspace,
    kid: &Kid,
) -> (
    Debouncer<notify::ReadDirectoryChangesWatcher, notify_debouncer_full::FileIdMap>,
    Receiver<Vec<DebouncedEvent>>,
) {
    let (tx, rx) = unbounded();

    let mut debouncer =
        notify_debouncer_full::new_debouncer(Duration::from_millis(500), None, move |evt| {
            let Ok(evt) = evt else {
                return;
            };
            _ = tx.try_send(evt)
        })
        .expect("Failed to create debounced watcher");

    // All krates need to be watched, including the main package.
    let mut watch_krates = vec![];
    if let krates::Node::Krate { krate, .. } = workspace.krates.node_for_kid(kid).unwrap() {
        watch_krates.push(krate);
    }
    watch_krates.extend(
        workspace
            .krates
            .get_deps(workspace.krates.nid_for_kid(&kid).unwrap())
            .filter_map(|(dep, _)| {
                let krate = match dep {
                    krates::Node::Krate { krate, .. } => krate,
                    krates::Node::Feature { krate_index, .. } => {
                        &workspace.krates[krate_index.index()]
                    }
                };
                if krate
                    .manifest_path
                    .components()
                    .any(|c| c.as_str() == ".cargo")
                {
                    None
                } else {
                    Some(krate)
                }
            }),
    );

    let watch_krate_roots = watch_krates
        .into_iter()
        .map(|krate| {
            krate
                .manifest_path
                .parent()
                .unwrap()
                .to_path_buf()
                .into_std_path_buf()
        })
        .collect::<Vec<_>>();

    let mut watch_paths = vec![];
    for krate_root in &watch_krate_roots {
        trace!("Adding watched dir for krate root {:?}", krate_root);
        let ignore_builder = ignore::gitignore::GitignoreBuilder::new(krate_root);
        let ignore = ignore_builder.build().unwrap();

        for entry in krate_root
            .read_dir()
            .into_iter()
            .flatten()
            .filter_map(|entry| entry.ok())
            .filter(|entry| {
                !ignore
                    .matched(entry.path(), entry.path().is_dir())
                    .is_ignore()
            })
            .filter(|entry| {
                !workspace
                    .ignore
                    .matched(entry.path(), entry.path().is_dir())
                    .is_ignore()
            })
        {
            trace!("Watching path {:?}", entry.path());
            watch_paths.push(entry.path().to_path_buf());
        }
    }
    watch_paths.dedup();

    for path in &watch_paths {
        trace!("Watching path {:?}", path);
        debouncer
            .watch(path, RecursiveMode::Recursive)
            .expect("Failed to watch path");
    }

    // Some more?

    (debouncer, rx)
}

pub fn preview_command(args: Args) {
    info!("Loading workspace...");
    let workspace = Workspace::current().unwrap();

    // Get the target package
    info!("Getting target package...");
    let kid = if let Some(package) = args.package.as_ref() {
        workspace.get_package(&package)
    } else {
        workspace.main_package()
    }
    .expect("no package");
    let package_name =
        if let krates::Node::Krate { krate, .. } = workspace.krates.node_for_kid(&kid).unwrap() {
            krate.name.to_string()
        } else {
            unreachable!()
        };
    info!("Target package name: {}", package_name);

    info!("Watching package...");
    let (_watcher, rx) = watch_krate(&workspace, &kid);

    let current_dir = std::env::current_dir().expect("Failed to get current directory");

    let (res_tx, res_rx) = bounded(1);
    let mut build_task: Option<(JoinHandle<()>, Sender<()>)> = None;
    let build_fn = move |res_tx: Sender<Result<RanimUserLibrary>>,
                         build_task: &mut Option<(JoinHandle<()>, Sender<()>)>| {
        let (tx, rx) = bounded(1);
        let _current_dir = current_dir.clone();
        let _package_name = package_name.clone();
        let _args = args.clone();
        let _workspace = workspace.clone();
        *build_task = Some((
            thread::spawn(move || {
                let current_dir = _current_dir;
                let package_name = _package_name;
                let args = _args;
                let workspace = _workspace;
                if let Err(err) = cargo_build(&current_dir, &package_name, &args, Some(rx)) {
                    error!("Failed to build package: {:?}", err);
                    res_tx
                        .send_blocking(Err(anyhow::anyhow!("Failed to build package: {:?}", err)))
                        .unwrap();
                } else {
                    let dylib_path = get_dylib_path(&workspace, &package_name, &args.args);
                    // let tmp_dir = std::env::temp_dir();
                    info!("loading {:?}...", dylib_path);

                    let lib = RanimUserLibrary::load(dylib_path);
                    res_tx.send_blocking(Ok(lib)).unwrap();
                }
            }),
            tx,
        ));
    };

    info!("Initial build");
    build_fn(res_tx.clone(), &mut build_task);
    let lib = res_rx
        .recv_blocking()
        .unwrap()
        .expect("Failed on initial build");

    let scene = lib.get_preview_func();
    // let scene = (preview_func.constructor)()();
    let app = AppState::new_with_title(scene.constructor, scene.name.to_string());
    let cmd_tx = app.cmd_tx.clone();

    let (shutdown_tx, shutdown_rx) = bounded(1);
    let daemon = thread::spawn(move || {
        let mut lib = Some(lib);
        loop {
            if let Ok(events) = rx.try_recv() {
                for event in events {
                    info!("{:?}: {:?}", event.kind, event.paths);
                }
                if let Some((task, tx)) = build_task.take() {
                    if !task.is_finished() {
                        info!("Cancelling previous build...");
                        tx.send_blocking(()).unwrap();
                    }
                }
                build_fn(res_tx.clone(), &mut build_task);
            }
            if let Ok(new_lib) = res_rx.try_recv() {
                if let Ok(new_lib) = new_lib {
                    let scene = new_lib.get_preview_func();
                    // let scene = (preview_func.fn_ptr)();

                    let (tx, rx) = bounded(1);
                    cmd_tx
                        .send_blocking(AppCmd::ReloadScene(scene.constructor, tx))
                        .unwrap();
                    rx.recv_blocking().unwrap();
                    lib.replace(new_lib);
                }
            }
            if let Ok(_) = shutdown_rx.try_recv() {
                break;
            }
            std::thread::sleep(Duration::from_millis(200));
        }
    });
    ranim::app::run_app(app);
    shutdown_tx.send_blocking(());
    daemon.join().unwrap();
}

fn get_dylib_path(workspace: &Workspace, package_name: &str, args: &[String]) -> PathBuf {
    // Construct the dylib path
    let target_dir = workspace
        .krates
        .workspace_root()
        .as_std_path()
        .join("target")
        .join(if args.contains(&"--release".to_string()) {
            "release"
        } else {
            "debug"
        });

    #[cfg(target_os = "windows")]
    let dylib_name = format!("{}.dll", package_name.replace("-", "_"));

    #[cfg(target_os = "macos")]
    let dylib_name = format!("lib{}.dylib", package_name.replace("-", "_"));

    #[cfg(target_os = "linux")]
    let dylib_name = format!("lib{}.so", package_name.replace("-", "_"));

    target_dir.join(dylib_name)
}

fn cargo_build(
    path: impl AsRef<Path>,
    package: &String,
    args: &Args,
    cancel_rx: Option<Receiver<()>>,
) -> Result<()> {
    let path = path.as_ref();
    info!("Cargo building...");
    let mut cmd = Command::new("cargo");
    cmd.args([
        "build",
        "-p",
        package.as_str(),
        "--lib",
        "--color=always",
        // "--message-format=json-render-diagnostics",
    ])
    .current_dir(path);
    cmd.args(&args.args);

    // Start an async task to wait for completion
    let mut child = match cmd.spawn() {
        Ok(child) => child,
        Err(e) => {
            error!("Failed to start cargo build: {}", e);
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

            anyhow::bail!("build cancelled");
        }
        match child.try_wait() {
            Ok(res) => {
                if let Some(status) = res {
                    if status.success() {
                        info!("Build successful!");
                        return Ok(());
                    } else {
                        error!("Build failed with exit code: {:?}", status.code());
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
