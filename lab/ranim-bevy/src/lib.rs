//! Experimental Bevy integration for Ranim.
//!
//! This crate keeps Bevy as an optional host. Ranim data remains plain
//! `ranim-core` data, while Bevy owns extraction, visibility, render phases,
//! and drawing.

mod component;
mod plugin;
mod render;
mod shader;
mod utils;
#[cfg(all(feature = "video", not(target_family = "wasm")))]
pub mod video;

pub use component::RanimVItem;
pub use plugin::RanimBevyPlugin;
pub use utils::collect_vitems_into_store;
