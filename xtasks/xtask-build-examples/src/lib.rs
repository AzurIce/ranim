use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::Result;
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

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Example {
    pub path: PathBuf,
    /// The name of the example, corresponding to the string passed to `--example`
    pub name: String,
    pub code: String,
    pub hash: String,

    pub meta: ExampleMeta,
}

impl Example {
    pub fn clean(&self, root_dir: impl AsRef<Path>) {
        let root_dir = root_dir.as_ref();

        let website_root = root_dir.join("website");
        let output_dir = website_root.join("static/examples").join(&self.name);
        if std::fs::exists(&output_dir).unwrap() {
            std::fs::remove_dir_all(&output_dir).expect("failed to clean example");
        }
    }
    pub fn build(&self, root_dir: impl AsRef<Path>, lazy: bool) {
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
        std::fs::create_dir_all(&output_dir).expect("failed to create dir");
        let data_dir = website_root.join("data");

        let data_path = data_dir.join(format!("{}.toml", self.name));
        if std::fs::exists(&data_path).unwrap() {
            let data = std::fs::read_to_string(&data_path).unwrap();
            let data = toml::from_str::<OutputData>(&data).unwrap();
            if data.hash == self.hash && lazy {
                return;
            }
        }

        let mut preview_imgs = vec![];
        let mut output_files = vec![];
        // Run example (build video and image)
        let status = Command::new("cargo")
            .current_dir(root_dir)
            .args(["run", "--example", &self.name, "--release"])
            .stdout(std::process::Stdio::null())
            .status()
            .unwrap();
        if !status.success() {
            panic!("failed to build example")
        }
        for entry in WalkDir::new(root_dir.join("output").join(&self.name))
            .into_iter()
            .filter_map(Result::ok)
        {
            let path = entry.path();
            if path.is_file() {
                if let Some(file_name) = path.file_name().and_then(|name| name.to_str()) {
                    print!("Found {file_name:?}");
                    if file_name.starts_with("preview") {
                        print!(", preview file");
                        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                            if ["png", "jpg"].contains(&ext) {
                                println!(", copying...");
                                preview_imgs.push(
                                    copy_file(path, &output_dir)
                                        .expect("failed to copy preview img"),
                                );
                            }
                        }
                    } else if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                        print!(", output file");
                        if ["mp4", "png", "jpg"].contains(&ext) {
                            print!(", copying...");
                            output_files.push(
                                copy_file(path, &output_dir).expect("failed to copy output file"),
                            );
                        }
                    }
                    println!()
                }
            }
        }

        // Build wasm
        if self.meta.wasm {
            let status = Command::new("cargo")
                .current_dir(root_dir)
                .args([
                    "build",
                    "--example",
                    &self.name,
                    "--target",
                    "wasm32-unknown-unknown",
                    "--release",
                    "--features",
                    "app",
                ])
                .stdout(std::process::Stdio::null())
                .status()
                .unwrap();
            if !status.success() {
                panic!("failed to build wasm")
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
                .unwrap();
            if !status.success() {
                panic!("failed to run wasm-bindgen")
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
        let output_data = toml::to_string(&output_data).unwrap();

        std::fs::write(&data_path, output_data).expect("failed to write data.toml");
        if !self.meta.hide {
            self.create_example_page(root_dir);
        }
    }

    pub fn create_example_page(&self, root_dir: impl AsRef<Path>) {
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
            let readme_content = std::fs::read_to_string(readme_path).unwrap();
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
        std::fs::write(output_page_path, md_content).unwrap();
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct ExampleMeta {
    #[serde(default)]
    wasm: bool,
    #[serde(default)]
    hide: bool,
}

pub fn get_examples(root_dir: impl AsRef<Path>) -> Vec<Example> {
    let root_dir = root_dir.as_ref();

    let manifest_path = root_dir.join("Cargo.toml");
    let manifest_file = std::fs::read_to_string(manifest_path).unwrap();

    let manifest = manifest_file.parse::<Table>().unwrap();

    let metadatas = manifest["package"]["metadata"]["example"]
        .clone()
        .try_into::<HashMap<String, ExampleMeta>>()
        .unwrap();

    manifest["example"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_table().unwrap())
        .map(|item| {
            let name = item["name"].as_str().unwrap().to_string();

            let path = root_dir.join(item["path"].as_str().unwrap());
            let code = std::fs::read_to_string(&path).unwrap();
            let hash = {
                let mut hasher = Sha1::new();
                hasher.update(code.as_bytes());
                base16ct::lower::encode_string(&hasher.finalize())
            };
            Example {
                meta: metadatas.get(&name).cloned().unwrap_or_default(),
                name,
                path,
                code: format!("```rust\n{code}```"),
                hash,
            }
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
        let examples = get_examples(&root_dir);
        println!("found {} examples:", examples.len());
        examples.iter().for_each(|example| {
            println!("{}", example.name);
            println!("{:?}", example.meta)
        });
    }

    #[test]
    fn test_build_example() {
        let xtask_root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let root_dir = xtask_root.join("../../");
        let examples = get_examples(&root_dir);
        println!("{:?}", examples[0].name);
        examples[0].build(&root_dir, false);
    }
    #[test]
    fn test_clean_example() {
        let xtask_root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let root_dir = xtask_root.join("../../");
        let examples = get_examples(&root_dir);
        println!("{:?}", examples[0].name);
        examples[0].clean(&root_dir);
    }
}
