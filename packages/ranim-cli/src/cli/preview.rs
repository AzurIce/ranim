use std::{
    path::Path,
    thread::{self},
    time::Duration,
};

use krates::Kid;
use notify_debouncer_full::{DebounceEventResult, DebouncedEvent, Debouncer};
use ranim::cmd::preview::{RanimPreviewApp, RanimPreviewAppCmd};

use anyhow::{Context, Result};
use async_channel::{Receiver, bounded, unbounded};
use notify::{EventKind, RecursiveMode};
use tracing::{error, info, trace};

use crate::{RanimUserLibraryBuilder, cli::CliArgs, resolve_build_target, workspace::Workspace};

fn is_vcs_metadata(path: &Path) -> bool {
    path.components().any(|component| {
        matches!(
            component.as_os_str().to_str(),
            Some(".git" | ".hg" | ".svn")
        )
    })
}

// Access events are plain reads. The build itself opens the watched manifests
// (`.cargo/config.toml`, every crate's `Cargo.toml`, `Cargo.lock`), so reacting
// to them makes each build trigger the next one in an endless loop.
fn is_rebuild_event(event: &DebouncedEvent) -> bool {
    !matches!(event.kind, EventKind::Access(_) | EventKind::Other)
}

fn watch_krate(
    workspace: &Workspace,
    kid: &Kid,
) -> (
    Debouncer<notify::RecommendedWatcher, notify_debouncer_full::RecommendedCache>,
    Receiver<Vec<DebouncedEvent>>,
) {
    let (tx, rx) = unbounded();

    let mut debouncer = notify_debouncer_full::new_debouncer(
        Duration::from_millis(500),
        None,
        move |evt: DebounceEventResult| {
            let Ok(mut evt) = evt else {
                return;
            };
            evt.retain(|event| !event.paths.iter().all(|path| is_vcs_metadata(path)));
            evt.retain(is_rebuild_event);
            if !evt.is_empty() {
                _ = tx.try_send(evt)
            }
        },
    )
    .expect("Failed to create debounced watcher");

    // All krates need to be watched, including the main package.
    let mut watch_krates = vec![];
    if let krates::Node::Krate { krate, .. } = workspace.krates.node_for_kid(kid).unwrap() {
        watch_krates.push(krate);
    }
    watch_krates.extend(
        workspace
            .krates
            .get_deps(workspace.krates.nid_for_kid(kid).unwrap())
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
        trace!("Adding watched dir for krate root {krate_root:?}");
        let mut ignore_builder = ignore::gitignore::GitignoreBuilder::new(krate_root);
        ignore_builder.add(krate_root.join(".gitignore"));
        let ignore = ignore_builder.build().unwrap();

        for entry in krate_root
            .read_dir()
            .into_iter()
            .flatten()
            .filter_map(|entry| entry.ok())
            .filter(|entry| !is_vcs_metadata(&entry.path()))
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
        trace!("Watching path {path:?}");
        debouncer
            .watch(path, RecursiveMode::Recursive)
            .expect("Failed to watch path");
    }

    // Some more?

    (debouncer, rx)
}

pub fn preview_command(args: &CliArgs, scene_name: &Option<String>) -> Result<()> {
    let (workspace, kid, package_name, target) = resolve_build_target(args)?;

    info!("Watching package...");
    let (_watcher, rx) = watch_krate(&workspace, &kid);

    let current_dir = std::env::current_dir().expect("Failed to get current directory");
    let mut builder = RanimUserLibraryBuilder::new(
        workspace.clone(),
        package_name.clone(),
        target,
        args.clone(),
        current_dir.clone(),
    );

    info!("Initial build");
    builder.start_build();
    let lib = builder
        .res_rx
        .recv_blocking()
        .context("Build worker exited without reporting")??;

    let scene = match scene_name {
        Some(scene) => lib.scenes().find(|s| s.name == *scene),
        None => lib.scenes().next(),
    }
    .ok_or(anyhow::anyhow!("Failed to find preview scene"))?;
    // error!("Failed to get preview scene, available scenes:");
    // for scene in lib.scenes() {
    //     info!("- {:?}", scene.name);
    // }
    // panic!("Failed to get preview scene");
    let mut app = RanimPreviewApp::new(scene.constructor, scene.name.clone(), scene.config.clone());
    app.set_clear_color_str(&scene.config.clear_color);
    let cmd_tx = app.cmd_tx.clone();

    let scene_name = scene_name.clone();
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
            if let Ok(new_lib) = res_rx.try_recv()
                && let Ok(new_lib) = new_lib
            {
                let scene = match &scene_name {
                    Some(name) => new_lib.scenes().find(|s| s.name == *name),
                    None => new_lib.scenes().next(),
                }
                .ok_or(anyhow::anyhow!("Failed to find preview scene"));
                if let Err(err) = scene {
                    error!("Failed to find preview scene: {err}");
                    continue;
                }
                let (tx, rx) = bounded(1);
                cmd_tx
                    .send_blocking(RanimPreviewAppCmd::ReloadScene(scene.unwrap(), tx))
                    .unwrap();
                rx.recv_blocking().unwrap();
                lib.replace(new_lib);
            }
            if shutdown_rx.try_recv().is_ok() {
                info!("exiting event loop...");
                break;
            }
            std::thread::sleep(Duration::from_millis(200));
        }
    });
    ranim::cmd::preview::run_app(app);
    shutdown_tx.send_blocking(()).unwrap();
    daemon.join().unwrap();
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{
        path::{Path, PathBuf},
        time::Instant,
    };

    use notify::event::{
        AccessKind, AccessMode, CreateKind, DataChange, ModifyKind, RemoveKind, RenameMode,
    };
    use notify::{Event, EventKind};

    use super::{DebouncedEvent, is_rebuild_event, is_vcs_metadata};

    fn event_of_kind(kind: EventKind) -> DebouncedEvent {
        let event = Event::new(kind).add_path(PathBuf::from("src/main.rs"));
        DebouncedEvent::new(event, Instant::now())
    }

    #[test]
    fn recognizes_vcs_metadata_at_any_depth() {
        assert!(is_vcs_metadata(Path::new("repo/.git/objects/ab/cd")));
        assert!(is_vcs_metadata(Path::new("repo/nested/.hg/store")));
        assert!(is_vcs_metadata(Path::new("repo/.svn/wc.db")));
        assert!(!is_vcs_metadata(Path::new("repo/src/git.rs")));
    }

    #[test]
    fn rebuild_on_create_modify_remove_only() {
        for kind in [
            EventKind::Create(CreateKind::File),
            EventKind::Modify(ModifyKind::Data(DataChange::Any)),
            EventKind::Modify(ModifyKind::Name(RenameMode::Both)),
            EventKind::Remove(RemoveKind::File),
        ] {
            assert!(is_rebuild_event(&event_of_kind(kind)), "{kind:?}");
        }
    }

    #[test]
    fn ignore_access_and_other_events() {
        for kind in [
            EventKind::Access(AccessKind::Open(AccessMode::Any)),
            EventKind::Access(AccessKind::Close(AccessMode::Read)),
            EventKind::Access(AccessKind::Read),
            EventKind::Other,
        ] {
            assert!(!is_rebuild_event(&event_of_kind(kind)), "{kind:?}");
        }
    }
}
