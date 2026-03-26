//! Runtime-loaded bridge access for `cbf_bridge`.

use std::{
    env,
    path::{Path, PathBuf},
    sync::OnceLock,
};

use libloading::{Library, Symbol};

#[cfg(target_os = "macos")]
const BRIDGE_LIB_FILE_NAME: &str = "libcbf_bridge.dylib";
#[cfg(target_os = "linux")]
const BRIDGE_LIB_FILE_NAME: &str = "libcbf_bridge.so";
#[cfg(target_os = "windows")]
const BRIDGE_LIB_FILE_NAME: &str = "cbf_bridge.dll";

#[derive(Debug, Clone, Default)]
pub struct BridgeLoadOptions {
    pub explicit_library_path: Option<PathBuf>,
    pub explicit_library_dir: Option<PathBuf>,
}

pub struct BridgeLibrary {
    library_path: PathBuf,
    library: Library,
}

#[derive(Debug, thiserror::Error, Clone)]
pub enum BridgeLoadError {
    #[error("failed to resolve cbf bridge library path")]
    PathNotFound,
    #[error("failed to load cbf bridge library from {path}: {source}")]
    OpenLibrary {
        path: PathBuf,
        #[source]
        source: ArcLibloadingError,
    },
}

#[derive(Debug, Clone)]
pub struct ArcLibloadingError(std::sync::Arc<libloading::Error>);

impl std::fmt::Display for ArcLibloadingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl std::error::Error for ArcLibloadingError {}

impl std::fmt::Debug for BridgeLibrary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BridgeLibrary")
            .field("library_path", &self.library_path)
            .finish_non_exhaustive()
    }
}

impl BridgeLoadOptions {
    pub fn with_explicit_library_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.explicit_library_path = Some(path.into());
        self
    }

    pub fn with_explicit_library_dir(mut self, path: impl Into<PathBuf>) -> Self {
        self.explicit_library_dir = Some(path.into());
        self
    }
}

impl BridgeLibrary {
    pub fn load(options: &BridgeLoadOptions) -> Result<Self, BridgeLoadError> {
        let library_path = resolve_bridge_library_path(options)?;
        let library = unsafe { Library::new(&library_path) }.map_err(|source| {
            BridgeLoadError::OpenLibrary {
                path: library_path.clone(),
                source: ArcLibloadingError(std::sync::Arc::new(source)),
            }
        })?;
        Ok(Self {
            library_path,
            library,
        })
    }

    pub fn library_path(&self) -> &Path {
        &self.library_path
    }

    pub(crate) unsafe fn get<T>(
        &self,
        symbol_name: &'static [u8],
    ) -> Result<Symbol<'_, T>, libloading::Error> {
        unsafe { self.library.get(symbol_name) }
    }
}

pub fn bridge() -> Result<&'static BridgeLibrary, BridgeLoadError> {
    static BRIDGE: OnceLock<BridgeLibrary> = OnceLock::new();

    if let Some(bridge) = BRIDGE.get() {
        return Ok(bridge);
    }

    let loaded = BridgeLibrary::load(&BridgeLoadOptions::default())?;
    let _ = BRIDGE.set(loaded);
    Ok(BRIDGE.get().expect("bridge library set"))
}

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
