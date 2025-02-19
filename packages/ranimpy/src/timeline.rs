use pyo3::{pyclass, pymethods, types::PyAnyMethods, Bound, PyAny};
use ranim::animation::timeline::Timeline;

use crate::items::{PySvgItem, PyVItem};

#[pyclass]
#[pyo3(name = "Timeline")]
#[derive(Debug)]
pub struct PyTimeline {
    pub(crate) inner: Timeline,
}

#[pymethods]
impl PyTimeline {
    #[new]
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            inner: Timeline::new(),
        }
    }
    pub fn show(&mut self, rabject: &Bound<'_, PyAny>) {
        if let Ok(vitem) = rabject.downcast::<PyVItem>() {
            self.inner.show(&vitem.borrow().inner);
        } else if let Ok(svg_item) = rabject.downcast::<PySvgItem>() {
            self.inner.show(&svg_item.borrow().inner);
        }
    }
    pub fn forward(&mut self, secs: f32) {
        self.inner.forward(secs);
    }
    pub fn elapsed_secs(&self) -> f32 {
        self.inner.elapsed_secs()
    }
}
