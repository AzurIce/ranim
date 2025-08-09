use libloading::Library;
use std::path::{Path, PathBuf};
use ranim::Scene;

pub mod cli;
pub mod workspace;

pub struct RanimUserLibrary {
    inner: Option<Library>,
    temp_path: PathBuf,
}

impl RanimUserLibrary {
    pub fn load(dylib_path: impl AsRef<Path>) -> Self {
        let dylib_path = dylib_path.as_ref();

        let temp_dir = std::env::temp_dir();
        let file_name = dylib_path.file_name().unwrap();

        // 使用时间戳和随机数确保每次都有唯一的临时文件名
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let temp_path = temp_dir.join(format!(
            "ranim_{}_{}_{}",
            std::process::id(),
            timestamp,
            file_name.to_string_lossy()
        ));

        std::fs::copy(dylib_path, &temp_path).unwrap();

        let lib = unsafe { Library::new(&temp_path).unwrap() };
        Self {
            inner: Some(lib),
            temp_path,
        }
    }

    pub fn get_preview_func(&self) -> &Scene {
        unsafe {
            use libloading::Symbol;

            let scenes: Symbol<extern "C" fn() -> &'static [Scene]> =
                self.inner.as_ref().unwrap().get(b"scenes").unwrap();
            let scenes = scenes();

            scenes.iter().find(|s| s.preview).expect("no scene marked with `#[preview]` found")
        }
    }
}

impl Drop for RanimUserLibrary {
    fn drop(&mut self) {
        println!("Dropping RanimUserLibrary...");

        drop(self.inner.take());
        std::fs::remove_file(&self.temp_path).unwrap();
    }
}