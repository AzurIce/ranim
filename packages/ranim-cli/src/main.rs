use std::{
    env::args,
    ffi::CString,
    path::{Path, PathBuf},
};

use pyo3::{
    ffi::c_str,
    types::{PyAnyMethods, PyModule},
    Python,
};
use ranim::{animation::AnimWithParams, utils::rate_functions::linear, AppOptions, RanimRenderApp};
use ranimpy::{ranimpy as ranimpy_module, timeline::PyTimeline};

fn main() {
    let args = args().skip(1).collect::<Vec<_>>();
    if args.is_empty() || args.len() > 2 {
        panic!("usage: ranim <input-file> [<venv_dir_path>]")
    }

    let input_file = &args[0];
    let input_file = PathBuf::from(input_file);
    assert!(input_file.extension() == Some("py".as_ref()));

    // let filename = input_file.file_name().unwrap().to_str().unwrap();
    // let filename_without_ext = input_file.file_stem().unwrap().to_str().unwrap();

    let content = std::fs::read_to_string(&input_file).expect("failed to read from file");
    let content = CString::new(content).expect("failed to convert to CString");

    pyo3::append_to_inittab!(ranimpy_module);
    pyo3::prepare_freethreaded_python();
    Python::with_gil(|py| {
        let sys = PyModule::import(py, "sys").unwrap();

        let executable = sys.getattr("executable").unwrap();
        let path = sys.getattr("path").unwrap();
        let version = sys.getattr("version").unwrap();
        if args.len() == 2 {
            let venv_dir = Path::new(&args[1]);
            let site_packages_path =
                dunce::canonicalize(venv_dir.join("Lib/site-packages")).unwrap();
            path.call_method1("append", (site_packages_path.to_str().unwrap(),))
                .unwrap();
        }
        println!("pyo3 sys.executable: {}", executable);
        println!("pyo3 sys.path: {}", path);
        println!("pyo3 sys.version: {}", version);

        let module = PyModule::from_code(
            py,
            &content,
            c_str!("scene.py"),
            c_str!("scene"),
            // &CString::new(filename).unwrap(),
            // &CString::new(filename_without_ext).unwrap(),
        )
        .expect("failed to load module");

        let timeline = module
            .getattr("build_timeline")
            .expect("failed to get build_timeline attr")
            .call0()
            .expect("failed to call0 on build_timeline");
        println!("{:?}", timeline);
        let timeline = timeline
            .downcast::<PyTimeline>()
            .expect("failed to downcast to PyTimeline");
        let mut timeline = timeline.borrow_mut();

        let mut app = RanimRenderApp::new(&AppOptions::default());
        if timeline.elapsed_secs() == 0.0 {
            timeline.forward(0.1);
        }
        let duration_secs = timeline.elapsed_secs();
        app.render_anim(
            AnimWithParams::new(timeline.inner.clone())
                .with_duration(duration_secs)
                .with_rate_func(linear),
        );
    })
}

#[cfg(test)]
mod test {
    use pyo3::PyResult;

    use super::*;

    const TEST: &str = include_str!("../test/test.py");

    #[test]
    fn test_main() -> PyResult<()> {
        pyo3::append_to_inittab!(ranimpy_module);
        pyo3::prepare_freethreaded_python();
        Python::with_gil(|py| {
            let sys = PyModule::import(py, "sys")?;

            let executable = sys.getattr("executable")?;
            let path = sys.getattr("path")?;
            let version = sys.getattr("version")?;
            // if args.len() == 2 {
            //     let venv_dir = Path::new(&args[1]);
            let site_packages_path = dunce::canonicalize("../../.venv/Lib/site-packages").unwrap();
            path.call_method1("append", (site_packages_path.to_str().unwrap(),))
                .unwrap();
            // }
            println!("pyo3 sys.executable: {}", executable);
            println!("pyo3 sys.path: {}", path);
            println!("pyo3 sys.version: {}", version);

            let content = CString::new(TEST).expect("failed to convert to CString");
            let module = PyModule::from_code(
                py,
                &content,
                c_str!("scene.py"),
                c_str!("scene"),
                // &CString::new(filename).unwrap(),
                // &CString::new(filename_without_ext).unwrap(),
            )?;

            let timeline = module.getattr("build_timeline")?.call0()?;

            // let ranimpy = py.import("ranimpy")?;
            // let timeline = py.eval(c_str!("ranimpy.Timeline()"), None, None)?;
            println!("{:?}", timeline);
            let timeline = timeline.downcast_into::<PyTimeline>()?;

            Ok(())
        })
    }
}
