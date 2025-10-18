use clap::Parser;
use ranim_cli::cli::Cli;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

fn build_filter() -> EnvFilter {
    const DEFAULT_DIRECTIVES: &[(&str, LevelFilter)] = &[
        ("ranim_cli", LevelFilter::INFO),
        ("ranim", LevelFilter::INFO),
    ];
    let mut filter = EnvFilter::from_default_env();
    let env = std::env::var("RUST_LOG").unwrap_or_default();
    for (name, level) in DEFAULT_DIRECTIVES
        .iter()
        .filter(|(name, _)| !env.contains(name))
    {
        filter = filter.add_directive(format!("{name}={level}").parse().unwrap());
    }
    filter
}

fn main() {
    let indicatif_layer = tracing_indicatif::IndicatifLayer::new();

    tracing_subscriber::registry()
        .with(fmt::layer().with_writer(indicatif_layer.get_stderr_writer()))
        .with(indicatif_layer)
        .with(build_filter())
        .init();

    Cli::parse().run().unwrap();
}
