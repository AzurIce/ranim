//! Animation bindings: a type-erased [`PyAnim`] and composition functions.
//!
//! A [`PyAnim`] always wraps an [`AnimSequence`]; single animations are
//! wrapped into a one-element sequence, and modifiers re-wrap the sequence.
//! This keeps one uniform Python type while preserving the Rust semantics
//! (nested sequences evaluate transparently).

use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;
use pyo3::types::PyModule;

use ranim::core::animation::{AnimSequence, AnimStack, AnimationExt, Placeable};
use ranim::core::utils::rate_functions as rf;

/// A `fn(f64) -> f64` rate function exposed to Python.
#[pyclass(name = "RateFunc", frozen)]
#[derive(Clone, Copy)]
pub struct PyRateFunc(pub(crate) fn(f64) -> f64);

/// The `ranimpy.rate_functions` submodule.
pub fn rate_functions_module(py: Python<'_>) -> PyResult<Bound<'_, PyModule>> {
    let m = PyModule::new(py, "rate_functions")?;
    m.add("linear", PyRateFunc(rf::linear))?;
    m.add("smooth", PyRateFunc(rf::smooth))?;
    m.add("ease_in_quad", PyRateFunc(rf::ease_in_quad))?;
    m.add("ease_out_quad", PyRateFunc(rf::ease_out_quad))?;
    m.add("ease_in_out_quad", PyRateFunc(rf::ease_in_out_quad))?;
    m.add("ease_in_cubic", PyRateFunc(rf::ease_in_cubic))?;
    m.add("ease_out_cubic", PyRateFunc(rf::ease_out_cubic))?;
    m.add("ease_in_out_cubic", PyRateFunc(rf::ease_in_out_cubic))?;
    Ok(m)
}

/// A type-erased animation.
///
/// Consumed by `RanimScene.play()`; playing the same `Anim` twice is an
/// error (create it again from the item instead).
///
/// `unsendable` because the erased evaluators are not `Sync`.
#[pyclass(name = "Anim", unsendable)]
pub struct PyAnim {
    pub(crate) inner: Option<AnimSequence>,
}

impl PyAnim {
    pub(crate) fn new(seq: AnimSequence) -> Self {
        Self { inner: Some(seq) }
    }

    /// Wrap any placeable animation into a one-element sequence.
    pub(crate) fn from_anim<A: Placeable + 'static>(anim: A) -> Self {
        let mut seq = AnimSequence::new();
        seq.push(anim);
        Self::new(seq)
    }

    /// Wrap `inner` into a fresh sequence, applying `f` to it first.
    fn rewrap<A: Placeable + 'static>(&mut self, f: impl FnOnce(AnimSequence) -> A) {
        let inner = self.inner.take().expect("anim consumed while borrowed");
        self.inner = Some(Self::from_anim(f(inner)).inner.unwrap());
    }

    /// Take the inner sequence out (consuming the animation).
    pub(crate) fn take(&mut self) -> PyResult<AnimSequence> {
        self.inner.take().ok_or_else(|| {
            PyRuntimeError::new_err("this Anim was already consumed by play()")
        })
    }
}

#[pymethods]
impl PyAnim {
    /// Override the animation's duration in seconds.
    fn with_duration<'py>(
        mut slf: PyRefMut<'py, Self>,
        secs: f64,
    ) -> PyResult<PyRefMut<'py, Self>> {
        slf.rewrap(|inner| inner.with_duration(secs));
        Ok(slf)
    }

    /// Override the animation's rate function.
    fn with_rate_func<'py>(
        mut slf: PyRefMut<'py, Self>,
        rate_func: PyRateFunc,
    ) -> PyResult<PyRefMut<'py, Self>> {
        slf.rewrap(|inner| inner.with_rate_func(rate_func.0));
        Ok(slf)
    }

    /// Shift the animation to start at `offset_sec` (meaningful inside `stack`).
    ///
    /// `At` is a terminal placement (it is not `Placeable`), so the placed
    /// animation is wrapped into an `AnimStack` to keep it composable.
    fn at<'py>(mut slf: PyRefMut<'py, Self>, offset_sec: f64) -> PyResult<PyRefMut<'py, Self>> {
        let inner = slf.take()?;
        let mut stack = AnimStack::new();
        stack.push(inner.at(offset_sec));
        slf.inner = Some(Self::from_anim(stack).inner.unwrap());
        Ok(slf)
    }

    /// Append another animation to play after this one (sequence semantics).
    fn push<'py>(
        mut slf: PyRefMut<'py, Self>,
        other: &Bound<'_, PyAnim>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let other = other.borrow_mut().take()?;
        slf.inner.as_mut().unwrap().push(other);
        Ok(slf)
    }

    /// Advance the cursor while holding the state immediately before it.
    fn hold<'py>(mut slf: PyRefMut<'py, Self>, secs: f64) -> PyResult<PyRefMut<'py, Self>> {
        slf.inner.as_mut().unwrap().hold(secs);
        Ok(slf)
    }

    /// Advance the cursor without adding an animation.
    fn forward<'py>(mut slf: PyRefMut<'py, Self>, secs: f64) -> PyResult<PyRefMut<'py, Self>> {
        slf.inner.as_mut().unwrap().forward(secs);
        Ok(slf)
    }

    /// The total duration in seconds.
    #[getter]
    fn duration(&self) -> f64 {
        self.inner.as_ref().map(|s| s.cursor_sec()).unwrap_or(0.0)
    }
}

/// Compose animations to play one after another.
#[pyfunction]
pub fn sequence(py: Python<'_>, anims: Vec<Py<PyAnim>>) -> PyResult<PyAnim> {
    let mut seq = AnimSequence::new();
    for anim in anims {
        let inner = anim.borrow_mut(py).take()?;
        seq.push(inner);
    }
    Ok(PyAnim::new(seq))
}

/// Compose animations to play together (overlay semantics).
#[pyfunction]
pub fn stack(py: Python<'_>, anims: Vec<Py<PyAnim>>) -> PyResult<PyAnim> {
    let mut stack = AnimStack::new();
    for anim in anims {
        let inner = anim.borrow_mut(py).take()?;
        stack.push(inner);
    }
    Ok(PyAnim::from_anim(stack))
}
