use std::{
    thread::{self},
    time::Duration,
};

use krates::Kid;
use notify_debouncer_full::{DebouncedEvent, Debouncer};
use ranim::app::{AppCmd, AppState};

use async_channel::{Receiver, bounded, unbounded};
use log::{info, trace};
use notify::RecursiveMode;

use crate::{
    RanimUserLibraryBuilder,
    cli::Args,
    workspace::{Workspace, get_target_package},
};

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

pub fn preview_command(args: &Args) {
    info!("Loading workspace...");
    let workspace = Workspace::current().unwrap();

    // Get the target package
    info!("Getting target package...");
    let (kid, package_name) = get_target_package(&workspace, args);
    info!("Target package name: {}", package_name);

    info!("Watching package...");
    let (_watcher, rx) = watch_krate(&workspace, &kid);

    let current_dir = std::env::current_dir().expect("Failed to get current directory");
    let mut builder = RanimUserLibraryBuilder::new(
        workspace.clone(),
        package_name.clone(),
        args.clone(),
        current_dir.clone(),
    );

    info!("Initial build");
    builder.start_build();
    let lib = builder
        .res_rx
        .recv_blocking()
        .unwrap()
        .expect("Failed on initial build");

    let scene = lib.get_preview_func();
    let app = AppState::new_with_title(scene.constructor, scene.name.to_string());
    let cmd_tx = app.cmd_tx.clone();

    let res_rx = builder.res_rx.clone();
    let (shutdown_tx, shutdown_rx) = bounded(1);
    let daemon = thread::spawn(move || {
        let mut lib = Some(lib);
        loop {
            if let Ok(events) = rx.try_recv() {
                for event in events {
                    info!("{:?}: {:?}", event.kind, event.paths);
                }
                builder.start_build();
            }
            if let Ok(new_lib) = res_rx.try_recv() {
                if let Ok(new_lib) = new_lib {
                    let scene = new_lib.get_preview_func();

                    let (tx, rx) = bounded(1);
                    cmd_tx
                        .send_blocking(AppCmd::ReloadScene(scene.constructor, tx))
                        .unwrap();
                    rx.recv_blocking().unwrap();
                    lib.replace(new_lib);
                }
            }
            if let Ok(_) = shutdown_rx.try_recv() {
                info!("exiting event loop...");
                break;
            }
            std::thread::sleep(Duration::from_millis(200));
        }
    });
    ranim::app::run_app(app);
    shutdown_tx.send_blocking(()).unwrap();
    daemon.join().unwrap();
}
