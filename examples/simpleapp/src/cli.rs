use std::path::PathBuf;

use cbf_chrome::{
    backend::ChromiumBackendOptions,
    data::custom_scheme::ChromeCustomSchemeRegistration,
    process::{
        ChromiumProcessOptions, RuntimeSelection, StartChromiumOptions, resolve_chromium_executable,
    },
};
use clap::Parser;

use crate::scene::embedded_assets::{APP_HOST, APP_SCHEME};

#[derive(Debug, Parser)]
#[command(name = "simpleapp", about = "CBF compositor sample app")]
pub(crate) struct Cli {
    #[arg(long, default_value = "https://www.google.com")]
    pub(crate) url: String,

    #[arg(long)]
    pub(crate) test_overlay_surface: bool,

    #[arg(long)]
    pub(crate) chromium_executable: Option<PathBuf>,

    #[arg(long)]
    pub(crate) user_data_dir: Option<PathBuf>,

    #[arg(long)]
    pub(crate) download_dir: Option<PathBuf>,

    #[arg(long, default_value_t = RuntimeSelection::default(), value_parser = parse_runtime_selection)]
    pub(crate) runtime: RuntimeSelection,

    #[arg(long)]
    pub(crate) enable_logging_stderr: bool,

    #[arg(long)]
    pub(crate) log_file: Option<PathBuf>,

    #[arg(long)]
    pub(crate) unsafe_enable_startup_default_window: bool,

    #[arg(long = "chromium-arg")]
    pub(crate) chromium_args: Vec<String>,
}

pub(crate) fn parse_cli() -> Cli {
    Cli::parse()
}

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

    let mut backend = ChromiumBackendOptions::new();
    backend
        .custom_scheme_registrations
        .push(ChromeCustomSchemeRegistration {
            scheme: APP_SCHEME.to_string(),
            host: APP_HOST.to_string(),
        });

    Ok(StartChromiumOptions {
        process: ChromiumProcessOptions {
            runtime: cli.runtime,
            executable_path: chromium_executable,
            user_data_dir: Some(user_data_dir.to_string_lossy().to_string()),
            enable_logging: if cli.enable_logging_stderr {
                Some("stderr".to_owned())
            } else if cli.log_file.is_some() {
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
        backend,
    })
}

fn default_user_data_dir() -> Option<PathBuf> {
    dirs::data_local_dir().map(|base| base.join("CBF SimpleApp"))
}

fn parse_runtime_selection(value: &str) -> Result<RuntimeSelection, String> {
    value.parse()
}
