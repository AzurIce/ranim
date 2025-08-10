use std::sync::Arc;

use anyhow::{Context, Result};
use ignore::gitignore::Gitignore;
use krates::{Kid, KrateDetails, Krates};
use log::{debug, error};

use crate::cli::Args;

pub struct Workspace {
    pub krates: Krates,
    pub ignore: Gitignore,
    pub cargo_toml: cargo_toml::Manifest,
}

impl Workspace {
    pub fn current() -> Result<Arc<Workspace>> {
        let krates = {
            let cmd = krates::Cmd::new();
            let mut builder = krates::Builder::new();
            builder.workspace(true);

            builder.build(cmd, |_| {})
        }
        .context("Failed to build crate graph")?;

        let workspace_root = krates.workspace_root().as_std_path().to_path_buf();
        let workspace_root = &workspace_root;

        let mut ignore_builder = ignore::gitignore::GitignoreBuilder::new(workspace_root);
        ignore_builder.add(workspace_root.join(".gitignore"));
        let ignore = ignore_builder
            .build()
            .context("Failed to build ignore file")?;

        let cargo_toml = cargo_toml::Manifest::from_path(workspace_root.join("Cargo.toml"))
            .context("Failed to load Cargo.toml")?;

        let workspace = Self {
            krates,
            ignore,
            cargo_toml,
        };
        let workspace = Arc::new(workspace);

        debug!("loaded workspace at {:?}", workspace_root);

        Ok(workspace)
    }

    pub fn main_package(&self) -> Result<Kid> {
        let current_dir = std::env::current_dir().unwrap();
        let current_dir = current_dir.as_path();

        let mut closest_parent = None;
        for member in self.krates.workspace_members() {
            if let krates::Node::Krate { id, krate, .. } = member {
                let member_path = krate.manifest_path.parent().unwrap();
                if let Ok(path) = current_dir.strip_prefix(member_path.as_std_path()) {
                    let len = path.components().count();
                    match closest_parent {
                        Some((_, closest_parent_len)) => {
                            if len < closest_parent_len {
                                closest_parent = Some((id, len));
                            }
                        }
                        None => {
                            closest_parent = Some((id, len));
                        }
                    }
                }
            }
        }

        let kid = closest_parent
        .map(|(id, _)| id)
        .with_context(|| {
            let dylib_targets = self.krates.workspace_members().filter_map(|krate|match krate {
                krates::Node::Krate { krate, .. } if krate.targets.iter().any(|t| t.kind.contains(&krates::cm::TargetKind::DyLib))=> {
                    Some(format!("- {}", krate.name))
                }
                _ => None
            }).collect::<Vec<_>>();
            format!("Failed to find a dylib package to build.\nYou need to either run ranim from inside a dylib crate or specify a dylib package to build with the `--package` flag. Try building again with one of the dylib packages in the workspace:\n{}", dylib_targets.join("\n"))

        })?;

        Ok(kid.clone())
    }

    pub fn get_package(&self, package_name: &str) -> Result<Kid> {
        let mut workspace_members = self.krates.workspace_members();
        let kid = workspace_members.find_map(|node| {
            if let krates::Node::Krate { id, krate, .. } = node {
                if krate.name == package_name {
                    return Some(id);
                }
            }
            None
        });

        let Some(kid) = kid else {
            error!("Failed to find package {package_name} in the workspace.");
            let packages = self
                .krates
                .workspace_members()
                .filter_map(|package| {
                    if let krates::Node::Krate { krate, .. } = package {
                        Some(krate.name())
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>();
            error!("Available packages: {packages:?}");
            anyhow::bail!("Failed to find package {package_name} in the workspace.");
        };

        Ok(kid.clone())
    }
}

/// Get the target package.
///
/// This combines the info from args and workspace:
/// - If `--package` is specified, use that.
/// - Otherwise, use workspace's main package.
pub fn get_target_package(workspace: &Workspace, args: &Args) -> (Kid, String) {
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
    (kid, package_name)
}
