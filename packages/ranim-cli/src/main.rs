use clap::Parser;
use ranim_cli::cli::Cli;

fn main() {
    tracing_subscriber::fmt::Subscriber::builder()
        .with_env_filter("ranim_cli=TRACE,ranim=TRACE")
        .init();
    // tracing_log::LogTracer::init().unwrap();
    Cli::parse().run();
}
