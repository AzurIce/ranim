//! Experimental embedded-Python bindings for Ranim.
//!
//! The interpreter is embedded in the `ranim` binary (see `ranim-cli`'s
//! `python` feature); users write scenes as Python scripts which `import
//! ranimpy`. This crate is the `ranimpy` module they see, plus the glue the
//! CLI uses to load scripts and render the scenes they register.

pub mod anims;
pub mod items;
pub mod scene;

use pyo3::prelude::*;

pub use scene::{PyScene, load_script};

/// Initialize the embedded interpreter and register the `ranimpy` module.
///
/// Must be called once before [`load_script`]. Safe to call more than once
/// (the inittab entry is appended again but CPython ignores duplicates by
/// name; interpreter initialization is idempotent).
pub fn init_python() {
    pyo3::append_to_inittab!(ranimpy);
    pyo3::prepare_freethreaded_python();
}

/// Python glue defining the user-facing `@scene` decorator and the scene
/// registry. Executed in the `ranimpy` module namespace at module init.
const SCENE_GLUE: &std::ffi::CStr = c"
_scenes = []

def scene(_fn=None, *, name=None, output_dir=None, clear_color=None):
    def wrap(fn):
        _scenes.append((name or fn.__name__, output_dir, clear_color, fn))
        return fn
    return wrap(_fn) if _fn is not None else wrap
";

/// The `ranimpy` Python module.
#[pymodule]
pub fn ranimpy(py: Python, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<items::PyColor>()?;
    m.add_class::<items::PyVItem>()?;
    m.add_class::<items::PySquare>()?;
    m.add_class::<items::PyCircle>()?;
    m.add_class::<items::PySvgItem>()?;
    m.add_class::<items::PyCameraFrame>()?;
    m.add_class::<anims::PyAnim>()?;
    m.add_class::<anims::PyRateFunc>()?;
    m.add_class::<scene::PyRanimScene>()?;

    m.add_function(wrap_pyfunction!(anims::sequence, m)?)?;
    m.add_function(wrap_pyfunction!(anims::stack, m)?)?;

    m.add_submodule(&anims::rate_functions_module(py)?)?;
    m.add_submodule(&items::palettes_module(py)?)?;

    py.run(SCENE_GLUE, Some(&m.dict()), None)?;
    Ok(())
}
