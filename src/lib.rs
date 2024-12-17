pub use glam;
pub mod prelude {
    pub use crate::interpolate::Interpolatable;

    pub use crate::animation::fading::Opacity;
    pub use crate::animation::transform::Alignable;

    pub use crate::rabject::{Blueprint, RabjectContainer};
}

pub mod color;
mod interpolate;
pub mod updater;

pub mod animation;
pub mod camera;
/// Rabjects are the basic objects in ranim scene
pub mod rabject;
pub mod scene;
pub mod utils;
pub mod context;
pub mod vello;
