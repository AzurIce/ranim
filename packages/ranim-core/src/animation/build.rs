//! Playback configuration and placement of retained animation definitions.
//!
//! These wrappers convert into [`AnimDefNode`] using `Into`, retaining the input
//! and recipe. An explicit duration and placement become a definition window;
//! placement temporarily builds a node if its natural window is needed.
//!
//! ```
//! use ranim_core::animation::{def_node::AnimDefNode, build::{PlaybackExt, Unplaced}};
//!
//! let definition: AnimDefNode = AnimDefNode::children(Vec::new())
//!     .with_duration(2.0)
//!     .with_enabled(false)
//!     .at(3.0)
//!     .into();
//! assert_eq!(definition.time_range, Some(3.0..5.0));
//! assert_eq!(definition.enabled, Some(false));
//! assert_eq!(definition.build().time_range(), 3.0..5.0);
//! ```

use super::def_node::{AnimDef, AnimDefNode, AnimDefRate};
use crate::animation::AnimRecipe;

/// A definition whose placement has not been fixed by an [`At`] wrapper.
///
/// Parameters are configured before placement. [`At`] is terminal and does not
/// implement this trait:
///
/// ```compile_fail
/// use ranim_core::animation::{def_node::AnimDefNode, build::{PlaybackExt, Unplaced}};
/// let _ = AnimDefNode::children(Vec::new()).at(1.0).with_duration(2.0);
/// ```
pub trait Unplaced: Into<AnimDefNode> {
    /// Shift the definition in its parent's time coordinates.
    ///
    /// # Panics
    ///
    /// Panics if the offset is not finite.
    fn at(self, offset_sec: f64) -> At<Self> {
        assert!(offset_sec.is_finite(), "definition offset must be finite");
        At {
            inner: self,
            offset_sec,
        }
    }
}

impl<T: 'static, R: AnimRecipe<T> + 'static> Unplaced for AnimDef<T, R> {}
impl Unplaced for AnimDefNode {}
impl<A: Unplaced> Unplaced for Paramed<A> {}

/// Playback builders for unplaced definitions. They never construct evaluators.
pub trait PlaybackExt: Unplaced {
    /// Override the definition's parent-time duration.
    fn with_duration(self, secs: f64) -> Paramed<Self> {
        Paramed::new(self).with_duration(secs)
    }

    /// Override the definition's progress mapping.
    fn with_rate_func(self, rate: fn(f64) -> f64) -> Paramed<Self> {
        Paramed::new(self).with_rate_func(rate)
    }

    /// Explicitly replace the recipe's rate with structural linear mapping.
    fn with_linear_rate(self) -> Paramed<Self> {
        Paramed::new(self).with_linear_rate()
    }

    /// Override whether the definition contributes output.
    fn with_enabled(self, enabled: bool) -> Paramed<Self> {
        Paramed::new(self).with_enabled(enabled)
    }
}
impl<A: Unplaced> PlaybackExt for A {}

/// A typed definition with explicit playback overrides, before type erasure.
pub struct Paramed<A> {
    inner: A,
    duration_secs: Option<f64>,
    rate_func: Option<AnimDefRate>,
    enabled: Option<bool>,
}

impl<A> Paramed<A> {
    fn new(inner: A) -> Self {
        Self {
            inner,
            duration_secs: None,
            rate_func: None,
            enabled: None,
        }
    }

    /// Borrow the original typed definition.
    pub fn inner(&self) -> &A {
        &self.inner
    }

    /// Edit the original input or recipe before converting or building.
    pub fn inner_mut(&mut self) -> &mut A {
        &mut self.inner
    }

    /// Override duration without resetting the other playback defaults.
    ///
    /// # Panics
    ///
    /// Panics if the duration is negative or non-finite.
    pub fn with_duration(mut self, secs: f64) -> Self {
        assert!(
            secs.is_finite() && secs >= 0.0,
            "definition duration must be finite and non-negative"
        );
        self.duration_secs = Some(secs);
        self
    }

    /// Override the content progress mapping.
    pub fn with_rate_func(mut self, rate: fn(f64) -> f64) -> Self {
        self.rate_func = Some(AnimDefRate::Function(rate));
        self
    }

    /// Explicitly use structural linear mapping, clearing a recipe's custom rate.
    pub fn with_linear_rate(mut self) -> Self {
        self.rate_func = Some(AnimDefRate::Linear);
        self
    }

    /// Override whether the definition contributes output.
    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = Some(enabled);
        self
    }
}

impl<A: Unplaced> From<Paramed<A>> for AnimDefNode {
    fn from(configured: Paramed<A>) -> Self {
        let mut node = configured.inner.into();
        if let Some(duration) = configured.duration_secs {
            let start = node.time_range.as_ref().map_or(0.0, |window| window.start);
            node.time_range = Some(start..start + duration);
        }
        if let Some(rate) = configured.rate_func {
            node.rate_func = Some(rate);
        }
        if let Some(enabled) = configured.enabled {
            node.enabled = Some(enabled);
        }
        node
    }
}

/// A definition with a fixed parent-time offset, terminal in the builder chain.
pub struct At<A> {
    inner: A,
    offset_sec: f64,
}

impl<A> At<A> {
    /// Borrow the definition before placement.
    pub fn inner(&self) -> &A {
        &self.inner
    }

    /// Edit the definition before placement without constructing runtime state.
    pub fn inner_mut(&mut self) -> &mut A {
        &mut self.inner
    }

    /// The offset to apply to the definition's local window.
    pub fn offset_sec(&self) -> f64 {
        self.offset_sec
    }
}

impl<A: Into<AnimDefNode>> From<At<A>> for AnimDefNode {
    fn from(placed: At<A>) -> Self {
        let mut node = placed.inner.into();
        node.shift_by(placed.offset_sec);
        node
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::animation::{eval::Static, node::AnimNode};
    use crate::utils::rate_functions::smooth;
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    struct Recipe {
        duration: f64,
        builds: Arc<AtomicUsize>,
    }
    impl AnimRecipe<f64> for Recipe {
        fn build(&self, initial: &f64) -> AnimNode {
            self.builds.fetch_add(1, Ordering::Relaxed);
            let mut node = AnimNode::from_eval(Static(*initial), self.duration);
            node.rate_func = Some(smooth);
            node.enabled = false;
            node.anim_name = "recipe";
            node
        }
    }
    fn definition(builds: &Arc<AtomicUsize>) -> AnimDef<f64, Recipe> {
        AnimDef::new(
            2.0,
            Recipe {
                duration: 7.0,
                builds: Arc::clone(builds),
            },
        )
    }

    #[test]
    fn configuration_preserves_definitions_and_resolves_natural_windows() {
        let builds = Arc::new(AtomicUsize::new(0));
        let node: AnimDefNode = definition(&builds)
            .with_duration(2.0)
            .with_enabled(true)
            .at(3.0)
            .into();
        assert_eq!(node.time_range, Some(3.0..5.0));
        assert_eq!(node.enabled, Some(true));
        assert_eq!(builds.load(Ordering::Relaxed), 0);

        // Placement resolves a natural window by temporarily building the node.
        let pending: AnimDefNode = definition(&builds).at(3.0).into();
        assert_eq!(pending.time_range, Some(3.0..10.0));
        let node: AnimDefNode = pending.with_duration(2.0).into();
        assert_eq!(node.time_range, Some(3.0..5.0));
        assert_eq!(node.build().time_range(), 3.0..5.0);

        // Without an explicit duration the recipe's own window applies.
        let natural: AnimDefNode = definition(&builds).at(3.0).into();
        let runtime = natural.build();
        assert_eq!(runtime.time_range(), 3.0..10.0);
        assert!(!runtime.enabled());
        assert_eq!(runtime.rate_func.unwrap()(0.25), smooth(0.25));
        assert_eq!(builds.load(Ordering::Relaxed), 4);
    }

    #[test]
    fn placement_shifts_an_existing_recipe_window_instead_of_replacing_its_start() {
        struct OffsetRecipe;
        impl AnimRecipe<()> for OffsetRecipe {
            fn build(&self, _: &()) -> AnimNode {
                let mut node = AnimNode::from_eval(Static(1.0), 7.0);
                node.time_range = 2.0..9.0;
                node
            }
        }
        let node: AnimDefNode = AnimDef::new((), OffsetRecipe).at(-3.0).into();
        assert_eq!(node.build().time_range(), -1.0..6.0);
    }

    #[test]
    fn nested_parameters_only_override_requested_fields() {
        let builds = Arc::new(AtomicUsize::new(0));
        let inner = definition(&builds).with_enabled(true).with_linear_rate();
        let node: AnimDefNode = PlaybackExt::with_duration(inner, 6.0).into();
        assert_eq!(node.time_range, Some(0.0..6.0));
        assert_eq!(node.enabled, Some(true));
        assert!(matches!(node.rate_func, Some(AnimDefRate::Linear)));
        assert_eq!(builds.load(Ordering::Relaxed), 0);

        let runtime = node.build();
        assert!(runtime.enabled());
        assert!(runtime.rate_func.is_none());
        assert_eq!(runtime.duration_secs(), 6.0);
    }

    #[test]
    fn invalid_configuration_is_rejected_before_building_runtime_state() {
        let builds = Arc::new(AtomicUsize::new(0));
        for duration in [-1.0, f64::NAN, f64::INFINITY] {
            assert!(
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    definition(&builds).with_duration(duration)
                }))
                .is_err()
            );
        }
        for offset in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            assert!(
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    definition(&builds).at(offset)
                }))
                .is_err()
            );
        }
        assert!(
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let _: AnimDefNode = definition(&builds)
                    .with_duration(f64::MAX)
                    .at(f64::MAX)
                    .into();
            }))
            .is_err()
        );
        assert_eq!(builds.load(Ordering::Relaxed), 0);
    }
}
