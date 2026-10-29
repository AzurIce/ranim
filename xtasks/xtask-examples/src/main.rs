use std::{
    collections::HashSet,
    env,
    path::{Path, PathBuf},
    process::ExitCode,
    time::{Duration, Instant},
};

use clap::{Parser, Subcommand};
use xtask_examples::{
    CleanStats, RunStatus, build_examples_wasm, clean_website_examples, get_examples,
    publish_website_outputs,
};

#[derive(Parser)]
#[command(author, version, about = "build ranim examples")]
struct Args {
    /// Remove website artifacts for examples that no longer exist.
    #[arg(long, global = true)]
    clean: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Prepare example pages and data for Notist SSG over JSON stdio.
    Site,
    /// Build individual WASM examples for the website or rustdoc.
    Build {
        /// Output root; packages go under examples/<name>/pkg.
        #[arg(long)]
        out_dir: Option<PathBuf>,
        /// Build only these examples. Defaults to all WASM examples.
        #[arg(value_name = "EXAMPLES")]
        examples: Vec<String>,
    },
    /// Render examples, publish media and record successful render hashes.
    Run {
        /// Skip examples whose last successful render has the current source hash.
        #[arg(long)]
        lazy_run: bool,

        /// Only print errors and the final summary.
        #[arg(long, short)]
        quiet: bool,

        /// Stop after the first failed example.
        #[arg(long)]
        fail_fast: bool,

        /// Process only these examples. Defaults to all examples.
        #[arg(value_name = "EXAMPLES")]
        examples: Vec<String>,
    },
    /// Publish existing website outputs through shadow (no rendering).
    Publish,
}

fn main() -> ExitCode {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let args = Args::parse();

    if let Commands::Site = args.command {
        return match xtask_examples::site::exchange(
            std::io::stdin().lock(),
            std::io::stdout().lock(),
        ) {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("site extension IO failed: {error:#}");
                ExitCode::FAILURE
            }
        };
    }

    let xtask_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = xtask_root.join("../../");

    if let Commands::Build { examples, out_dir } = &args.command {
        return build_examples_wasm_cmd(&workspace_root, examples, out_dir.as_deref());
    }

    if let Commands::Publish = args.command {
        let all_examples = match get_examples(&workspace_root) {
            Ok(examples) => examples,
            Err(err) => {
                eprintln!("读取示例列表失败: {err:#}");
                return ExitCode::FAILURE;
            }
        };
        if args.clean {
            match clean_website_examples(&workspace_root, &all_examples) {
                Ok(stats) => print_clean_stats(stats),
                Err(err) => {
                    eprintln!("清理网站 example 输出失败: {err:#}");
                    return ExitCode::FAILURE;
                }
            }
        }
        return publish_website_outputs_cmd(&workspace_root, &all_examples);
    }

    let Commands::Run {
        lazy_run,
        quiet,
        fail_fast,
        examples: filter,
    } = &args.command
    else {
        unreachable!();
    };

    let all_examples = match get_examples(&workspace_root) {
        Ok(examples) => examples,
        Err(err) => {
            eprintln!("读取示例列表失败: {err:#}");
            return ExitCode::FAILURE;
        }
    };

    if args.clean {
        match clean_website_examples(&workspace_root, &all_examples) {
            Ok(stats) => print_clean_stats(stats),
            Err(err) => {
                eprintln!("清理网站 example 输出失败: {err:#}");
                return ExitCode::FAILURE;
            }
        }
    }

    let total_cnt = all_examples.len();
    let selected: Vec<_> = all_examples
        .into_iter()
        .filter(|example| filter.is_empty() || filter.contains(&example.name))
        .collect();

    if !filter.is_empty() {
        let known: HashSet<&str> = selected
            .iter()
            .map(|example| example.name.as_str())
            .collect();
        let mut unknown: Vec<_> = filter
            .iter()
            .filter(|name| !known.contains(name.as_str()))
            .collect();
        unknown.sort();
        unknown.dedup();
        if !unknown.is_empty() {
            eprintln!("未找到示例: {unknown:?}");
            return ExitCode::FAILURE;
        }
    }

    let selected_cnt = selected.len();
    println!("\n处理 {selected_cnt}/{total_cnt} 个示例");
    if !*quiet {
        for example in &selected {
            let group = example_group_label(example);
            println!("  - {} ({group})", example.name);
        }
        println!();
    }

    let started = Instant::now();
    let mut rendered = 0usize;
    let mut up_to_date = 0usize;
    let mut failed: Vec<(String, String)> = vec![];

    for (index, example) in selected.iter().enumerate() {
        if !*quiet {
            println!("[{:>3}/{selected_cnt}] {} …", index + 1, example.name);
        }

        let example_started = Instant::now();
        let result = example.run(&workspace_root, *lazy_run);
        let elapsed = example_started.elapsed();

        match result {
            Ok(RunStatus::Rendered) => {
                rendered += 1;
                if !*quiet {
                    println!(
                        "        {} 完成 ({}s)",
                        example.name,
                        format_duration(elapsed)
                    );
                }
            }
            Ok(RunStatus::UpToDate) => {
                up_to_date += 1;
                if !*quiet {
                    println!("        {} 已是最新，跳过", example.name);
                }
            }
            Err(err) => {
                let message = format!("{err:#}");
                eprintln!(
                    "[{}/{}] 运行示例 {} 失败 ({}s): {message}",
                    index + 1,
                    selected_cnt,
                    example.name,
                    format_duration(elapsed)
                );
                failed.push((example.name.clone(), message));
                if *fail_fast {
                    break;
                }
            }
        }
    }

    let elapsed = started.elapsed();
    println!();
    println!(
        "总计 {} 个示例，耗时 {}s",
        selected_cnt,
        format_duration(elapsed)
    );
    println!("  成功渲染: {rendered}");
    if *lazy_run {
        println!("  跳过(最新): {up_to_date}");
    }
    println!("  失败: {}", failed.len());

    if failed.is_empty() {
        ExitCode::SUCCESS
    } else {
        eprintln!();
        eprintln!("失败的示例:");
        for (name, message) in &failed {
            eprintln!("  - {name}: {message}");
        }
        ExitCode::FAILURE
    }
}

fn build_examples_wasm_cmd(
    workspace_root: &Path,
    examples: &[String],
    out_dir: Option<&Path>,
) -> ExitCode {
    let started = Instant::now();
    println!("构建 WASM examples…");
    if let Err(err) = build_examples_wasm(workspace_root, examples, out_dir) {
        eprintln!("构建 WASM examples 失败: {err:#}");
        return ExitCode::FAILURE;
    }
    println!(
        "构建 WASM examples 完成 ({:.1}s)",
        started.elapsed().as_secs_f64()
    );
    ExitCode::SUCCESS
}

fn publish_website_outputs_cmd(
    workspace_root: &Path,
    examples: &[xtask_examples::Example],
) -> ExitCode {
    let started = Instant::now();
    println!("发布网站 example 输出到 shadow…");
    if let Err(err) = publish_website_outputs(workspace_root, examples) {
        eprintln!("发布网站 example 输出失败: {err:#}");
        return ExitCode::FAILURE;
    }
    println!("发布完成 ({}s)", format_duration(started.elapsed()));
    ExitCode::SUCCESS
}

fn example_group_label(example: &xtask_examples::Example) -> String {
    if example.group.as_os_str().is_empty() {
        "examples".to_string()
    } else {
        format!("examples/{}", example.group.display())
    }
}

fn print_clean_stats(stats: CleanStats) {
    if stats.is_empty() {
        println!("没有需要清理的网站 example 输出");
        return;
    }
    println!("已清理:");
    println!("  - 渲染记录: {}", stats.renders);
    println!("  - 静态输出: {}", stats.outputs);
}

fn format_duration(duration: Duration) -> String {
    if duration.as_secs() >= 60 {
        format!("{:.1}m", duration.as_secs_f64() / 60.0)
    } else {
        format!("{:.2}", duration.as_secs_f64())
    }
}
