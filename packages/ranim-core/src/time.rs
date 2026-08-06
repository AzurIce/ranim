//! Time vocabulary for animation evaluation: points and spans are different algebra.
//!
//! `Time` is a **point** (an absolute reading on a clock); `DeltaTime` is a
//! **span** (the length of one logic step). They are separate types so that
//! contexts that must not see deltas — sampling, seeking — cannot carry them at
//! the type level: [`Eval::sample`](crate::animation::Eval::sample) takes only
//! `&Time`, while [`Eval::step`](crate::animation::Eval::step) takes both.
//!
//! These are logical animation clocks, not wall-clock measurements, so they are
//! plain `f64` values rather than `std::time` types: `std::time::Instant`
//! cannot represent a logical scene position at all (it can only be derived
//! from `Instant::now()`), `std::time::Duration` is unsigned (a non-monotonic
//! rate function can produce a negative `Δalpha`), and `alpha` is a
//! dimensionless progress, not a time span.
//!
//! Boundary rule: animation logic receives **time readings** (the values here),
//! never **time configuration**. The segment's start, duration and rate
//! function belong to the owning `AnimationCell`, which turns them into
//! `alpha`/`Δalpha` before calling into the evaluator. `with_duration` and
//! `with_rate_func` are therefore pure playback transforms for every segment
//! kind; how much an iterative segment simulates is its own construction
//! parameter, scaled internally from `DeltaTime::alpha`.
//!
//! `global_secs` is the one escape hatch for segments that need unwarped,
//! wall-clock-shaped time: it is threaded down from the scene clock unchanged
//! through any depth of nested containers.

/// A point in time: an absolute reading, free of any delta information.
///
/// `alpha` is the rate-warped progress `r((t-s)/D)` of the current segment,
/// computed by the owning `AnimationCell` (a zero-duration cell reports
/// `alpha == 1.0`). `global_secs` is the true scene-global time, inherited
/// unchanged from the driving session at any nesting depth.
#[derive(Debug, Clone, Copy, Default)]
pub struct Time {
    /// Rate-warped progress of the current segment in `[0, 1]`.
    pub alpha: f64,
    /// True scene-global time at the current logic tick (seconds).
    pub global_secs: f64,
}

/// A span of time: the length of one logic step.
///
/// Deltas exist only here. `alpha` is the integration step in the segment's
/// warped local progress — it varies per tick under a non-linear rate
/// function, and iterative segments should be written for variable-step
/// integration. `global_secs` is the true global logic step, stable at
/// `1 / logic_fps`, for segments that need unwarped physical time.
#[derive(Debug, Clone, Copy, Default)]
pub struct DeltaTime {
    /// Step length in the segment's warped local progress (dimensionless).
    pub alpha: f64,
    /// True global logic step length (seconds), stable at `1 / logic_fps`.
    pub global_secs: f64,
}

/// The unwarped global time channel, passed down unchanged through stepping.
///
/// Scene ticks originate in [`SceneEvaluator`](crate::SceneEvaluator); each
/// container forwards this value to its children untouched while it remaps
/// the local coordinates, so `Time::global_secs` and
/// `DeltaTime::global_secs` stay honest at any nesting depth.
#[derive(Debug, Clone, Copy)]
pub(crate) struct GlobalTime {
    /// Global scene time at the current tick (seconds).
    pub secs: f64,
    /// Global logic step length (seconds), `1 / logic_fps`.
    pub delta_secs: f64,
}

impl GlobalTime {
    /// Rebuild the global channel from the leaf-facing time pair.
    pub(crate) fn of(time: &Time, delta_time: &DeltaTime) -> Self {
        Self {
            secs: time.global_secs,
            delta_secs: delta_time.global_secs,
        }
    }
}
