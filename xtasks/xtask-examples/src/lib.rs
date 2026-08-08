use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::{Context, Result, bail};
use clap::Parser;
use ranim_cli::cli::Cli;
use serde::{Deserialize, Serialize};
use sha1::{Digest, Sha1};
use toml::Table;
use walkdir::WalkDir;

fn copy_file(source: &Path, target_dir: &Path) -> Result<String> {
    let file_name = source
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or(anyhow::anyhow!("无法获取文件名"))?
        .to_string();

    let target_path = target_dir.join(&file_name);
    std::fs::copy(source, &target_path)?;

    Ok(file_name)
}

/// Optimize a wasm-bindgen output module for website delivery.
///
/// `wasm-bindgen` may rewrite the module it receives, so this must run after
/// bindgen has produced the final `*_bg.wasm` file in the package directory.
fn optimize_wasm(wasm_path: &Path) -> Result<()> {
    let optimized_path = wasm_path.with_extension("wasm-opt.wasm");
    if optimized_path.exists() {
        std::fs::remove_file(&optimized_path)
            .with_context(|| format!("failed to remove {}", optimized_path.display()))?;
    }

    let status = Command::new("wasm-opt")
        .args(["-Oz", "--output"])
        .arg(&optimized_path)
        .arg(wasm_path)
        .stdout(std::process::Stdio::null())
        .status()
        .context(
            "failed to run wasm-opt; install Binaryen and ensure `wasm-opt` is available on PATH",
        )?;
    if !status.success() {
        bail!("wasm-opt failed for {}", wasm_path.display());
    }

    std::fs::copy(&optimized_path, wasm_path).with_context(|| {
        format!(
            "failed to replace {} with wasm-opt output",
            wasm_path.display()
        )
    })?;
    std::fs::remove_file(&optimized_path)
        .with_context(|| format!("failed to remove {}", optimized_path.display()))?;
    Ok(())
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Example {
    pub path: PathBuf,
    /// The name of the example, corresponding to the string passed to `--example`
    pub name: String,
    pub code: String,
    pub hash: String,
    pub required_features: Vec<String>,

    pub meta: ExampleMeta,
}

impl Example {
    pub fn clean_wasm(&self, root_dir: impl AsRef<Path>) -> Result<()> {
        let root_dir = root_dir.as_ref();

        let website_root = root_dir.join("website");
        let output_dir = website_root
            .join("static/examples")
            .join(&self.name)
            .join("pkg");
        if std::fs::exists(&output_dir)? {
            std::fs::remove_dir_all(&output_dir)
                .with_context(|| format!("failed to clean {}", output_dir.display()))?;
        }
        Ok(())
    }

    pub fn clean_output(&self, root_dir: impl AsRef<Path>) -> Result<()> {
        let root_dir = root_dir.as_ref();

        let website_root = root_dir.join("website");
        let output_dir = website_root.join("static/examples").join(&self.name);
        if !std::fs::exists(&output_dir)? {
            return Ok(());
        }
        for entry in std::fs::read_dir(&output_dir)? {
            let Ok(entry) = entry else {
                continue;
            };
            let Ok(t) = entry.file_type() else {
                continue;
            };

            if t.is_dir() {
                if entry.file_name().to_string_lossy() == "pkg" {
                    continue;
                }
                std::fs::remove_dir_all(entry.path())
                    .with_context(|| format!("failed to clean {}", entry.path().display()))?;
            } else {
                std::fs::remove_file(entry.path())
                    .with_context(|| format!("failed to clean {}", entry.path().display()))?;
            }
        }
        Ok(())
    }

    /// Render the scene and refresh `website/static/examples/<example-name>/`
    /// and `website/data/<example-name>.toml`.
    ///
    /// The render runs **before** any existing website output is cleaned, so a
    /// failed render leaves the previous outputs untouched.
    pub fn run(&self, root_dir: impl AsRef<Path>, lazy: bool) -> Result<()> {
        #[derive(Serialize, Deserialize)]
        struct OutputData {
            name: String,
            code: String,
            hash: String,

            preview_imgs: Vec<String>,
            output_files: Vec<String>,
            wasm: bool,
        }

        let root_dir = root_dir.as_ref();
        let website_root = root_dir.join("website");
        let output_dir = website_root
            .join("static")
            .join("examples")
            .join(&self.name);
        std::fs::create_dir_all(&output_dir)?;
        let data_dir = website_root.join("data");

        let data_path = data_dir.join(format!("{}.toml", self.name));
        // Missing or outdated data is not an error — it just means "regenerate".
        let up_to_date = std::fs::read_to_string(&data_path)
            .ok()
            .and_then(|data| toml::from_str::<OutputData>(&data).ok())
            .is_some_and(|data| data.hash == self.hash);
        if lazy && up_to_date {
            return Ok(());
        }

        let mut args = vec!["ranim", "render", "--example", &self.name];
        let features = self.required_features.join(",");
        if !features.is_empty() {
            args.extend(["--features", &features]);
        }
        Cli::parse_from(args)
            .run()
            .with_context(|| format!("failed to render example <{}>", self.name))?;

        self.clean_output(root_dir)?;

        let mut preview_imgs = vec![];
        let mut output_files = vec![];
        for entry in WalkDir::new(root_dir.join("output").join(&self.name))
            .into_iter()
            .filter_map(Result::ok)
        {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
                continue;
            };
            let Some(ext) = path.extension().and_then(|e| e.to_str()) else {
                continue;
            };
            if file_name.starts_with("preview") && ["png", "jpg"].contains(&ext) {
                preview_imgs.push(copy_file(path, &output_dir)?);
            } else if ["mp4", "webm", /*"mov",*/ "gif", "png", "jpg"].contains(&ext) {
                output_files.push(copy_file(path, &output_dir)?);
            }
        }

        let output_data = OutputData {
            name: self.name.clone(),
            code: self.code.clone(),
            hash: self.hash.clone(),

            preview_imgs: preview_imgs
                .into_iter()
                .map(|f| format!("/examples/{}/{}", self.name, f))
                .collect(),
            output_files: output_files
                .into_iter()
                .map(|f| format!("/examples/{}/{}", self.name, f))
                .collect(),
            wasm: self.meta.wasm,
        };
        let output_data = toml::to_string(&output_data)?;

        std::fs::write(&data_path, output_data)?;
        if !self.meta.hide {
            self.create_example_page(root_dir)?;
        }
        Ok(())
    }

    /// Build wasm to `website/static/examples/<example-name>/pkg/`
    pub fn build_wasm(&self, root_dir: impl AsRef<Path>) -> Result<()> {
        let root_dir = root_dir.as_ref();
        let website_root = root_dir.join("website");
        let output_dir = website_root
            .join("static")
            .join("examples")
            .join(&self.name);
        std::fs::create_dir_all(&output_dir)?;

        self.clean_wasm(root_dir)?;

        if !self.meta.wasm {
            return Ok(());
        }

        let features = std::iter::once("preview")
            .chain(self.required_features.iter().map(String::as_str))
            .collect::<Vec<_>>()
            .join(",");
        let status = Command::new("cargo")
            .current_dir(root_dir)
            .args([
                "build",
                "--example",
                &self.name,
                "--features",
                &features,
                "--target",
                "wasm32-unknown-unknown",
                "--release",
            ])
            .stdout(std::process::Stdio::null())
            .status()
            .context("failed to run cargo build for wasm")?;
        if !status.success() {
            bail!("failed to build wasm for example <{}>", self.name);
        }
        let status = Command::new("wasm-bindgen")
            .current_dir(root_dir)
            .args([
                "--out-dir",
                output_dir.join("pkg").as_os_str().to_str().unwrap(),
                "--target",
                "web",
                &format!(
                    "target/wasm32-unknown-unknown/release/examples/{}.wasm",
                    self.name
                ),
            ])
            .stdout(std::process::Stdio::null())
            .status()
            .context("failed to run wasm-bindgen; is it installed?")?;
        if !status.success() {
            bail!("wasm-bindgen failed for example <{}>", self.name);
        }

        let wasm_path = output_dir
            .join("pkg")
            .join(format!("{}_bg.wasm", self.name));
        optimize_wasm(&wasm_path)
            .with_context(|| format!("failed to optimize wasm for example <{}>", self.name))?;
        Ok(())
    }

    pub fn create_example_page(&self, root_dir: impl AsRef<Path>) -> Result<()> {
        let root_dir = root_dir.as_ref();
        let website_root = root_dir.join("website");

        let example_dir = root_dir.join("examples").join(&self.name);

        let readme_path = example_dir.join("README.md");
        let output_page_path = website_root
            .join("content")
            .join("examples")
            .join(format!("{}.md", self.name));

        // 创建markdown内容
        let mut md_content = format!(
            r#"+++
title = "{}"
template = "examples-page.html"
+++

"#,
            self.name
        );

        // 如果README.md存在，则复制其内容
        if readme_path.exists() {
            let readme_content = std::fs::read_to_string(readme_path)?;
            md_content.push_str(&readme_content);

            // 确保README内容后有换行
            if !readme_content.ends_with('\n') {
                md_content.push('\n');
            }

            // 如果最后一行不是空行，添加一个空行
            if !md_content.ends_with("\n\n") {
                md_content.push('\n');
            }
        }

        // 添加示例标记
        md_content.push_str(&format!("!example-{}\n", self.name));

        // 写入markdown文件
        std::fs::write(&output_page_path, md_content)
            .with_context(|| format!("failed to write {}", output_page_path.display()))?;
        Ok(())
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct ExampleMeta {
    #[serde(default)]
    wasm: bool,
    #[serde(default)]
    hide: bool,
}

pub fn get_examples(root_dir: impl AsRef<Path>) -> Result<Vec<Example>> {
    let root_dir = root_dir.as_ref();

    let manifest_path = root_dir.join("Cargo.toml");
    let manifest_file = std::fs::read_to_string(&manifest_path)
        .with_context(|| format!("failed to read {}", manifest_path.display()))?;

    let manifest = manifest_file
        .parse::<Table>()
        .context("failed to parse workspace Cargo.toml")?;

    let metadatas = manifest["package"]["metadata"]["example"]
        .clone()
        .try_into::<HashMap<String, ExampleMeta>>()
        .context("failed to parse [package.metadata.example]")?;

    manifest["example"]
        .as_array()
        .context("no [[example]] targets in workspace Cargo.toml")?
        .iter()
        .map(|v| v.as_table().unwrap())
        .map(|item| {
            let name = item["name"].as_str().unwrap().to_string();
            let required_features = item
                .get("required-features")
                .and_then(toml::Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(toml::Value::as_str)
                .map(str::to_owned)
                .collect();

            let path = root_dir.join(item["path"].as_str().unwrap());
            let code = std::fs::read_to_string(&path)
                .with_context(|| format!("failed to read example source {}", path.display()))?;
            let hash = {
                let mut hasher = Sha1::new();
                hasher.update(code.as_bytes());
                base16ct::lower::encode_string(&hasher.finalize())
            };
            Ok(Example {
                meta: metadatas.get(&name).cloned().unwrap_or_default(),
                name,
                path,
                code: format!("```rust\n{code}```"),
                hash,
                required_features,
            })
        })
        .collect()
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_get_examples() {
        let xtask_root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let root_dir = xtask_root.join("../../");
        let examples = get_examples(&root_dir).unwrap();
        assert_eq!(examples.len(), 31); // + iterative_spring, nbody, cloth_wrap
        assert_eq!(
            examples
                .iter()
                .find(|example| example.name == "basic")
                .unwrap()
                .required_features,
            ["typst"]
        );
        println!("found {} examples:", examples.len());
        examples.iter().for_each(|example| {
            println!("{}", example.name);
            println!("{:?}", example.meta)
        });
    }

    /// Renders an example end-to-end (requires a GPU); ignored by default.
    #[test]
    #[ignore]
    fn test_example_run() {
        let xtask_root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let root_dir = xtask_root.join("../../");
        let examples = get_examples(&root_dir).unwrap();
        println!("{:?}", examples[0].name);
        examples[0].run(&root_dir, false).unwrap();
    }

    /// Builds the wasm package (requires the wasm32 target, wasm-bindgen and
    /// wasm-opt); ignored by default.
    #[test]
    #[ignore]
    fn test_example_build_wasm() {
        let xtask_root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let root_dir = xtask_root.join("../../");
        let examples = get_examples(&root_dir).unwrap();
        println!("{:?}", examples[0].name);
        examples[0].build_wasm(&root_dir).unwrap();
    }

    /// Destructive: deletes the example's website outputs; ignored by default.
    #[test]
    #[ignore]
    fn test_example_clean_output() {
        let xtask_root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let root_dir = xtask_root.join("../../");
        let examples = get_examples(&root_dir).unwrap();
        println!("{:?}", examples[0].name);
        examples[0].clean_output(&root_dir).unwrap();
    }

    /// Destructive: deletes the example's wasm package; ignored by default.
    #[test]
    #[ignore]
    fn test_example_clean_wasm() {
        let xtask_root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let root_dir = xtask_root.join("../../");
        let examples = get_examples(&root_dir).unwrap();
        println!("{:?}", examples[0].name);
        examples[0].clean_wasm(&root_dir).unwrap();
    }
}
