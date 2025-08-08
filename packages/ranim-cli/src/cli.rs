pub mod preview;
pub mod render;

use clap::{Parser, Subcommand};

#[derive(Parser, Clone, Default)]
pub struct Args {
    #[arg(short, long, help_heading = "Cargo Options")]
    package: Option<String>,

    // #[arg(long, global = true, help_heading = "Cargo Options")]
    // pub locked: bool,
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    args: Vec<String>,
}

#[derive(Parser)]
#[command(name = "ranim")]
#[command(about = "A CLI tool for Ranim animation library")]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    command: Commands,

    #[command(flatten)]
    pub args: Args,
}

impl Cli {
    pub fn run(self) {
        let args = self.args;

        match self.command {
            Commands::Preview => {
                preview::preview_command(args);
            }
            Commands::Render => {
                render::render_command(args);
            }
        }
    }
}

#[derive(Subcommand)]
pub enum Commands {
    /// Launch a preview app, watch the lib crate and rebuild it to dylib when it is changed
    Preview,
    /// Build the lib crate and load it, then render it to video
    Render,
}
