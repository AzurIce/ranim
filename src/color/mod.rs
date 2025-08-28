pub use ::color::*;

/// palettes
pub mod palettes;

/// Color preludes
pub mod prelude {
    pub use super::{rgb, rgb8, rgba, rgba8};
    pub use crate::color;
}

/// Construct an [`AlphaColor<Srgb>`] from rgb u8, the alpha value will be 255
pub const fn rgb8(r: u8, g: u8, b: u8) -> AlphaColor<Srgb> {
    OpaqueColor::from_rgb8(r, g, b).with_alpha(1.0)
}

/// Construct an [`AlphaColor<Srgb>`] from rgba u8
pub const fn rgba8(r: u8, g: u8, b: u8, a: u8) -> AlphaColor<Srgb> {
    AlphaColor::from_rgba8(r, g, b, a)
}

/// Construct an [`AlphaColor<Srgb>`] from rgb f32, the alpha value will be 1.0
pub const fn rgb(r: f32, g: f32, b: f32) -> AlphaColor<Srgb> {
    OpaqueColor::new([r, g, b]).with_alpha(1.0)
}

/// Construct an [`AlphaColor<Srgb>`] from rgba f32
pub const fn rgba(r: f32, g: f32, b: f32, a: f32) -> AlphaColor<Srgb> {
    AlphaColor::new([r, g, b, a])
}

/// A macro to parse color string to [`AlphaColor<Srgb>`]
///
/// In its inner it uses [`color::parse_color`]
#[macro_export]
macro_rules! color {
    ($color_str:expr) => {{
        use ::ranim::color::{Srgb, parse_color};
        parse_color($color_str)
            .expect("Invalid color string")
            .to_alpha_color::<Srgb>()
    }};
}
