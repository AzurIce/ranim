pub mod items;
pub mod timeline;

use std::path::PathBuf;

use pyo3::{
    pyfunction, pymodule,
    types::{PyModule, PyModuleMethods},
    wrap_pyfunction, Bound, PyResult,
};
use ranim::{animation::AnimWithParams, utils::rate_functions::linear, AppOptions, RanimRenderApp};
use timeline::PyTimeline;

#[pyfunction]
fn render_timeline(timeline: Bound<'_, PyTimeline>, output_dir: PathBuf) {
    let options = AppOptions {
        output_dir,
        ..Default::default()
    };

    let mut app = RanimRenderApp::new(&options);
    let mut timeline = timeline.borrow_mut();
    if timeline.elapsed_secs() == 0.0 {
        timeline.forward(0.1);
    }
    let duration_secs = timeline.elapsed_secs();
    app.render_anim(
        AnimWithParams::new(timeline.inner.clone())
            .with_duration(duration_secs)
            .with_rate_func(linear),
    );
}

#[pymodule]
pub fn ranimpy(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(render_timeline, m)?)?;
    m.add_class::<PyTimeline>()?;
    m.add_class::<items::PySvgItem>()?;
    m.add_class::<items::PyVItem>()?;

    // m.add_wrapped(wrap_pymodule!(items::items))?;
    // let submodule = PyModule::new(py, "items")?;
    // items::items(&submodule)?;
    // m.add_submodule(&submodule)?;
    Ok(())
}
