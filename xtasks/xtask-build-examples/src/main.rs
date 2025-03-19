use clap::Parser;
use serde::{Deserialize, Serialize};
use sha1::{Digest, Sha1};
use std::env;
use std::error::Error;
use std::fs::{self, create_dir_all, read_to_string};
use std::path::{Path, PathBuf};
use std::process::Command;
use walkdir::WalkDir;

const EXCLUDE_EXAMPLES: [&str; 1] = ["test"];
const HIDE_EXAMPLES: [&str; 4] = [
    "getting_started0",
    "getting_started1",
    "getting_started2",
    "getting_started3",
];

#[derive(Parser)]
#[command(author, version, about = "build ranim examples")]
struct Args {
    /// 指定要处理的示例名称，不指定则处理所有示例
    #[arg(value_name = "EXAMPLES")]
    examples: Vec<String>,

    #[arg(long)]
    lazy_run: bool,

    /// 清除不存在的示例对应的输出文件
    #[arg(long)]
    clean: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct ExampleMeta {
    name: String,
    code: String,
    hash: String,
    preview_imgs: Vec<String>,
    output_files: Vec<String>,
}

fn main() -> Result<(), Box<dyn Error>> {
    // 解析命令行参数
    let args = Args::parse();

    let xtask_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = xtask_root.join("../../");

    let examples_dir = workspace_root.join("examples");
    let website_root = workspace_root.join("website");
    let website_data_dir = website_root.join("data");
    let website_static_examples_dir = website_root.join("static").join("examples");
    let website_content_examples_dir = website_root.join("content").join("examples");
    let output_dir = workspace_root.join("output");

    // 确保目标目录存在
    create_dir_all(&website_static_examples_dir)?;
    create_dir_all(&website_data_dir)?;
    create_dir_all(&website_content_examples_dir)?;

    let example_dirs = get_examples(examples_dir, &args.examples);
    println!(
        "找到 {} 个示例: {:?}",
        example_dirs.len(),
        example_dirs
            .iter()
            .map(|p| p.file_name().unwrap())
            .collect::<Vec<_>>()
    );

    // 如果指定了clean选项，清理不存在的示例输出
    if args.clean {
        clean_nonexistent_examples(
            &example_dirs,
            &output_dir,
            &website_data_dir,
            &website_static_examples_dir,
            &website_content_examples_dir,
        )?;
    }

    // 处理每个示例
    for example_dir in example_dirs {
        let example_name = example_dir
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap();
        println!("处理示例: {}", example_name);

        // 读取示例源代码
        let code = read_to_string(example_dir.join("main.rs"))?;
        let mut hasher = Sha1::new();
        hasher.update(code.as_bytes());
        let hash = hasher.finalize();
        let hash = base16ct::lower::encode_string(&hash);

        // 创建元数据
        let new_meta = ExampleMeta {
            name: example_name.to_string(),
            code: format!("```rust,linenos\n{code}\n```"),
            hash: hash.clone(),
            preview_imgs: Vec::new(),
            output_files: Vec::new(),
        };

        let old_meta = read_to_string(website_data_dir.join(format!("{}.toml", example_name)))
            .ok()
            .and_then(|s| toml::from_str::<ExampleMeta>(&s).ok());
        let mut new_meta = match old_meta.clone() {
            Some(old_meta) => {
                // Only need to update code and hash
                ExampleMeta {
                    code: new_meta.code,
                    hash: new_meta.hash,
                    ..old_meta
                }
            }
            None => new_meta,
        };
        // 如果非 Lazy 或无法读取 toml 或 toml 中无输出或 hash 有变化，则重新运行
        if !args.lazy_run
            || old_meta
                .map(|meta| meta.hash != new_meta.hash || meta.output_files.is_empty())
                .unwrap_or(true)
        {
            // 运行示例
            run_example(example_name, &workspace_root)?;

            // 复制输出文件
            let output_dir = workspace_root.join("output").join(example_name);
            let example_output_dir = website_static_examples_dir.join(example_name);
            create_dir_all(&example_output_dir)?;

            // 复制文件并更新元数据
            let (preview_imgs, output_files) = copy_output_files(&output_dir, &example_output_dir)?;
            new_meta.preview_imgs = preview_imgs
                .into_iter()
                .map(|path| format!("/examples/{}/{}", example_name, path))
                .collect();
            new_meta.output_files = output_files
                .into_iter()
                .map(|path| format!("/examples/{}/{}", example_name, path))
                .collect();
        }

        // 写入元数据文件
        let meta_path = website_data_dir.join(format!("{}.toml", example_name));
        let meta_toml = toml::to_string(&new_meta)?;
        fs::write(meta_path, meta_toml)?;

        // 处理README.md并创建content/examples下的markdown文件
        if !HIDE_EXAMPLES.contains(&example_name) {
            create_example_page(example_name, &example_dir, &website_content_examples_dir);
        }

        println!("示例 {} 处理完成", example_name);
    }

    println!("所有示例处理完成");
    Ok(())
}

fn create_example_page(
    example_name: impl AsRef<str>,
    example_dir: impl AsRef<Path>,
    website_content_examples_dir: impl AsRef<Path>,
) {
    let example_name = example_name.as_ref();
    let example_dir = example_dir.as_ref();
    let website_content_examples_dir = website_content_examples_dir.as_ref();

    let readme_path = example_dir.join("README.md");
    let content_md_path = website_content_examples_dir.join(format!("{}.md", example_name));

    // 创建markdown内容
    let mut md_content = format!(
        r#"+++
title = "{}"
template = "examples-page.html"
+++

"#,
        example_name
    );

    // 如果README.md存在，则复制其内容
    if readme_path.exists() {
        let readme_content = read_to_string(readme_path).unwrap();
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
    md_content.push_str(&format!("!example-{}\n", example_name));

    // 写入markdown文件
    fs::write(content_md_path, md_content).unwrap();
}

fn get_examples(examples_dir: impl AsRef<Path>, example_filter: &[String]) -> Vec<PathBuf> {
    // 获取所有示例目录
    let mut example_dirs: Vec<PathBuf> = fs::read_dir(&examples_dir)
        .unwrap()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false))
        .map(|entry| entry.path())
        .filter(|p| !EXCLUDE_EXAMPLES.contains(&p.file_name().unwrap().to_str().unwrap()))
        .collect();

    // 如果提供了示例名称参数，则只处理指定的示例
    if !example_filter.is_empty() {
        example_dirs.retain(|dir| {
            dir.file_name()
                .and_then(|n| n.to_str())
                .map(|name| example_filter.contains(&name.to_string()))
                .unwrap_or(false)
        });
    }
    example_dirs
}

fn run_example(example_name: &str, workspace_root: &Path) -> Result<(), Box<dyn Error>> {
    println!("运行示例: {}", example_name);

    let status = Command::new("cargo")
        .current_dir(workspace_root)
        .args(["run", "--example", example_name, "--release"])
        .status()?;

    if !status.success() {
        return Err(format!("示例 {} 运行失败", example_name).into());
    }

    Ok(())
}

fn copy_output_files(
    source_dir: &Path,
    target_dir: &Path,
) -> Result<(Vec<String>, Vec<String>), Box<dyn Error>> {
    let mut preview_imgs = Vec::new();
    let mut output_files = Vec::new();

    for entry in WalkDir::new(source_dir).into_iter().filter_map(Result::ok) {
        let path = entry.path();
        if path.is_file() {
            if let Some(file_name) = path.file_name().and_then(|name| name.to_str()) {
                print!("Found {:?}", file_name);
                if file_name.starts_with("preview") {
                    print!(", preview file");
                    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                        if ["png", "jpg"].contains(&ext) {
                            println!(", copying...");
                            preview_imgs.push(copy_file(path, target_dir)?);
                        }
                    }
                } else if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                    print!(", output file");
                    if ["mp4", "png", "jpg"].contains(&ext) {
                        print!(", copying...");
                        output_files.push(copy_file(path, target_dir)?);
                    }
                }
                println!()
            }
        }
    }

    if output_files.is_empty() {
        return Err("未找到输出文件".into());
    }

    Ok((preview_imgs, output_files))
}

fn copy_file(source: &Path, target_dir: &Path) -> Result<String, Box<dyn Error>> {
    let file_name = source
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or("无法获取文件名")?
        .to_string();

    let target_path = target_dir.join(&file_name);
    fs::copy(source, &target_path)?;

    Ok(file_name)
}

/// 清理不存在的示例对应的输出文件
fn clean_nonexistent_examples(
    example_dirs: &[PathBuf],
    output_dir: &Path,
    website_data_dir: &Path,
    website_static_examples_dir: &Path,
    website_content_examples_dir: &Path,
) -> Result<(), Box<dyn Error>> {
    println!("开始清理不存在的示例输出...");

    // 获取所有现存示例的名称
    let existing_examples: Vec<String> = example_dirs
        .iter()
        .filter_map(|p| p.file_name().and_then(|n| n.to_str()).map(String::from))
        .collect();

    // 清理output目录
    if output_dir.exists() {
        for entry in fs::read_dir(output_dir)? {
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                if let Some(dir_name) = entry.file_name().to_str() {
                    if !existing_examples.contains(&dir_name.to_string())
                        && !EXCLUDE_EXAMPLES.contains(&dir_name)
                    {
                        println!("删除output目录中的: {}", dir_name);
                        fs::remove_dir_all(entry.path())?;
                    }
                }
            }
        }
    }

    // 清理website/data目录
    if website_data_dir.exists() {
        for entry in fs::read_dir(website_data_dir)? {
            let entry = entry?;
            if entry.file_type()?.is_file() {
                if let Some(file_name) = entry.file_name().to_str() {
                    if file_name.ends_with(".toml") {
                        let example_name = file_name.trim_end_matches(".toml");
                        if !existing_examples.contains(&example_name.to_string()) {
                            println!("删除website/data中的: {}", file_name);
                            fs::remove_file(entry.path())?;
                        }
                    }
                }
            }
        }
    }

    // 清理website/static/examples目录
    if website_static_examples_dir.exists() {
        for entry in fs::read_dir(website_static_examples_dir)? {
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                if let Some(dir_name) = entry.file_name().to_str() {
                    if !existing_examples.contains(&dir_name.to_string()) {
                        println!("删除website/static/examples中的: {}", dir_name);
                        fs::remove_dir_all(entry.path())?;
                    }
                }
            }
        }
    }

    // 清理website/content/examples目录
    if website_content_examples_dir.exists() {
        for entry in fs::read_dir(website_content_examples_dir)? {
            let entry = entry?;
            if entry.file_type()?.is_file() {
                if let Some(file_name) = entry.file_name().to_str() {
                    if file_name.ends_with(".md") {
                        let example_name = file_name.trim_end_matches(".md");
                        if !existing_examples.contains(&example_name.to_string())
                            && example_name != "_index"
                        {
                            println!("删除website/content/examples中的: {}", file_name);
                            fs::remove_file(entry.path())?;
                        }
                    }
                }
            }
        }
    }

    println!("清理完成");
    Ok(())
}
