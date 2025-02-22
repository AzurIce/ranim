use pyo3::{
    pyclass, pymethods, pymodule,
    types::{PyModule, PyModuleMethods},
    Bound, PyResult,
};
use ranim::{
    glam::vec3,
    items::{svg_item::SvgItem, vitem::VItem, Rabject},
};

#[pymodule]
pub fn items(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyVItem>()?;
    m.add_class::<PySvgItem>()?;
    Ok(())
}

// MARK: SvgItem

#[pyclass]
#[pyo3(name = "SvgItem")]
pub struct PySvgItem {
    pub(crate) inner: Rabject<SvgItem>,
}

#[pymethods]
impl PySvgItem {
    #[new]
    pub fn new(svg_str: &str) -> Self {
        Self {
            inner: Rabject::new(SvgItem::from_svg(svg_str)),
        }
    }
}

// MARK: VItem

#[pyclass]
#[pyo3(name = "VItem")]
pub struct PyVItem {
    pub(crate) inner: Rabject<VItem>,
}

#[pymethods]
impl PyVItem {
    #[new]
    pub fn new(vpoints: Vec<[f32; 3]>) -> Self {
        let vpoints = vpoints.iter().map(|v| vec3(v[0], v[1], v[2])).collect();
        Self {
            inner: Rabject::new(VItem::from_vpoints(vpoints)),
        }
    }
}
