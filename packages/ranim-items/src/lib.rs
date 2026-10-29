//! Ranim's built-in items
#![recursion_limit = "256"]
#![warn(missing_docs)]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![allow(rustdoc::private_intra_doc_links)]
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/AzurIce/ranim/refs/heads/main/assets/ranim.svg",
    html_favicon_url = "https://raw.githubusercontent.com/AzurIce/ranim/refs/heads/main/assets/ranim.svg"
)]

pub mod camera;
pub mod debug;
/// Hierarchical scene-graph composition for items.
pub mod hierarchy;
pub mod mesh;
pub mod vitem;

#[cfg(feature = "render")]
pub use {camera::CameraFramePlugin, mesh::MeshItemPlugin, vitem::VItemPlugin};

/// Installs the built-in render plugins: camera frame, vector items, and mesh
/// items. This is what the default render setup of a project binary uses;
/// install it explicitly when composing a custom
/// [`RenderSetup`](ranim_render::RenderSetup) with additional extraction
/// plugins, and do not combine it with the individual plugins (they are
/// unique in a bevy app).
#[cfg(feature = "render")]
pub struct RanimItemsPlugin;

#[cfg(feature = "render")]
impl bevy_app::Plugin for RanimItemsPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.add_plugins((CameraFramePlugin, VItemPlugin, MeshItemPlugin));
    }
}
