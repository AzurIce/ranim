pub use glam;
pub mod prelude {
    pub use crate::interpolate::Interpolatable;

    pub use crate::animation::fading::Opacity;
    pub use crate::animation::transform::Alignable;

    pub use crate::rabject::Blueprint;
}

pub mod color;
mod interpolate;
pub mod updater;

pub mod animation;
pub mod camera;
pub mod canvas;
pub mod context;
/// Rabjects are the basic objects in ranim scene
pub mod rabject;
pub mod scene;
pub mod utils;
