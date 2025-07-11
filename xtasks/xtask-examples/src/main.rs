use clap::{Parser, Subcommand};
use std::env;
use std::error::Error;
use std::path::Path;
use xtask_examples::get_examples;

#[derive(Parser)]
#[command(author, version, about = "build ranim examples")]
struct Args {
    /// 指定要处理的示例名称，不指定则处理所有示例
    #[arg(value_name = "EXAMPLES")]
    examples: Vec<String>,

    /// 清除不存在的示例对应的输出文件
    #[arg(long)]
    clean: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Build,
    Run {
        #[arg(long)]
        lazy_run: bool,
    },
}

fn main() -> Result<(), Box<dyn Error>> {
    // 解析命令行参数
    let args = Args::parse();

    let xtask_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = xtask_root.join("../../");

    let website_root = workspace_root.join("website");
    std::fs::create_dir_all(website_root.join("static").join("examples"))?;
    std::fs::create_dir_all(website_root.join("content").join("examples"))?;
    std::fs::create_dir_all(website_root.join("data"))?;

    let mut examples = get_examples(&workspace_root);
    let total_cnt = examples.len();
    if !args.examples.is_empty() {
        examples.retain(|example| args.examples.contains(&example.name));
    }
    println!(
        "processing {}/{} examples: {:?}...",
        examples.len(),
        total_cnt,
        examples.iter().map(|e| e.name.clone()).collect::<Vec<_>>()
    );

    if args.clean {
        // TODO: clean
    }
    for example in examples {
        match args.command {
            Commands::Build => {
                example.build_wasm(&workspace_root);
            }
            Commands::Run { lazy_run } => {
                example.run(&workspace_root, lazy_run);
            }
        }
        println!("示例 {} 处理完成", example.name);
    }

    println!("所有示例处理完成");
    Ok(())
}
