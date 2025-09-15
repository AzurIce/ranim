use clap::Parser;
use ranim_cli::cli::Cli;

fn main() {
    pretty_env_logger::formatted_timed_builder()
        .filter(Some("ranim_cli"), log::LevelFilter::Info)
        .filter(Some("ranim"), log::LevelFilter::Info)
        .parse_default_env()
        .init();
    // tracing_log::LogTracer::init().unwrap();
    Cli::parse().run().unwrap();
}
