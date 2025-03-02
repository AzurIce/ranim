use serde::{Deserialize, Serialize};
use std::env;
use std::error::Error;
use std::fs::{self, create_dir_all, read_to_string};
use std::path::{Path, PathBuf};
use std::process::Command;
use walkdir::WalkDir;

const EXCLUDE_EXAMPLE: [&str; 1] = ["test_scene"];

#[derive(Serialize, Deserialize)]
struct ExampleMeta {
    name: String,
    code: String,
    output_type: String, // "video" 或 "image"
    output_path: String,
}

fn main() -> Result<(), Box<dyn Error>> {
    let xtask_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = xtask_root.join("../../");

    let examples_dir = workspace_root.join("examples");
    let website_root = workspace_root.join("website");
    let website_data_dir = website_root.join("data");
    let website_static_examples_dir = website_root.join("static").join("examples");

    // 确保目标目录存在
    create_dir_all(&website_static_examples_dir)?;
    create_dir_all(&website_data_dir)?;

    // 获取命令行参数
    let args: Vec<String> = env::args().skip(1).collect();

    // 获取所有示例目录
    let mut example_dirs: Vec<PathBuf> = fs::read_dir(&examples_dir)?
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false))
        .map(|entry| entry.path())
        .filter(|p| !EXCLUDE_EXAMPLE.contains(&p.file_name().unwrap().to_str().unwrap()))
        .collect();

    // 如果提供了命令行参数，则只处理指定的示例
    if !args.is_empty() {
        example_dirs.retain(|dir| {
            if let Some(name) = dir.file_name().and_then(|n| n.to_str()) {
                args.contains(&name.to_string())
            } else {
                false
            }
        });
    }

    println!("找到 {} 个示例", example_dirs.len());

    // 处理每个示例
    for example_dir in example_dirs {
        let example_name = example_dir
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or("无法获取示例名称")?;

        println!("处理示例: {}", example_name);

        // 运行示例
        run_example(example_name, &workspace_root)?;

        // 复制输出文件
        let output_dir = workspace_root.join("output").join(example_name);
        let example_output_dir = website_static_examples_dir.join(example_name);
        create_dir_all(&example_output_dir)?;

        // 确定输出类型并复制文件
        let (output_type, output_path) = copy_output_files(&output_dir, &example_output_dir)?;

        // 读取示例源代码
        let main_rs_path = example_dir.join("main.rs");
        let code = read_to_string(main_rs_path)?;

        // 创建元数据
        let meta = ExampleMeta {
            name: example_name.to_string(),
            code: format!("```rust\n{code}\n```"),
            output_type,
            output_path: format!("/examples/{}/{}", example_name, output_path),
        };

        // 写入元数据文件
        let meta_path = website_data_dir.join(format!("{}.toml", example_name));
        let meta_toml = toml::to_string(&meta)?;
        fs::write(meta_path, meta_toml)?;

        println!("示例 {} 处理完成", example_name);
    }

    println!("所有示例处理完成");
    Ok(())
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
) -> Result<(String, String), Box<dyn Error>> {
    // 查找输出文件
    let mut output_type = String::new();
    let mut output_path = String::new();

    for entry in WalkDir::new(source_dir).into_iter().filter_map(Result::ok) {
        let path = entry.path();
        if path.is_file() {
            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                if ext == "mp4" {
                    output_type = "video".to_string();
                    output_path = copy_file(path, target_dir)?;
                } else if ext == "png" || ext == "jpg" || ext == "jpeg" {
                    output_type = "image".to_string();
                    output_path = copy_file(path, target_dir)?;
                }
            }
        }
    }

    if output_type.is_empty() {
        return Err("未找到输出文件".into());
    }

    Ok((output_type, output_path))
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
