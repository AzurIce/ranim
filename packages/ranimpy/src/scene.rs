//! Scene bindings: the Python-facing `RanimScene` and the script loader the
//! CLI uses to discover scenes registered with `@ranimpy.scene`.

use std::ffi::CString;
use std::path::Path;

use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;
use pyo3::types::PyList;

use ranim::core::TimeMark;
use ranim::{RanimScene, SceneConstructor};

use crate::anims::PyAnim;

/// The Python-facing scene handle passed to scene functions.
///
/// `unsendable` because `RanimScene` contains `Box<dyn EvalDyn>` which is
/// not `Sync`; it is only used on the GIL-holding thread anyway.
#[pyclass(name = "RanimScene", unsendable)]
pub struct PyRanimScene {
    pub(crate) inner: Option<RanimScene>,
}

impl PyRanimScene {
    fn inner_mut(&mut self) -> PyResult<&mut RanimScene> {
        self.inner
            .as_mut()
            .ok_or_else(|| PyRuntimeError::new_err("scene already built"))
    }
}

#[pymethods]
impl PyRanimScene {
    /// Play an animation (consuming it).
    fn play(&mut self, anim: &Bound<'_, PyAnim>) -> PyResult<()> {
        let anim = anim.borrow_mut().take()?;
        self.inner_mut()?.play(anim);
        Ok(())
    }

    /// Capture a frame to an image file at `sec`.
    fn insert_time_mark_capture(&mut self, sec: f64, path: &str) -> PyResult<()> {
        self.inner_mut()?
            .insert_time_mark(sec, TimeMark::Capture(path.to_string()));
        Ok(())
    }
}

/// A scene discovered in a Python script.
pub struct PyScene {
    /// Scene name (from `@scene(name=...)` or the function name).
    pub name: String,
    /// Output directory (from `@scene(output_dir=...)`).
    pub output_dir: Option<String>,
    /// Clear color (from `@scene(clear_color=...)`).
    pub clear_color: Option<String>,
    func: Py<PyAny>,
}

impl SceneConstructor for PyScene {
    fn construct(&self, r: &mut RanimScene) {
        Python::with_gil(|py| -> PyResult<()> {
            let scene = Py::new(
                py,
                PyRanimScene {
                    inner: Some(RanimScene::new()),
                },
            )?;
            self.func.call1(py, (scene.clone_ref(py),))?;
            let inner = scene
                .borrow_mut(py)
                .inner
                .take()
                .expect("scene already built");
            *r = inner;
            Ok(())
        })
        .expect("python scene constructor failed");
    }
}

/// Execute a Python script and collect the scenes it registered with
/// `@ranimpy.scene`.
///
/// The interpreter must already be initialized (see [`crate::init_python`]).
pub fn load_script(path: &Path) -> PyResult<Vec<PyScene>> {
    let code = std::fs::read_to_string(path)
        .map_err(|e| PyRuntimeError::new_err(format!("failed to read {}: {e}", path.display())))?;
    let code = CString::new(code)?;
    let file_name = CString::new(path.to_string_lossy().into_owned())?;

    Python::with_gil(|py| {
        // Make sibling modules importable from the script.
        if let Some(dir) = path.canonicalize().ok().as_ref().and_then(|p| p.parent()) {
            let sys = py.import("sys")?;
            sys.getattr("path")?
                .call_method1("insert", (0, dir.to_string_lossy().into_owned()))?;
        }

        // Reset the scene registry (it lives in the shared ranimpy module).
        let ranimpy = py.import("ranimpy")?;
        ranimpy.setattr("_scenes", PyList::empty(py))?;

        let module = PyModule::from_code(py, &code, &file_name, c"__ranim_script__")?;
        let _ = module;

        ranimpy
            .getattr("_scenes")?
            .downcast::<PyList>()?
            .iter()
            .map(|entry| {
                let (name, output_dir, clear_color, func) =
                    entry.extract::<(String, Option<String>, Option<String>, Py<PyAny>)>()?;
                Ok(PyScene {
                    name,
                    output_dir,
                    clear_color,
                    func,
                })
            })
            .collect()
    })
}
