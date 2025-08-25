use anyhow::{Context, Result};
use async_channel::{Receiver, Sender, bounded};
use libloading::{Library, Symbol};
use log::{error, info};
use ranim::Scene;
use std::{
    path::{Path, PathBuf},
    process::Command,
    sync::Arc,
    thread::{self, JoinHandle},
};

use crate::{cli::Args, workspace::Workspace};

pub mod cli;
pub mod workspace;

#[derive(Clone)]
pub struct BuildProcess {
    workspace: Arc<Workspace>,
    package_name: String,
    args: Args,
    current_dir: PathBuf,

    res_tx: Sender<Result<RanimUserLibrary>>,
    cancel_rx: Receiver<()>,
}

impl BuildProcess {
    pub fn build(self) {
        if let Err(err) = cargo_build(
            &self.current_dir,
            &self.package_name,
            &self.args,
            Some(self.cancel_rx),
        ) {
            error!("Failed to build package: {err:?}");
            self.res_tx
                .send_blocking(Err(anyhow::anyhow!("Failed to build package: {err:?}")))
                .unwrap();
        } else {
            let dylib_path = get_dylib_path(&self.workspace, &self.package_name, &self.args.args);
            // let tmp_dir = std::env::temp_dir();
            info!("loading {dylib_path:?}...");

            let lib = RanimUserLibrary::load(dylib_path);
            self.res_tx.send_blocking(Ok(lib)).unwrap();
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
        args: Args,
        current_dir: PathBuf,
    ) -> Self {
        let (res_tx, res_rx) = bounded(1);
        let (cancel_tx, cancel_rx) = bounded(1);

        let build_process = BuildProcess {
            workspace,
            package_name,
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
    inner: Option<Library>,
    temp_path: PathBuf,
}

pub struct RanimUserLibrarySceneIter<'a> {
    lib: &'a RanimUserLibrary,
    idx: usize,
}

impl<'a> Iterator for RanimUserLibrarySceneIter<'a> {
    type Item = &'a Scene;

    fn next(&mut self) -> Option<Self::Item> {
        let res = self.lib.get_scene(self.idx);
        self.idx += 1;
        res
    }
}

impl RanimUserLibrary {
    pub fn load(dylib_path: impl AsRef<Path>) -> Self {
        let dylib_path = dylib_path.as_ref();

        let temp_dir = std::env::temp_dir();
        let file_name = dylib_path.file_name().unwrap();

        // 使用时间戳和随机数确保每次都有唯一的临时文件名
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let temp_path = temp_dir.join(format!(
            "ranim_{}_{}_{}",
            std::process::id(),
            timestamp,
            file_name.to_string_lossy()
        ));

        std::fs::copy(dylib_path, &temp_path).unwrap();

        let lib = unsafe { Library::new(&temp_path).unwrap() };
        Self {
            inner: Some(lib),
            temp_path,
        }
    }

    pub fn scene_cnt(&self) -> usize {
        let scene_cnt: Symbol<extern "C" fn() -> usize> =
            unsafe { self.inner.as_ref().unwrap().get(b"scene_cnt").unwrap() };
        scene_cnt()
    }

    pub fn get_scene(&self, idx: usize) -> Option<&Scene> {
        let get_scene: Symbol<extern "C" fn(usize) -> *const Scene> =
            unsafe { self.inner.as_ref().unwrap().get(b"get_scene").unwrap() };
        if self.scene_cnt() <= idx {
            None
        } else {
            Some(unsafe { &*get_scene(idx) })
        }
    }

    pub fn scenes(&self) -> impl Iterator<Item = &Scene> {
        RanimUserLibrarySceneIter { lib: self, idx: 0 }
    }

    pub fn get_preview_func(&self) -> Result<&Scene> {
        self.scenes()
            .find(|s| s.preview)
            .context("no scene marked with `#[preview]` found")
    }
}

impl Drop for RanimUserLibrary {
    fn drop(&mut self) {
        println!("Dropping RanimUserLibrary...");

        drop(self.inner.take());
        std::fs::remove_file(&self.temp_path).unwrap();
    }
}

fn cargo_build(
    path: impl AsRef<Path>,
    package: &str,
    args: &Args,
    cancel_rx: Option<Receiver<()>>,
) -> Result<()> {
    let path = path.as_ref();
    let mut cmd = Command::new("cargo");
    cmd.args([
        "build",
        "-p",
        package,
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
