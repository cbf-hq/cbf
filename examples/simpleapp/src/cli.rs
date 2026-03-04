use std::path::PathBuf;

use cbf_chrome::{
    backend::ChromiumBackendOptions,
    process::{
        resolve_chromium_executable, ChromiumProcessOptions, RuntimeSelection, StartChromiumOptions,
    },
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

    /// Runtime selection gate.
    ///
    /// `chrome` is the default and the only currently supported runtime.
    #[arg(long, default_value_t = RuntimeSelection::default(), value_parser = parse_runtime_selection)]
    pub(crate) runtime: RuntimeSelection,

    /// Enable Chromium logging to stderr.
    ///
    /// Note: this forces Chromium to log only to stderr.
    #[arg(long)]
    pub(crate) enable_logging_stderr: bool,

    /// Optional Chromium log file path.
    #[arg(long)]
    pub(crate) log_file: Option<PathBuf>,

    /// Allow Chromium to create the built-in startup default window.
    ///
    /// This is unsafe for CBF-controlled lifecycle behavior.
    #[arg(long)]
    pub(crate) unsafe_enable_startup_default_window: bool,

    /// Extra Chromium command-line argument.
    ///
    /// Repeat this option to pass multiple args.
    #[arg(long = "chromium-arg")]
    pub(crate) chromium_args: Vec<String>,
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
    let chromium_executable = resolve_chromium_executable(cli.chromium_executable.clone())
        .ok_or_else(|| {
            "missing chromium executable: pass --chromium-executable, set CBF_CHROMIUM_EXECUTABLE, or run from a bundled app"
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
            runtime: cli.runtime,
            executable_path: chromium_executable,
            user_data_dir: Some(user_data_dir.to_string_lossy().to_string()),
            enable_logging: if cli.enable_logging_stderr {
                Some("stderr".to_owned())
            } else if cli.log_file.is_some() {
                // `--enable-logging=` (empty destination) keeps file logging enabled.
                Some(String::new())
            } else {
                None
            },
            log_file: cli
                .log_file
                .as_ref()
                .map(|path| path.to_string_lossy().to_string()),
            unsafe_enable_startup_default_window: cli.unsafe_enable_startup_default_window,
            v: None,
            vmodule: None,
            extra_args: cli.chromium_args.clone(),
        },
        backend: ChromiumBackendOptions::new(),
    })
}

/// Returns the default user data directory for the application.
/// On most platforms, this is located in the local data directory under "CBF SimpleApp".
fn default_user_data_dir() -> Option<PathBuf> {
    dirs::data_local_dir().map(|base| base.join("CBF SimpleApp"))
}

fn parse_runtime_selection(value: &str) -> Result<RuntimeSelection, String> {
    value.parse()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cli_runtime_defaults_to_chrome() {
        let cli = Cli::try_parse_from(["simpleapp"]).unwrap();

        assert_eq!(cli.runtime, RuntimeSelection::Chrome);
    }

    #[test]
    fn cli_runtime_accepts_alloy_for_explicit_gating() {
        let cli = Cli::try_parse_from(["simpleapp", "--runtime", "alloy"]).unwrap();

        assert_eq!(cli.runtime, RuntimeSelection::Alloy);
    }
}
