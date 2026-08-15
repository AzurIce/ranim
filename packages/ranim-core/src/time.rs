//! Time vocabulary for animation evaluation.
//!
//! Animation evaluation converged on a single progress coordinate: the units
//! of evaluation are **dimensionless progress** (`alpha ∈ [0, 1]`), not
//! seconds. There is no scene clock channel anywhere in the evaluation path:
//! an animation's *content* is a pure function of its own progress axis
//! ("content is sequence"), and anything that needs real time is authored at
//! the top level where the author knows the placement.
//!
//! Because progress is the only coordinate, there are no `Time`/`DeltaTime`
//! point/span structs anymore — their `secs` payloads only ever made sense at
//! the `AnimationCell` level (start/duration/rate live there), and stripping
//! them keeps the evaluation protocol free of cell time configuration.
//!
//! Evaluators receive a plain `f64` progress (`alpha`) and a plain `f64`
//! progress step (`delta_alpha`), so this module is nothing but the
//! documentation of that coordinate.

/// The sole evaluation coordinate: normalized progress in `[0, 1]`.
///
/// Lives as a bare `f64` in signatures (`Eval::eval_alpha`).
pub type Alpha = f64;

/// A uniform progress step (`1 / N` for a content declared as an N-step
/// sequence).
pub type DeltaAlpha = f64;
