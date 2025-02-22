use std::{ffi::CString, path::PathBuf};

use color_print::cprintln;
use pyo3::{
    ffi::c_str,
    types::{PyAnyMethods, PyDictMethods, PyModule, PyModuleMethods},
    Bound, PyAny, PyResult, Python,
};
use ranim::{animation::AnimWithParams, utils::rate_functions::linear, AppOptions, RanimRenderApp};
use ranimpy::{ranimpy as ranimpy_module, timeline::PyTimeline};

/// 从 Python 模块中获取符合条件的 timeline 构建函数
/// 条件:
/// 1. 函数名以 timeline_ 开头
/// 2. 无参数
/// 3. 返回类型为 ranimpy.Timeline
fn get_timeline_funcs<'py>(
    py: &Python<'py>,
    module: &Bound<'py, PyModule>,
) -> PyResult<Vec<(String, Bound<'py, PyAny>)>> {
    let dict = module.dict();
    let timeline_type = py.import("ranimpy")?.getattr("Timeline")?;

    let mut result = Vec::new();

    for (_, obj) in dict.iter() {
        let Ok(func) = obj.downcast::<pyo3::types::PyFunction>() else {
            continue;
        };

        // 检查函数名
        let name = func.getattr("__name__")?.to_string();
        if !name.starts_with("timeline_") {
            continue;
        }

        // 检查参数个数
        if let Ok(code) = func.getattr("__code__") {
            if code.getattr("co_argcount")?.extract::<usize>()? != 0 {
                continue;
            }
        }

        // 检查返回类型注解
        let Ok(return_type) = func
            .getattr("__annotations__")
            .and_then(|annotations| annotations.get_item("return"))
        else {
            continue;
        };

        if !return_type.is(&timeline_type) {
            continue;
        }

        // 提取 timeline_xxx 中的 xxx 部分
        let name = name.strip_prefix("timeline_").unwrap().to_string();
        result.push((name, obj));
    }

    Ok(result)
}

use clap::Parser;

/// ranim CLI 参数
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
#[command(arg_required_else_help = true)]
struct Args {
    /// Python 源文件路径
    #[arg(value_name = "INPUT_FILE")]
    input_file: PathBuf,

    /// 虚拟环境目录路径
    #[arg(value_name = "VENV_DIR")]
    venv_dir: Option<PathBuf>,
}

fn main() -> anyhow::Result<()> {
    // let args = args_os()
    //     .into_iter()
    //     .skip_while(|arg| !arg.to_str().unwrap().contains("ranim"))
    //     .collect::<Vec<_>>();
    // println!("{:?}", args);
    let args = Args::parse();

    // 验证输入文件扩展名
    if args.input_file.extension() != Some("py".as_ref()) {
        anyhow::bail!("Input file must have .py extension");
    }

    let content = std::fs::read_to_string(&args.input_file).expect("failed to read from file");
    let content = CString::new(content).expect("failed to convert to CString");

    pyo3::append_to_inittab!(ranimpy_module);
    pyo3::prepare_freethreaded_python();
    Python::with_gil(|py| {
        let sys = PyModule::import(py, "sys")?;

        let executable = sys.getattr("executable")?;
        let version = sys.getattr("version")?;
        cprintln!("<b>pyo3</b> sys.executable: <dim>{}</dim>", executable);
        cprintln!("<b>pyo3</b> sys.version: <dim>{}</dim>", version);

        let path = sys.getattr("path")?;
        if let Some(venv_dir) = args.venv_dir {
            cprintln!("using venv {:?}", venv_dir);
            let site_packages_path =
                dunce::canonicalize(venv_dir.join("Lib/site-packages")).unwrap();
            path.call_method1("append", (site_packages_path.to_str().unwrap(),))?;
        }
        cprintln!("<b>pyo3</b> sys.path: <dim>{}</dim>", path);

        cprintln!("[ranim]: loading module {:?}...", args.input_file);
        let module = PyModule::from_code(py, &content, c_str!("scene.py"), c_str!("scene"))?;

        cprintln!("[ranim]: getting timeline funcs...");
        let timeline_funcs = get_timeline_funcs(&py, &module)?;

        cprintln!("[ranim]: building timelines...");
        let timelines = timeline_funcs
            .into_iter()
            .map(|(name, func)| {
                cprintln!("[ranim]: building timeline <strong>{}</strong>...", name);
                let timeline = func
                    .call0()
                    .map_err(|err| anyhow::anyhow!("{:?}", err))
                    .and_then(|t| {
                        t.downcast_into::<PyTimeline>()
                            .map_err(|err| anyhow::anyhow!("{:?}", err))
                    })?;
                Ok::<_, anyhow::Error>((name, timeline))
            })
            .collect::<anyhow::Result<Vec<_>>>()?;
        cprintln!(
            "[ranim]: built timelines: {:?}",
            timelines.iter().map(|(name, _)| name).collect::<Vec<_>>()
        );

        cprintln!("[ranim]: rendering timelines...");
        for (name, timeline) in timelines {
            cprintln!("[ranim]: rendering timeline <strong>{}</strong>...", name);
            let mut timeline = timeline.borrow_mut();
            let mut app = RanimRenderApp::new(&AppOptions {
                output_dir: PathBuf::from(format!("output/{}", name)),
                ..Default::default()
            });
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

        Ok(())
    })
}

#[cfg(test)]
mod test {
    use pyo3::PyResult;

    use super::*;

    #[test]
    fn test_downcast() -> PyResult<()> {
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

            let module = PyModule::from_code(
                py,
                c_str!(
                    r#"
                    import ranimpy

                    def build_timeline():
                        return ranimpy.Timeline()
                "#
                ),
                c_str!("scene.py"),
                c_str!("scene"),
            )?;

            let timeline = module.getattr("build_timeline")?.call0()?;
            assert!(timeline.downcast_into::<PyTimeline>().is_ok());

            Ok(())
        })
    }

    #[test]
    fn test_python_module_methods() -> PyResult<()> {
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

            let module = PyModule::from_code(
                py,
                c_str!(
                    r#"
import ranimpy
a = 1

def foo1() -> int:
    return 0

def foo2(a: int):
    pass

def timeline_foo1():
    return ranimpy.Timeline()

def timeline_foo2() -> ranimpy.Timeline:
    return ranimpy.Timeline()

def timeline_foo3():
    pass
    
def timeline_foo4():
    a = 0

def timeline_foo5() -> int:
    pass

def timeline_foo6(a) -> ranimpy.Timeline:
    return ranimpy.Timeline()

foo1()
foo2(foo1())
                "#
                ),
                c_str!("scene.py"),
                c_str!("scene"),
            )?;

            let timelines = get_timeline_funcs(&py, &module)?;
            assert!(timelines.len() == 1);
            assert!(timelines[0].0 == "foo2");

            Ok(())
        })
    }
}
