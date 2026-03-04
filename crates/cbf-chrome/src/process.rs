use async_process::{Child, Command};
use cbf_chrome_sys::ffi::{cbf_bridge_client_create, cbf_bridge_client_destroy};
use futures_lite::future::block_on;
use std::{env, path::PathBuf, process::ExitStatus};

use cbf::{
    browser::{BrowserSession, EventStream},
    delegate::BackendDelegate,
    error::{ApiErrorKind, BackendErrorInfo, Error},
};

use crate::{
    backend::{ChromiumBackend, ChromiumBackendOptions},
    ffi::IpcClient,
};

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

/// Launches the Chromium process and connects to it via an inherited Mojo endpoint.
///
/// This function prepares the IPC channel, spawns the browser process with the
/// channel handle argument, completes the Mojo connection, and authenticates with
/// a freshly generated per-session token before returning a ready backend session.
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
        unsafe_enable_startup_default_window,
        extra_args,
    } = process;

    validate_runtime_selection(runtime)?;

    // Create the bridge client handle and prepare the Mojo channel pair.
    let inner = unsafe { cbf_bridge_client_create() };
    if inner.is_null() {
        return Err(Error::BackendFailure(cbf::error::BackendErrorInfo {
            kind: cbf::error::ApiErrorKind::ConnectTimeout,
            operation: None,
            detail: Some("cbf_bridge_client_create returned null".to_owned()),
        }));
    }

    let (remote_fd, switch_arg) = IpcClient::prepare_channel().map_err(|_| {
        unsafe { cbf_bridge_client_destroy(inner) };
        Error::BackendFailure(cbf::error::BackendErrorInfo {
            kind: cbf::error::ApiErrorKind::ConnectTimeout,
            operation: None,
            detail: Some("prepare_channel failed".to_owned()),
        })
    })?;

    // Generate a per-session token.
    let mut token_bytes = [0u8; 32];
    getrandom::fill(&mut token_bytes).map_err(|_| {
        Error::BackendFailure(cbf::error::BackendErrorInfo {
            kind: cbf::error::ApiErrorKind::ConnectTimeout,
            operation: None,
            detail: Some("token generation failed".to_owned()),
        })
    })?;
    let session_token: String = token_bytes.iter().map(|b| format!("{b:02x}")).collect();

    let mut command = Command::new(&executable_path);

    command.arg("--enable-features=Cbf");
    command.arg(&switch_arg);
    command.arg(format!("--cbf-session-token={session_token}"));

    // Clear FD_CLOEXEC on the remote endpoint fd so it is inherited by the child.
    #[cfg(unix)]
    {
        use std::os::unix::io::RawFd;
        if remote_fd >= 0 {
            unsafe { libc::fcntl(remote_fd as RawFd, libc::F_SETFD, 0) };
        }
    }

    if let Some(user_data_dir) = &user_data_dir {
        command.arg(format!("--user-data-dir={}", user_data_dir));
    }

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

    command.args(&extra_args);

    let child = command.spawn().map_err(Error::ProcessSpawnError)?;

    // Notify the bridge of the child PID: on macOS this registers the Mach
    // port with the rendezvous server; on other platforms it is bookkeeping.
    IpcClient::pass_child_pid(child.id());

    // Close the parent's copy of the remote fd after spawning.
    #[cfg(unix)]
    {
        use std::os::unix::io::RawFd;
        if remote_fd >= 0 {
            unsafe { libc::close(remote_fd as RawFd) };
        }
    }

    // Complete the Mojo handshake: send the OutgoingInvitation and bind the remote.
    let client = IpcClient::connect_inherited(inner).map_err(|_| {
        Error::BackendFailure(cbf::error::BackendErrorInfo {
            kind: cbf::error::ApiErrorKind::ConnectTimeout,
            operation: None,
            detail: Some("connect_inherited failed".to_owned()),
        })
    })?;

    // Authenticate and set up the browser observer.
    client.authenticate(&session_token).map_err(|_| {
        Error::BackendFailure(cbf::error::BackendErrorInfo {
            kind: cbf::error::ApiErrorKind::ConnectTimeout,
            operation: None,
            detail: Some("authenticate failed".to_owned()),
        })
    })?;

    let backend = ChromiumBackend::new(backend, client);
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
