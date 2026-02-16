use async_process::{Child, Command};
use futures_lite::future::block_on;
use std::{path::PathBuf, process::ExitStatus};

use crate::{
    BrowserSession, ChromiumBackend, Error, EventStream, backend_delegate::BackendDelegate, connect,
};

/// Options for launching the Chromium process.
#[derive(Debug, Clone)]
pub struct ChromiumOptions {
    /// Path to the browser executable (e.g. "Chromium.app/Contents/MacOS/Chromium").
    pub executable_path: PathBuf,
    /// Path to the user data directory.
    /// If provided, passed as `--user-data-dir=<path>`.
    /// Prefer setting this explicitly unless you have a strong reason not to.
    /// If `None`, Chromium may use a default profile location, which can conflict
    /// with normal Chromium usage and risk profile data issues (for example,
    /// profile/schema version mismatch).
    pub user_data_dir: Option<String>,
    /// Whether to enable logging.
    /// If provided, passed as `--enable-logging=<stream>`.
    /// e.g. "--enable-logging=stderr"
    pub enable_logging: Option<String>,
    /// Path to the log file.
    /// If provided, passed as `--log-file=<path>`.
    pub log_file: Option<String>,
    /// Chromium VLOG verbosity.
    /// If provided, passed as `--v=<level>`.
    pub v: Option<i32>,
    /// Per-module VLOG verbosity.
    /// If provided, passed as `--vmodule=<pattern1=N,...>`.
    pub vmodule: Option<String>,
    /// The name of the IPC channel to use.
    /// Passed as `--cbf-ipc-channel=<name>`.
    pub channel_name: String,
    /// Additional arguments to pass to the browser process.
    pub extra_args: Vec<String>,
}

/// A handle to the running Chromium process.
///
/// This struct holds the `std::process::Child` and allows managing its lifecycle.
#[derive(Debug)]
pub struct ChromiumProcess {
    child: Child,
}

impl ChromiumProcess {
    /// Forcefully kills the browser process.
    pub fn kill(&mut self) -> std::io::Result<()> {
        self.child.kill()
    }

    /// Waits for the browser process to exit.
    pub fn wait_blocking(&mut self) -> std::io::Result<ExitStatus> {
        block_on(self.child.status())
    }

    /// Attempts to check if the browser process has exited without blocking.
    pub fn try_wait(&mut self) -> std::io::Result<Option<ExitStatus>> {
        self.child.try_status()
    }

    /// Await the browser process exit status asynchronously.
    pub async fn wait(&mut self) -> std::io::Result<ExitStatus> {
        self.child.status().await
    }
}

/// Launches the Chromium process and connects to it.
///
/// This function spawns the browser process with the specified options and
/// establishes a CBF connection.
pub fn start_chromium(
    options: ChromiumOptions,
    delegate: impl BackendDelegate,
) -> Result<(BrowserSession, EventStream, ChromiumProcess), Error> {
    let mut command = Command::new(&options.executable_path);

    command.arg("--enable-features=Cbf");

    // Set IPC channel argument
    command.arg(format!("--cbf-ipc-channel={}", options.channel_name));

    // Set user data dir argument if provided
    if let Some(user_data_dir) = &options.user_data_dir {
        command.arg(format!("--user-data-dir={}", user_data_dir));
    }

    // Set logging arguments
    if let Some(enable_logging) = options.enable_logging {
        command.arg(format!("--enable-logging={}", enable_logging));
    }

    if let Some(log_file) = &options.log_file {
        command.arg(format!("--log-file={}", log_file));
    }

    if let Some(v) = options.v {
        command.arg(format!("--v={}", v));
    }

    if let Some(vmodule) = &options.vmodule {
        command.arg(format!("--vmodule={}", vmodule));
    }

    // Add extra arguments
    command.args(&options.extra_args);

    // Spawn the process
    let child = command.spawn().map_err(Error::ProcessSpawnError)?;

    // Connect to the backend
    let backend = ChromiumBackend::new(options.channel_name);
    let (session, events) = connect(backend, delegate)?;

    Ok((session, events, ChromiumProcess { child }))
}
