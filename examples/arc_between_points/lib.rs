mod scenes;

pub use scenes::ScenesPlugin;

// Keep the exported factory here: the bundle reuses `scenes.rs` without
// generating multiple native `ranim_plugin` entry symbols.
#[ranim::export_plugin]
pub fn plugin() -> impl ranim::Plugin {
    ScenesPlugin
}
