pub mod preview;
pub mod render;

use clap::{Parser, Subcommand};

#[derive(Parser, Clone, Default)]
pub struct Args {
    #[arg(global = true, short, long, help_heading = "Cargo Options")]
    pub package: Option<String>,

    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    pub args: Vec<String>,
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
                preview::preview_command(&args);
            }
            Commands::Render { scenes } => {
                render::render_command(&args, &scenes);
            }
        }
    }
}

#[derive(Subcommand)]
pub enum Commands {
    /// Launch a preview app, watch the lib crate and rebuild it to dylib when it is changed
    Preview,
    /// Build the lib crate and load it, then render it to video
    Render {
        /// Optional scene names to render (if not provided, render all scenes)
        #[arg(num_args = 0..)]
        scenes: Vec<String>,
    },
}
