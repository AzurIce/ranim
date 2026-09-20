//! Ranim's built-in items
#![warn(missing_docs)]
#![cfg_attr(docsrs, feature(doc_cfg))]
// Arena extraction uses std `allocator_api`; the workspace targets the
// pinned nightly.
#![feature(allocator_api)]
#![allow(rustdoc::private_intra_doc_links)]
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/AzurIce/ranim/refs/heads/main/assets/ranim.svg",
    html_favicon_url = "https://raw.githubusercontent.com/AzurIce/ranim/refs/heads/main/assets/ranim.svg"
)]

pub mod debug;
/// Hierarchical scene-graph composition for items.
pub mod hierarchy;
pub mod mesh;
pub mod vitem;
