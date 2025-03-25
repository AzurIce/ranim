pub fn linear(t: f64) -> f64 {
    t
}

/// Smooth rate function
///
/// from <https://github.com/3b1b/manim/blob/003c4d86262565bb21001f74f67e6788cae62df4/manimlib/utils/rate_functions.py#L17>
pub fn smooth(t: f64) -> f64 {
    let s = 1.0 - t;
    t * t * t * (10.0 * s * s + 5.0 * s * t + t * t)
}
