//! Ranim's built-in animations
//!
//! This crate contains the built-in animations for Ranim.
//!
//! An **Animation** in ranim is basically a struct that implements the [`ranim_core::animation::EvalDynamic`] trait:
//!
//! ```rust,ignore
//! pub trait EvalDynamic<T> {
//!     /// Evaluates at the given progress value `alpha` in range [0, 1].
//!     fn eval_alpha(&self, alpha: f64) -> T;
//! }
//! ```
//!
//! Every animation self-contains the evaluation process (the trait impl of [`ranim_core::animation::EvalDynamic::eval_alpha`])
//! and the data that the evaluation process needs (the struct it self). Here is the example of [`fading::FadeIn`] animation:
//!
//! ```rust,ignore
//! pub trait FadingRequirement: Opacity + Interpolatable + Clone {}
//! impl<T: Opacity + Interpolatable + Clone> FadingRequirement for T {}
//!
//! pub struct FadeIn<T: FadingRequirement> {
//!     src: T,
//!     dst: T,
//! }
//!
//! impl<T: FadingRequirement> FadeIn<T> {
//!     pub fn new(target: T) -> Self {
//!         let mut src = target.clone();
//!         let dst = target.clone();
//!         src.set_opacity(0.0);
//!         Self { src, dst }
//!     }
//! }
//!
//! impl<T: FadingRequirement> EvalDynamic<T> for FadeIn<T> {
//!     fn eval_alpha(&self, alpha: f64) -> T {
//!         self.src.lerp(&self.dst, alpha)
//!     }
//! }
//! ```
//!
//! In addition, to make the construction of anim for any type that satisfies the requirement,
//! It is recommended to write a trait like this:
//!
//! ```rust,ignore
//! /// The methods to create animations for `T` that satisfies [`FadingRequirement`]
//! pub trait FadingAnim<T: FadingRequirement + 'static> {
//!     fn fade_in(self) -> AnimationSpan<T>;
//!     fn fade_out(self) -> AnimationSpan<T>;
//! }
//!
//! impl<T: FadingRequirement + 'static> FadingAnim<T> for T {
//!     fn fade_in(self) -> AnimationSpan<T> {
//!         FadeIn::new(self.clone())
//!             .into_animation_span()
//!             .with_rate_func(smooth)
//!     }
//!     fn fade_out(self) -> AnimationSpan<T> {
//!         FadeOut::new(self.clone())
//!             .into_animation_span()
//!             .with_rate_func(smooth)
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

/// Creation animation
pub mod creation;
/// Fading animation
pub mod fading;
/// Func animation
pub mod func;
/// Lagged animation
pub mod lagged;
/// Transform animation
pub mod transform;
