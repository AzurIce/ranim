//! Ranim's built-in animation families.
//!
//! Each module contains named, closed-form animation types plus the
//! convenience traits used to construct them from an item (e.g.
//! [`fading::FadingAnim`]). Every animation type implements
//! [`Eval`](ranim_core::animation::eval::Eval) directly; generic authoring adapters
//! live in `ranim_core::animation`:
//!
//! - [`Pure`](ranim_core::animation::eval::pure::Pure) wraps a raw
//!   `Fn(f64) -> T` closure into an `Eval`;
//! - [`Iterative`](ranim_core::animation::eval::iterative::Iterative) turns an
//!   [`IterativeEval`](ranim_core::animation::eval::iterative::IterativeEval) step function
//!   into a stateful `Eval`.
//!
//! A built-in animation is a struct that implements `Eval` together with the
//! data its closed form needs. For example, [`fading::FadeIn`]:
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
//! Construction traits mutate the source item to its end state while building
//! the animation:
//!
//! ```rust,ignore
//! pub trait FadingAnim: FadingRequirement + Sized + 'static {
//!     fn fade_in(&mut self) -> FadeIn<Self>;
//!     fn fade_out(&mut self) -> FadeOut<Self>;
//! }
//!
//! impl<T: FadingRequirement + Sized + 'static> FadingAnim for T {
//!     fn fade_in(&mut self) -> FadeIn<Self> {
//!         FadeIn::new(self.clone()).apply_to(self)
//!     }
//!
//!     fn fade_out(&mut self) -> FadeOut<Self> {
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

/// Camera frame animations.
pub mod camera;
/// Creation animations.
pub mod creation;
/// Fading animations.
pub mod fading;
/// Morph animations.
pub mod morph;
/// Rotating animations.
pub mod rotating;
