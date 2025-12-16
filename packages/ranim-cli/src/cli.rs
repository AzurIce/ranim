pub mod preview;
pub mod render;

use anyhow::Result;
use clap::{Args, Parser, Subcommand};

#[derive(Args, Debug, Clone, Default)]
#[group(multiple = false)]
pub struct TargetArg {
    #[arg(global = true, long, help_heading = "Cargo Target Options")]
    pub lib: bool,
    #[arg(global = true, long, help_heading = "Cargo Target Options")]
    pub example: Option<String>,
}

#[derive(Parser, Debug, Clone, Default)]
pub struct CliArgs {
    #[arg(global = true, short, long, help_heading = "Cargo Options")]
    pub package: Option<String>,

    #[arg(global = true, long, help_heading = "Cargo Options")]
    pub features: Vec<String>,

    #[command(flatten)]
    pub target: TargetArg,

    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    pub args: Vec<String>,
}

#[derive(Parser, Debug)]
#[command(name = "ranim")]
#[command(about = "A CLI tool for Ranim animation library")]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    command: Commands,

    #[command(flatten)]
    pub args: CliArgs,
}

impl Cli {
    pub fn run(self) -> Result<()> {
        let args = self.args;

        match self.command {
            Commands::Preview { scene } => {
                preview::preview_command(&args, &scene)?;
            }
            Commands::Render { scenes } => {
                render::render_command(&args, &scenes)?;
            }
        }

        Ok(())
    }
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Launch a preview app, watch the lib crate and rebuild it to dylib when it is changed
    Preview { scene: Option<String> },
    /// Build the lib crate and load it, then render it to video
    Render {
        /// Optional scene names to render (if not provided, render all scenes)
        #[arg(num_args = 0..)]
        scenes: Vec<String>,
    },
}

#[cfg(test)]
mod test {
    use crate::Target;

    use super::*;

    #[test]
    fn test_cli() {
        let parse_args = |args: &[&str]| {
            println!("parsing args {:?}", args);
            let cli = Cli::try_parse_from(args);
            println!("result: {:?}", cli);
            cli
        };
        let cli = parse_args(&["ranim", "render", "-p", "package"]).unwrap();
        let Commands::Render { scenes } = &cli.command else {
            unreachable!()
        };
        assert!(scenes.is_empty());
        assert_eq!(cli.args.package, Some("package".to_string()));
        assert!(!cli.args.target.lib);
        assert!(cli.args.target.example.is_none());

        let cli = parse_args(&["ranim", "preview", "--lib"]).unwrap();
        assert!(matches!(cli.command, Commands::Preview { scene: None }));
        assert!(cli.args.package.is_none());
        let TargetArg { lib, example } = cli.args.target.clone();
        assert!(lib);
        assert!(example.is_none());
        assert_eq!(Target::from(cli.args.target.clone()), Target::Lib);

        let cli = parse_args(&["ranim", "preview", "--example", "example"]).unwrap();
        assert!(matches!(cli.command, Commands::Preview { scene: None }));
        assert!(cli.args.package.is_none());
        let TargetArg { lib, example } = cli.args.target.clone();
        assert!(!lib);
        assert_eq!(example, Some("example".to_string()));
        assert_eq!(
            Target::from(cli.args.target.clone()),
            Target::Example("example".to_string())
        );
    }
}
