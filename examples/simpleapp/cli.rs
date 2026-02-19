use std::{env, path::PathBuf};

use cbf_chrome::{
    chromium_backend::ChromiumBackendOptions,
    chromium_process::{ChromiumProcessOptions, StartChromiumOptions},
};
use clap::Parser;

/// Command-line interface configuration for simpleapp.
#[derive(Debug, Parser)]
#[command(name = "simpleapp", about = "CBF single-window sample app")]
pub(crate) struct Cli {
    /// Initial URL to open.
    #[arg(long, default_value = "https://www.google.com")]
    pub(crate) url: String,

    /// Path to Chromium fork executable.
    ///
    /// If omitted, CBF_CHROMIUM_EXECUTABLE is used.
    #[arg(long)]
    pub(crate) chromium_executable: Option<PathBuf>,

    /// Optional user data directory.
    #[arg(long)]
    pub(crate) user_data_dir: Option<PathBuf>,

    /// CBF IPC channel name.
    #[arg(long, default_value = "cbf-simpleapp")]
    pub(crate) channel_name: String,

    /// Enable Chromium logging to stderr.
    #[arg(long)]
    pub(crate) enable_logging_stderr: bool,

    /// Optional Chromium log file path.
    #[arg(long)]
    pub(crate) log_file: Option<PathBuf>,
}

/// Parses command-line arguments into a [`Cli`] struct.
pub(crate) fn parse_cli() -> Cli {
    Cli::parse()
}

/// Constructs [`StartChromiumOptions`] from CLI arguments.
///
/// This function resolves the Chromium executable path and user data directory,
/// either from CLI arguments or from environment variables/defaults.
pub(crate) fn chromium_options_from_cli(cli: &Cli) -> Result<StartChromiumOptions, String> {
    let chromium_executable = cli
        .chromium_executable
        .clone()
        .or_else(|| env::var_os("CBF_CHROMIUM_EXECUTABLE").map(PathBuf::from))
        .ok_or_else(|| {
            "missing chromium executable: pass --chromium-executable or set CBF_CHROMIUM_EXECUTABLE"
                .to_owned()
        })?;

    let user_data_dir = cli
        .user_data_dir
        .clone()
        .or_else(default_user_data_dir)
        .ok_or_else(|| {
            "failed to resolve user data dir: --user-data-dir is required on this platform"
                .to_owned()
        })?;

    Ok(StartChromiumOptions {
        process: ChromiumProcessOptions {
            executable_path: chromium_executable,
            user_data_dir: Some(user_data_dir.to_string_lossy().to_string()),
            enable_logging: cli.enable_logging_stderr.then_some("stderr".to_owned()),
            log_file: cli
                .log_file
                .as_ref()
                .map(|path| path.to_string_lossy().to_string()),
            channel_name: cli.channel_name.clone(),
            v: None,
            vmodule: None,
            extra_args: Vec::new(),
        },
        backend: ChromiumBackendOptions::new(cli.channel_name.clone()),
    })
}

/// Returns the default user data directory for the application.
/// On most platforms, this is located in the local data directory under "CBF SimpleApp".
fn default_user_data_dir() -> Option<PathBuf> {
    dirs::data_local_dir().map(|base| base.join("CBF SimpleApp"))
}
