//! Runtime-loaded bridge access for `cbf_bridge`.

use std::{
    env,
    ops::Deref,
    path::{Path, PathBuf},
    sync::OnceLock,
};

use crate::ffi::CbfBridge;

#[cfg(target_os = "macos")]
const BRIDGE_LIB_FILE_NAME: &str = "libcbf_bridge.dylib";
#[cfg(target_os = "linux")]
const BRIDGE_LIB_FILE_NAME: &str = "libcbf_bridge.so";
#[cfg(target_os = "windows")]
const BRIDGE_LIB_FILE_NAME: &str = "cbf_bridge.dll";

/// Options for overriding how the bridge library is located at runtime.
#[derive(Debug, Clone, Default)]
pub struct BridgeLoadOptions {
    /// An explicit full path to the bridge library file.
    pub explicit_library_path: Option<PathBuf>,
    /// An explicit directory that contains the bridge library file.
    pub explicit_library_dir: Option<PathBuf>,
}

/// A process-wide loaded bridge API wrapper.
pub struct BridgeLibrary {
    library_path: PathBuf,
    bindings: CbfBridge,
}

#[derive(Debug, thiserror::Error, Clone)]
/// Errors that can occur while locating or opening the bridge library.
pub enum BridgeLoadError {
    /// No supported runtime search location contained the bridge library.
    #[error("failed to resolve cbf bridge library path")]
    PathNotFound,
    /// Loading the discovered bridge library file or one of its required symbols failed.
    #[error("failed to load cbf bridge library from {path}: {source}")]
    LoadLibrary {
        /// The path of the library that failed to load.
        path: PathBuf,
        #[source]
        /// The underlying dynamic loader error.
        source: ArcLibloadingError,
    },
}

/// A cloneable wrapper around `libloading::Error`.
#[derive(Debug, Clone)]
pub struct ArcLibloadingError(std::sync::Arc<libloading::Error>);

impl From<libloading::Error> for ArcLibloadingError {
    fn from(value: libloading::Error) -> Self {
        Self(std::sync::Arc::new(value))
    }
}

impl std::fmt::Display for ArcLibloadingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl std::error::Error for ArcLibloadingError {}

impl BridgeLoadOptions {
    /// Return options that load the bridge from an explicit file path.
    pub fn with_explicit_library_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.explicit_library_path = Some(path.into());
        self
    }

    /// Return options that load the bridge from an explicit directory.
    pub fn with_explicit_library_dir(mut self, path: impl Into<PathBuf>) -> Self {
        self.explicit_library_dir = Some(path.into());
        self
    }
}

impl BridgeLibrary {
    /// Load the bridge API using the provided search options.
    ///
    /// The library file path is resolved by [`resolve_bridge_library_path`].
    /// See that function for the runtime search order and fallback behavior.
    pub fn load(options: &BridgeLoadOptions) -> Result<Self, BridgeLoadError> {
        let library_path = resolve_bridge_library_path(options)?;
        let bindings = unsafe { CbfBridge::new(&library_path) }.map_err(|source| {
            BridgeLoadError::LoadLibrary {
                path: library_path.clone(),
                source: source.into(),
            }
        })?;

        Ok(Self {
            library_path,
            bindings,
        })
    }

    /// Return the resolved filesystem path of the loaded bridge library.
    pub fn library_path(&self) -> &Path {
        &self.library_path
    }
}

impl std::fmt::Debug for BridgeLibrary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BridgeApi")
            .field("library_path", &self.library_path)
            .finish_non_exhaustive()
    }
}

impl Deref for BridgeLibrary {
    type Target = CbfBridge;

    fn deref(&self) -> &Self::Target {
        &self.bindings
    }
}

/// Return the process-wide bridge API instance, loading it on first use.
pub fn bridge() -> Result<&'static BridgeLibrary, BridgeLoadError> {
    static BRIDGE: OnceLock<BridgeLibrary> = OnceLock::new();

    if let Some(bridge) = BRIDGE.get() {
        return Ok(bridge);
    }

    let loaded = BridgeLibrary::load(&BridgeLoadOptions::default())?;
    BRIDGE.set(loaded).ok();
    Ok(BRIDGE.get().expect("bridge api set"))
}

/// Resolve the bridge library path from explicit options and known runtime locations.
///
/// Path resolution proceeds in this order:
///
/// 1. If [`BridgeLoadOptions::explicit_library_path`] is set and points to an
///    existing file, return it as-is.
/// 2. If [`BridgeLoadOptions::explicit_library_dir`] is set, append the
///    platform-specific library file name and return that path when the file
///    exists.
/// 3. If the `CBF_BRIDGE_LIB_DIR` environment variable is set, append the same
///    platform-specific file name and return that path when the file exists.
/// 4. Fall back to locations derived from the current executable:
///    - a sibling file next to the executable on all platforms
///    - `Contents/Frameworks/<bridge-lib>` inside a macOS app bundle
///
/// The first existing file wins. If none of these locations contain the bridge
/// library, this function returns [`BridgeLoadError::PathNotFound`].
pub fn resolve_bridge_library_path(
    options: &BridgeLoadOptions,
) -> Result<PathBuf, BridgeLoadError> {
    if let Some(path) = options
        .explicit_library_path
        .as_ref()
        .filter(|path| path.is_file())
    {
        return Ok(path.clone());
    }

    if let Some(dir) = options.explicit_library_dir.as_ref() {
        let path = dir.join(BRIDGE_LIB_FILE_NAME);
        if path.is_file() {
            return Ok(path);
        }
    }

    if let Some(dir) = env::var_os("CBF_BRIDGE_LIB_DIR") {
        let path = PathBuf::from(dir).join(BRIDGE_LIB_FILE_NAME);
        if path.is_file() {
            return Ok(path);
        }
    }

    if let Some(path) = bridge_path_from_current_executable() {
        return Ok(path);
    }

    Err(BridgeLoadError::PathNotFound)
}

fn bridge_path_from_current_executable() -> Option<PathBuf> {
    let current_exe = env::current_exe().ok()?;
    let exe_dir = current_exe.parent()?;

    let sibling = exe_dir.join(BRIDGE_LIB_FILE_NAME);
    if sibling.is_file() {
        return Some(sibling);
    }

    #[cfg(target_os = "macos")]
    {
        let contents_dir = exe_dir.parent()?;
        if contents_dir.file_name()?.to_str()? != "Contents" {
            return None;
        }

        let frameworks = contents_dir.join("Frameworks").join(BRIDGE_LIB_FILE_NAME);
        return frameworks.is_file().then_some(frameworks);
    }

    #[allow(unreachable_code)]
    None
}
