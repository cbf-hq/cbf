use async_process::{Child, Command};
use cbf_chrome_sys::ffi::{cbf_bridge_client_create, cbf_bridge_client_destroy};
use futures_lite::future::block_on;
use signal_hook::iterator::Signals;
use std::{
    collections::BTreeSet,
    env,
    path::PathBuf,
    process::ExitStatus,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU8, AtomicU64, Ordering},
    },
    thread,
    time::{Duration, Instant},
};
use thiserror::Error;

use cbf::{
    browser::{BrowserHandle, BrowserSession, EventStream},
    delegate::BackendDelegate,
    error::{ApiErrorKind, BackendErrorInfo, Error as CbfError},
};

use crate::{
    backend::{ChromiumBackend, ChromiumBackendOptions},
    data::custom_scheme::ChromeCustomSchemeRegistration,
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

fn validate_runtime_selection(runtime: RuntimeSelection) -> Result<(), CbfError> {
    if matches!(runtime, RuntimeSelection::Chrome) {
        return Ok(());
    }

    Err(CbfError::BackendFailure(BackendErrorInfo {
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

fn build_custom_schemes_switch_value(
    registrations: &[ChromeCustomSchemeRegistration],
) -> Result<Option<String>, CbfError> {
    let mut scheme_names = BTreeSet::new();
    for registration in registrations {
        let scheme = registration.scheme.trim().to_ascii_lowercase();
        if scheme.is_empty() {
            return Err(CbfError::BackendFailure(BackendErrorInfo {
                kind: ApiErrorKind::InvalidInput,
                operation: None,
                detail: Some("custom scheme registration contains an empty scheme".to_owned()),
            }));
        }
        scheme_names.insert(scheme);
    }

    if scheme_names.is_empty() {
        Ok(None)
    } else {
        Ok(Some(scheme_names.into_iter().collect::<Vec<_>>().join(",")))
    }
}

/// A handle to the running Chromium process.
///
/// This struct holds the `std::process::Child` and allows managing its lifecycle.
#[derive(Debug)]
pub struct ChromiumProcess {
    child: Child,
}

impl ChromiumProcess {
    const WAIT_POLL_INTERVAL: Duration = Duration::from_millis(50);

    /// Returns the process id of the browser process.
    pub fn pid(&self) -> u32 {
        self.child.id()
    }

    /// Forcefully kills the browser process.
    pub fn kill(&mut self) -> std::io::Result<()> {
        self.child.kill()
    }

    /// Requests browser process termination with `SIGTERM`.
    #[cfg(unix)]
    pub fn terminate(&self) -> std::io::Result<()> {
        send_signal(self.pid(), libc::SIGTERM)
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

    /// Polls for process exit until `timeout` elapses.
    pub fn wait_for_exit_timeout(
        &mut self,
        timeout: Duration,
    ) -> std::io::Result<Option<ExitStatus>> {
        let deadline = Instant::now() + timeout;
        loop {
            if let Some(status) = self.try_wait()? {
                return Ok(Some(status));
            }
            if Instant::now() >= deadline {
                return Ok(None);
            }
            thread::sleep(
                Self::WAIT_POLL_INTERVAL.min(deadline.saturating_duration_since(Instant::now())),
            );
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShutdownMode {
    Graceful,
    Force,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChromiumRuntimeShutdownState {
    Idle,
    Graceful,
    Force,
}

impl ChromiumRuntimeShutdownState {
    fn as_u8(self) -> u8 {
        match self {
            Self::Idle => 0,
            Self::Graceful => 1,
            Self::Force => 2,
        }
    }

    fn from_u8(value: u8) -> Self {
        match value {
            1 => Self::Graceful,
            2 => Self::Force,
            _ => Self::Idle,
        }
    }

    fn from_mode(mode: ShutdownMode) -> Self {
        match mode {
            ShutdownMode::Graceful => Self::Graceful,
            ShutdownMode::Force => Self::Force,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ChromiumRuntimeShutdownStateReader {
    state: Arc<AtomicU8>,
}

impl ChromiumRuntimeShutdownStateReader {
    pub fn shutdown_state(&self) -> ChromiumRuntimeShutdownState {
        ChromiumRuntimeShutdownState::from_u8(self.state.load(Ordering::Acquire))
    }
}

#[derive(Debug, Error)]
pub enum InstallSignalHandlersError {
    #[error("signal handlers are already installed for a Chromium runtime")]
    AlreadyInstalled,
    #[error("failed to install signal handlers: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug)]
struct ShutdownController {
    browser: BrowserHandle<ChromiumBackend>,
    pid: u32,
    next_shutdown_request_id: AtomicU64,
    shutdown_state: Arc<AtomicU8>,
    shutdown_started: AtomicBool,
}

impl ShutdownController {
    const FORCE_WAIT_TIMEOUT: Duration = Duration::from_secs(3);
    const TERM_WAIT_TIMEOUT: Duration = Duration::from_secs(1);

    fn new(browser: BrowserHandle<ChromiumBackend>, pid: u32) -> Self {
        Self {
            browser,
            pid,
            next_shutdown_request_id: AtomicU64::new(1),
            shutdown_state: Arc::new(AtomicU8::new(ChromiumRuntimeShutdownState::Idle.as_u8())),
            shutdown_started: AtomicBool::new(false),
        }
    }

    fn begin_shutdown(&self, mode: ShutdownMode) -> bool {
        if self
            .shutdown_started
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
        {
            self.shutdown_state.store(
                ChromiumRuntimeShutdownState::from_mode(mode).as_u8(),
                Ordering::Release,
            );
            true
        } else {
            false
        }
    }

    fn shutdown_state(&self) -> ChromiumRuntimeShutdownState {
        ChromiumRuntimeShutdownState::from_u8(self.shutdown_state.load(Ordering::Acquire))
    }

    fn shutdown_state_reader(&self) -> ChromiumRuntimeShutdownStateReader {
        ChromiumRuntimeShutdownStateReader {
            state: Arc::clone(&self.shutdown_state),
        }
    }

    fn shutdown_via_pid(&self, mode: ShutdownMode) {
        if !self.begin_shutdown(mode) {
            return;
        }

        _ = match mode {
            ShutdownMode::Graceful => self.browser.request_shutdown(
                self.next_shutdown_request_id
                    .fetch_add(1, Ordering::Relaxed),
            ),
            ShutdownMode::Force => self.browser.force_shutdown(),
        };

        if wait_for_pid_exit(self.pid, Self::FORCE_WAIT_TIMEOUT) {
            return;
        }

        #[cfg(unix)]
        {
            let _ = send_signal(self.pid, libc::SIGTERM);
        }

        if wait_for_pid_exit(self.pid, Self::TERM_WAIT_TIMEOUT) {
            return;
        }

        #[cfg(unix)]
        {
            let _ = send_signal(self.pid, libc::SIGKILL);
        }
    }
}

static SIGNAL_HANDLERS_INSTALLED: AtomicBool = AtomicBool::new(false);

#[derive(Debug)]
pub struct ChromiumRuntime {
    session: BrowserSession<ChromiumBackend>,
    events: EventStream<ChromiumBackend>,
    process: ChromiumProcess,
    shutdown_controller: Arc<ShutdownController>,
}

impl ChromiumRuntime {
    pub fn new(
        session: BrowserSession<ChromiumBackend>,
        events: EventStream<ChromiumBackend>,
        process: ChromiumProcess,
    ) -> Self {
        let shutdown_controller =
            Arc::new(ShutdownController::new(session.handle(), process.pid()));
        Self {
            session,
            events,
            process,
            shutdown_controller,
        }
    }

    pub fn session(&self) -> &BrowserSession<ChromiumBackend> {
        &self.session
    }

    pub fn events(&self) -> EventStream<ChromiumBackend> {
        self.events.clone()
    }

    pub fn process(&self) -> &ChromiumProcess {
        &self.process
    }

    pub fn process_mut(&mut self) -> &mut ChromiumProcess {
        &mut self.process
    }

    pub fn shutdown_state(&self) -> ChromiumRuntimeShutdownState {
        self.shutdown_controller.shutdown_state()
    }

    pub fn shutdown_state_reader(&self) -> ChromiumRuntimeShutdownStateReader {
        self.shutdown_controller.shutdown_state_reader()
    }

    pub fn install_signal_handlers(&self) -> Result<(), InstallSignalHandlersError> {
        if SIGNAL_HANDLERS_INSTALLED
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return Err(InstallSignalHandlersError::AlreadyInstalled);
        }

        let controller = Arc::clone(&self.shutdown_controller);
        let signals = Signals::new([signal_hook::consts::SIGINT, signal_hook::consts::SIGTERM])
            .inspect_err(|_| {
                SIGNAL_HANDLERS_INSTALLED.store(false, Ordering::Release);
            })?;

        thread::spawn(move || {
            let mut signals = signals;
            if signals.forever().next().is_some() {
                controller.shutdown_via_pid(ShutdownMode::Force);
            }
        });

        Ok(())
    }

    pub fn shutdown(&mut self, mode: ShutdownMode) -> std::io::Result<()> {
        if !self.shutdown_controller.begin_shutdown(mode) {
            return Ok(());
        }

        let _ = match mode {
            ShutdownMode::Graceful => self.session.close(),
            ShutdownMode::Force => self.session.force_close(),
        };

        if self
            .process
            .wait_for_exit_timeout(ShutdownController::FORCE_WAIT_TIMEOUT)?
            .is_some()
        {
            return Ok(());
        }

        #[cfg(unix)]
        self.process.terminate()?;

        if self
            .process
            .wait_for_exit_timeout(ShutdownController::TERM_WAIT_TIMEOUT)?
            .is_some()
        {
            return Ok(());
        }

        match self.process.kill() {
            Ok(()) => Ok(()),
            Err(err) if err.kind() == std::io::ErrorKind::InvalidInput => Ok(()),
            Err(err) => Err(err),
        }
    }
}

impl Drop for ChromiumRuntime {
    fn drop(&mut self) {
        let _ = self.shutdown(ShutdownMode::Force);
    }
}

#[cfg(unix)]
fn send_signal(pid: u32, signal: libc::c_int) -> std::io::Result<()> {
    let result = unsafe { libc::kill(pid as libc::pid_t, signal) };
    if result == 0 {
        return Ok(());
    }

    let err = std::io::Error::last_os_error();
    if matches!(err.raw_os_error(), Some(libc::ESRCH)) {
        return Ok(());
    }
    Err(err)
}

#[cfg(unix)]
fn process_exists(pid: u32) -> bool {
    let result = unsafe { libc::kill(pid as libc::pid_t, 0) };
    if result == 0 {
        return true;
    }

    let err = std::io::Error::last_os_error();
    !matches!(err.raw_os_error(), Some(libc::ESRCH))
}

#[cfg(not(unix))]
fn process_exists(_pid: u32) -> bool {
    false
}

fn wait_for_pid_exit(pid: u32, timeout: Duration) -> bool {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if !process_exists(pid) {
            return true;
        }
        thread::sleep(
            ChromiumProcess::WAIT_POLL_INTERVAL
                .min(deadline.saturating_duration_since(Instant::now())),
        );
    }
    !process_exists(pid)
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
    CbfError,
> {
    let StartChromiumOptions { process, backend } = options;
    let custom_schemes_switch_value =
        build_custom_schemes_switch_value(&backend.custom_scheme_registrations)?;

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
        return Err(CbfError::BackendFailure(cbf::error::BackendErrorInfo {
            kind: cbf::error::ApiErrorKind::ConnectTimeout,
            operation: None,
            detail: Some("cbf_bridge_client_create returned null".to_owned()),
        }));
    }

    let (remote_fd, switch_arg) = IpcClient::prepare_channel().map_err(|_| {
        unsafe { cbf_bridge_client_destroy(inner) };
        CbfError::BackendFailure(cbf::error::BackendErrorInfo {
            kind: cbf::error::ApiErrorKind::ConnectTimeout,
            operation: None,
            detail: Some("prepare_channel failed".to_owned()),
        })
    })?;

    // Generate a per-session token.
    let mut token_bytes = [0u8; 32];
    getrandom::fill(&mut token_bytes).map_err(|_| {
        CbfError::BackendFailure(cbf::error::BackendErrorInfo {
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
    if let Some(custom_schemes_switch_value) = custom_schemes_switch_value {
        command.arg(format!(
            "--cbf-custom-schemes={custom_schemes_switch_value}"
        ));
    }

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

    let child = command.spawn().map_err(CbfError::ProcessSpawnError)?;

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
    let client = unsafe { IpcClient::connect_inherited(inner) }.map_err(|_| {
        CbfError::BackendFailure(cbf::error::BackendErrorInfo {
            kind: cbf::error::ApiErrorKind::ConnectTimeout,
            operation: None,
            detail: Some("connect_inherited failed".to_owned()),
        })
    })?;

    // Authenticate and set up the browser observer.
    client.authenticate(&session_token).map_err(|_| {
        CbfError::BackendFailure(cbf::error::BackendErrorInfo {
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
    use crate::data::custom_scheme::ChromeCustomSchemeRegistration;
    use async_process::Command;

    #[test]
    fn runtime_selection_defaults_to_chrome() {
        assert_eq!(RuntimeSelection::default(), RuntimeSelection::Chrome);
        assert_eq!(RuntimeSelection::default().to_string(), "chrome");
    }

    #[test]
    fn runtime_selection_rejects_alloy_until_implemented() {
        let err = validate_runtime_selection(RuntimeSelection::Alloy).unwrap_err();

        match err {
            CbfError::BackendFailure(info) => {
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

    #[test]
    fn build_custom_schemes_switch_value_dedupes_and_normalizes() {
        let registrations = vec![
            ChromeCustomSchemeRegistration {
                scheme: "App".to_string(),
                host: "simpleapp".to_string(),
            },
            ChromeCustomSchemeRegistration {
                scheme: " app ".to_string(),
                host: "other".to_string(),
            },
            ChromeCustomSchemeRegistration {
                scheme: "Tool".to_string(),
                host: "simpleapp".to_string(),
            },
        ];

        let value = build_custom_schemes_switch_value(&registrations)
            .expect("switch value should be built");

        assert_eq!(value.as_deref(), Some("app,tool"));
    }

    #[test]
    fn build_custom_schemes_switch_value_rejects_empty_scheme() {
        let registrations = vec![ChromeCustomSchemeRegistration {
            scheme: "   ".to_string(),
            host: "simpleapp".to_string(),
        }];

        let err = build_custom_schemes_switch_value(&registrations).unwrap_err();

        match err {
            CbfError::BackendFailure(info) => {
                assert_eq!(info.kind, ApiErrorKind::InvalidInput);
                assert_eq!(
                    info.detail.as_deref(),
                    Some("custom scheme registration contains an empty scheme")
                );
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[cfg(unix)]
    fn spawn_sleeping_process() -> ChromiumProcess {
        let child = Command::new("sh")
            .arg("-c")
            .arg("sleep 30")
            .spawn()
            .expect("spawn sleeping process");
        ChromiumProcess { child }
    }

    #[cfg(unix)]
    #[test]
    fn wait_for_exit_timeout_returns_none_while_process_is_alive() {
        let mut process = spawn_sleeping_process();

        let result = process
            .wait_for_exit_timeout(Duration::from_millis(10))
            .unwrap();

        assert!(result.is_none());
        process.kill().unwrap();
        process.wait_blocking().unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn terminate_requests_process_exit() {
        let mut process = spawn_sleeping_process();

        process.terminate().unwrap();

        let status = process
            .wait_for_exit_timeout(Duration::from_secs(2))
            .unwrap()
            .expect("terminated process should exit");
        assert!(!status.success());
    }
}
