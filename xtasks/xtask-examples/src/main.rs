use clap::{Parser, Subcommand};
use std::env;
use std::error::Error;
use std::path::Path;
use xtask_examples::get_examples;

#[derive(Parser)]
#[command(author, version, about = "build ranim examples")]
struct Args {
    /// 清除不存在的示例对应的输出文件
    #[arg(long)]
    clean: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Build {
        /// 指定要处理的示例名称，不指定则处理所有示例
        #[arg(value_name = "EXAMPLES")]
        examples: Vec<String>,
    },
    Run {
        #[arg(long)]
        lazy_run: bool,

        /// 指定要处理的示例名称，不指定则处理所有示例
        #[arg(value_name = "EXAMPLES")]
        examples: Vec<String>,
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

    let mut all_examples = get_examples(&workspace_root);
    let total_cnt = all_examples.len();

    let mut filter_examples = |example_filters: &Vec<String>| {
        if !example_filters.is_empty() {
            all_examples.retain(|example| example_filters.contains(&example.name));
        }
        println!(
            "processing {}/{} examples: {:?}...",
            all_examples.len(),
            total_cnt,
            all_examples
                .iter()
                .map(|e| e.name.clone())
                .collect::<Vec<_>>()
        );
    };

    if args.clean {
        // TODO: clean
    }
    match args.command {
        Commands::Build { examples } => {
            filter_examples(&examples);
            for example in all_examples {
                example.build_wasm(&workspace_root);
                println!("示例 {} 处理完成", example.name);
            }
        }
        Commands::Run { lazy_run, examples } => {
            filter_examples(&examples);
            for example in all_examples {
                example.run(&workspace_root, lazy_run);
                println!("示例 {} 处理完成", example.name);
            }
        }
    }

    println!("所有示例处理完成");
    Ok(())
}
