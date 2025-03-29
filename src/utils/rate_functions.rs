#[inline]
pub fn linear(t: f64) -> f64 {
    t
}

/// Smooth rate function
///
/// from <https://github.com/3b1b/manim/blob/003c4d86262565bb21001f74f67e6788cae62df4/manimlib/utils/rate_functions.py#L17>
#[inline]
pub fn smooth(t: f64) -> f64 {
    let s = 1.0 - t;
    t * t * t * (10.0 * s * s + 5.0 * s * t + t * t)
}

#[inline]
pub fn ease_in_quad(t: f64) -> f64 {
    t * t
}

#[inline]
pub fn ease_out_quad(t: f64) -> f64 {
    t * (2.0 - t)
}

#[inline]
pub fn ease_in_out_quad(t: f64) -> f64 {
    if t < 0.5 {
        2.0 * t * t
    } else {
        1.0 - 2.0 * (t - 1.0) * (t - 1.0)
    }
}

#[inline]
pub fn ease_in_cubic(t: f64) -> f64 {
    t * t * t
}

#[inline]
pub fn ease_out_cubic(t: f64) -> f64 {
    t * (t - 1.0) * (t - 1.0) + 1.0
}

#[inline]
pub fn ease_in_out_cubic(t: f64) -> f64 {
    if t < 0.5 {
        4.0 * t * t * t
    } else {
        1.0 - 4.0 * (t - 1.0) * (t - 1.0) * (t - 1.0)
    }
}
