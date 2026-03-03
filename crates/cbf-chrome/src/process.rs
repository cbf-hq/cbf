use async_process::{Child, Command};
use futures_lite::future::block_on;
use std::{env, path::PathBuf, process::ExitStatus};

use cbf::{
    browser::{BrowserSession, EventStream},
    delegate::BackendDelegate,
    error::{ApiErrorKind, BackendErrorInfo, Error},
};

use crate::backend::{ChromiumBackend, ChromiumBackendOptions};

/// Resolves Chromium executable path for CBF applications.
///
/// Resolution order:
/// 1. Explicit path (for example CLI argument)
/// 2. `CBF_CHROMIUM_EXECUTABLE` environment variable
/// 3. Path relative to current app bundle:
///    `../Frameworks/Chromium.app/Contents/MacOS/Chromium`
///
/// Returns `None` when no candidate can be resolved.
pub fn resolve_chromium_executable(explicit_path: Option<PathBuf>) -> Option<PathBuf> {
    explicit_path
        .or_else(|| env::var_os("CBF_CHROMIUM_EXECUTABLE").map(PathBuf::from))
        .or_else(resolve_chromium_executable_from_bundle)
}

fn resolve_chromium_executable_from_bundle() -> Option<PathBuf> {
    let current_exe = env::current_exe().ok()?;
    let macos_dir = current_exe.parent()?;
    let contents_dir = macos_dir.parent()?;

    if contents_dir.file_name()?.to_str()? != "Contents" {
        return None;
    }

    let candidate = contents_dir
        .join("Frameworks")
        .join("Chromium.app")
        .join("Contents")
        .join("MacOS")
        .join("Chromium");

    candidate.is_file().then_some(candidate)
}

/// Runtime selection for Chromium-backed startup.
///
/// `Chrome` is the only currently supported runtime. `Alloy` is reserved as an
/// explicit future selection target, but remains unavailable in this phase.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum RuntimeSelection {
    /// Chrome-backed runtime path used by current CBF integration.
    #[default]
    Chrome,
    /// Reserved for future Alloy runtime work. Selecting this currently fails.
    Alloy,
}

impl RuntimeSelection {
    /// Stable string form for config surfaces and diagnostics.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Chrome => "chrome",
            Self::Alloy => "alloy",
        }
    }
}

impl std::fmt::Display for RuntimeSelection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for RuntimeSelection {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "chrome" => Ok(Self::Chrome),
            "alloy" => Ok(Self::Alloy),
            _ => Err(format!(
                "unsupported runtime '{value}': expected 'chrome' or 'alloy'"
            )),
        }
    }
}

fn validate_runtime_selection(runtime: RuntimeSelection) -> Result<(), Error> {
    if matches!(runtime, RuntimeSelection::Chrome) {
        return Ok(());
    }

    Err(Error::BackendFailure(BackendErrorInfo {
        kind: ApiErrorKind::Unsupported,
        operation: None,
        detail: Some(format!(
            "runtime '{}' is not available in this phase; use 'chrome'",
            runtime
        )),
    }))
}

/// Options for launching the Chromium process.
#[derive(Debug, Clone)]
pub struct ChromiumProcessOptions {
    /// Runtime path to use for startup.
    ///
    /// The default is `chrome`. `alloy` is currently reserved and will be
    /// rejected by `start_chromium` until that runtime exists.
    pub runtime: RuntimeSelection,
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
    /// Allow Chromium to create its default startup window.
    ///
    /// By default, CBF passes `--no-startup-window` to prevent Chromium's
    /// built-in initial window from being created unexpectedly.
    ///
    /// This option is intentionally marked unsafe because enabling it can
    /// interfere with CBF-controlled window lifecycle behavior.
    pub unsafe_enable_startup_default_window: bool,
    /// Additional arguments to pass to the browser process.
    pub extra_args: Vec<String>,
}

/// Combined options for launching Chromium and connecting the backend.
#[derive(Debug, Clone)]
pub struct StartChromiumOptions {
    /// Options for the Chromium child process.
    pub process: ChromiumProcessOptions,
    /// Options for backend IPC connection behavior.
    pub backend: ChromiumBackendOptions,
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
///
/// # Panics
///
/// In debug builds, panics if `options.process.channel_name` and
/// `options.backend.channel_name` do not match.
pub fn start_chromium(
    options: StartChromiumOptions,
    delegate: impl BackendDelegate,
) -> Result<
    (
        BrowserSession<ChromiumBackend>,
        EventStream<ChromiumBackend>,
        ChromiumProcess,
    ),
    Error,
> {
    let StartChromiumOptions { process, backend } = options;

    let ChromiumProcessOptions {
        runtime,
        executable_path,
        user_data_dir,
        enable_logging,
        log_file,
        v,
        vmodule,
        channel_name,
        unsafe_enable_startup_default_window,
        extra_args,
    } = process;

    validate_runtime_selection(runtime)?;

    let mut command = Command::new(&executable_path);

    command.arg("--enable-features=Cbf");

    // Set IPC channel argument
    command.arg(format!("--cbf-ipc-channel={}", channel_name));

    // Set user data dir argument if provided
    if let Some(user_data_dir) = &user_data_dir {
        command.arg(format!("--user-data-dir={}", user_data_dir));
    }

    // Set logging arguments
    if let Some(enable_logging) = enable_logging {
        command.arg(format!("--enable-logging={}", enable_logging));
    }

    if let Some(log_file) = &log_file {
        command.arg(format!("--log-file={}", log_file));
    }

    if let Some(v) = v {
        command.arg(format!("--v={}", v));
    }

    if let Some(vmodule) = &vmodule {
        command.arg(format!("--vmodule={}", vmodule));
    }

    if !unsafe_enable_startup_default_window {
        command.arg("--no-startup-window");
    }

    // Add extra arguments
    command.args(&extra_args);

    // Spawn the process
    let child = command.spawn().map_err(Error::ProcessSpawnError)?;

    // Connect to the backend
    debug_assert_eq!(
        backend.channel_name, channel_name,
        "process.channel_name and backend.channel_name must match"
    );

    let backend = ChromiumBackend::new(backend);
    let (session, events) = BrowserSession::connect(backend, delegate, None)?;

    Ok((session, events, ChromiumProcess { child }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_selection_defaults_to_chrome() {
        assert_eq!(RuntimeSelection::default(), RuntimeSelection::Chrome);
        assert_eq!(RuntimeSelection::default().to_string(), "chrome");
    }

    #[test]
    fn runtime_selection_rejects_alloy_until_implemented() {
        let err = validate_runtime_selection(RuntimeSelection::Alloy).unwrap_err();

        match err {
            Error::BackendFailure(info) => {
                assert_eq!(info.kind, ApiErrorKind::Unsupported);
                assert_eq!(
                    info.detail.as_deref(),
                    Some("runtime 'alloy' is not available in this phase; use 'chrome'")
                );
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[test]
    fn runtime_selection_parses_known_values() {
        assert_eq!("chrome".parse(), Ok(RuntimeSelection::Chrome));
        assert_eq!("alloy".parse(), Ok(RuntimeSelection::Alloy));
    }
}
