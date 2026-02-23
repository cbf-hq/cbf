mod bundle;
mod cli;
mod config;
mod error;
mod plist;

use std::error::Error as _;
use std::process::ExitCode;

use clap::Parser;

use crate::cli::{Cli, Commands};
use crate::config::ResolvedConfig;
use crate::error::Result;

fn run() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Bundle(bundle) => match bundle.command {
            cli::BundleCommands::Macos(args) => {
                let resolved = ResolvedConfig::resolve(args)?;
                for warning in &resolved.warnings {
                    eprintln!("warning: {warning}");
                }
                let output = bundle::macos::bundle(&resolved)?;
                println!("Created bundle: {}", output.app_path.display());
                println!("Executable: {}", output.executable_path.display());
                println!("Bridge: {}", output.bridge_path.display());
                println!("Chromium: {}", output.chromium_path.display());
            }
        },
    }

    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("error: {err}");

            let mut source = err.source();
            while let Some(inner) = source {
                eprintln!("  caused by: {inner}");
                source = inner.source();
            }

            ExitCode::FAILURE
        }
    }
}
