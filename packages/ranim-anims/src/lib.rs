//! Ranim's built-in animations
//!
//! This crate contains the built-in animations for Ranim, plus the two
//! author-facing specializations of the general
//! [`ranim_core::animation::Eval`] protocol:
//!
//! - **Pure** (closed-form, stateless): implement [`pure::PureEval`] — a single
//!   `eval_alpha(alpha)` method — and wrap it in [`pure::Pure`] to get a full
//!   animation segment. Closures `Fn(f64) -> T` implement `PureEval`
//!   automatically, so `Pure(|alpha| ...)` just works.
//! - **Iterative** (stateful, stepped): implement
//!   [`iterative::IterativeEval`] — a single `step(&self, &mut S, &Time, &DeltaTime)`
//!   method — and pass it with an initial state to
//!   [`iterative::Iterative::new`]. Closures `Fn(&mut S, &Time, &DeltaTime)`
//!   implement `IterativeEval<S>` automatically.
//!
//! A built-in animation is basically a struct that implements
//! [`pure::PureEval`], together with the data its closed form needs. Here is
//! the example of [`pure::fading::FadeIn`]:
//!
//! ```rust,ignore
//! pub struct FadeIn<T: FadingRequirement> {
//!     src: T,
//!     dst: T,
//! }
//!
//! impl<T: FadingRequirement> PureEval for FadeIn<T> {
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
//!     fn fade_in(&mut self) -> Pure<FadeIn<T>>;
//!     fn fade_out(&mut self) -> Pure<FadeOut<T>>;
//! }
//!
//! impl<T: FadingRequirement + 'static> FadingAnim<T> for T {
//!     fn fade_in(&mut self) -> Pure<FadeIn<T>> {
//!         Pure(FadeIn::new(self.clone())).apply_to(self)
//!     }
//!     fn fade_out(&mut self) -> Pure<FadeOut<T>> {
//!         Pure(FadeOut::new(self.clone())).apply_to(self)
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
