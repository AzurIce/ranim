//! Ranim's built-in animations
//!
//! This crate contains the built-in animations for Ranim, plus the two
//! author-facing specializations of the general
//! [`ranim_core::animation::Eval`] protocol:
//!
//! - **Pure** (closed-form, stateless): a struct that implements
//!   [`Eval`](ranim_core::animation::Eval) directly with a closed-form
//!   `eval_alpha(alpha)`. A raw closure can be turned into an `Eval` via
//!   [`pure::PureFunc`] (`PureFunc::new(|alpha| ...)`).
//! - **Iterative** (stateful, stepped): implement
//!   [`iterative::IterativeEval`] — an associated `Output` and a single
//!   `step(&self, &mut Self::Output, alpha, delta_alpha)` method (both
//!   dimensionless progress) — and pass it with an initial state to
//!   [`iterative::Iterative::new`], optionally declaring `with_steps(N)`.
//!   For closures, use [`iterative::Iterative::from_fn`], which binds the
//!   closure's mutable input through [`iterative::IterativeFn`].
//!
//! A built-in animation is basically a struct that implements
//! [`Eval`](ranim_core::animation::Eval), together with the data its closed form needs. Here is
//! the example of [`pure::fading::FadeIn`]:
//!
//! ```rust,ignore
//! pub struct FadeIn<T: FadingRequirement> {
//!     src: T,
//!     dst: T,
//! }
//!
//! impl<T: FadingRequirement> Eval for FadeIn<T> {
//!     type Output = T;
//!
//!     fn eval_alpha(&self, alpha: f64) -> Self::Output {
//!         self.src.lerp(&self.dst, alpha)
//!     }
//! }
//! ```
//!
//! In addition, to make the construction of anim for any type that satisfies
//! the requirement, it is recommended to write a trait like this:
//!
//! ```rust,ignore
//! /// The methods to create animations for `T` that satisfies [`FadingRequirement`]
//! pub trait FadingAnim<T: FadingRequirement + 'static> {
//!     fn fade_in(&mut self) -> FadeIn<T>;
//!     fn fade_out(&mut self) -> FadeOut<T>;
//! }
//!
//! impl<T: FadingRequirement + 'static> FadingAnim<T> for T {
//!     fn fade_in(&mut self) -> FadeIn<T> {
//!         FadeIn::new(self.clone()).apply_to(self)
//!     }
//!     fn fade_out(&mut self) -> FadeOut<T> {
//!         FadeOut::new(self.clone()).apply_to(self)
//!     }
//! }
//! ```
#![warn(missing_docs)]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![allow(rustdoc::private_intra_doc_links)]
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/AzurIce/ranim/refs/heads/main/assets/ranim.svg",
    html_favicon_url = "https://raw.githubusercontent.com/AzurIce/ranim/refs/heads/main/assets/ranim.svg"
)]

/// Iterative (stateful) evaluation
pub mod iterative;
/// Pure (closed-form) evaluation
pub mod pure;
