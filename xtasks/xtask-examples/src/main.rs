use clap::{Parser, Subcommand};
use std::env;
use std::path::Path;
use xtask_examples::{build_wasm_bundle, get_examples};

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
        /// 指定要处理的示例名称,不指定则处理所有示例
        #[arg(value_name = "EXAMPLES")]
        examples: Vec<String>,
    },
    Run {
        #[arg(long)]
        lazy_run: bool,

        /// 指定要处理的示例名称,不指定则处理所有示例
        #[arg(value_name = "EXAMPLES")]
        examples: Vec<String>,
    },
}

fn main() {
    ranim_cli::init_tracing();

    let args = Args::parse();

    let xtask_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = xtask_root.join("../../");

    let website_root = workspace_root.join("website");
    for dir in ["static/examples", "content/examples", "data"] {
        if let Err(err) = std::fs::create_dir_all(website_root.join(dir)) {
            eprintln!("无法创建目录 {}: {err}", website_root.join(dir).display());
            std::process::exit(1);
        }
    }

    let all_examples = match get_examples(&workspace_root) {
        Ok(examples) => examples,
        Err(err) => {
            eprintln!("读取示例列表失败: {err:#}");
            std::process::exit(1);
        }
    };
    let total_cnt = all_examples.len();
    let mut selected_examples = all_examples.clone();

    let filter = match &args.command {
        Commands::Build { examples } | Commands::Run { examples, .. } => examples,
    };
    if !filter.is_empty() {
        selected_examples.retain(|example| filter.contains(&example.name));
        let unknown: Vec<_> = filter
            .iter()
            .filter(|name| !selected_examples.iter().any(|e| &e.name == *name))
            .collect();
        if !unknown.is_empty() {
            eprintln!("未找到示例: {unknown:?}");
            std::process::exit(1);
        }
    }
    let selected_cnt = selected_examples.len();
    println!(
        "处理 {selected_cnt}/{total_cnt} 个示例: {:?}",
        selected_examples
            .iter()
            .map(|e| e.name.as_str())
            .collect::<Vec<_>>()
    );

    if args.clean {
        // TODO: clean
    }

    if matches!(args.command, Commands::Build { .. }) {
        if let Err(err) = build_wasm_bundle(&workspace_root) {
            eprintln!("构建 wasm example bundle 失败: {err:#}");
            std::process::exit(1);
        }

        // The shared bundle replaced per-example wasm packages. Remove any
        // leftovers from previous website builds.
        for example in &all_examples {
            if let Err(err) = example.clean_wasm(&workspace_root) {
                eprintln!("清理示例 {} 的旧 wasm 失败: {err:#}", example.name);
                std::process::exit(1);
            }
        }
        println!("构建 wasm example bundle 完成");
        return;
    }

    let mut succeeded = 0usize;
    let mut failed: Vec<String> = vec![];
    for (index, example) in selected_examples.iter().enumerate() {
        let result = match &args.command {
            Commands::Run { lazy_run, .. } => example.run(&workspace_root, *lazy_run),
            Commands::Build { .. } => unreachable!(),
        };
        match result {
            Ok(()) => {
                succeeded += 1;
                println!(
                    "[{}/{selected_cnt}] 运行示例 {} 完成",
                    index + 1,
                    example.name
                );
            }
            Err(err) => {
                eprintln!(
                    "[{}/{selected_cnt}] 运行示例 {} 失败: {err:#}",
                    index + 1,
                    example.name
                );
                failed.push(example.name.clone());
            }
        }
    }

    println!("完成 {succeeded}/{selected_cnt} 个示例");
    if !failed.is_empty() {
        eprintln!("失败 {} 个: {failed:?}", failed.len());
        std::process::exit(1);
    }
}
