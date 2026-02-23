use std::path::PathBuf;

use thiserror::Error;

pub type Result<T> = std::result::Result<T, CliError>;

#[derive(Debug, Error)]
pub enum CliError {
    #[error("failed to load cargo metadata")]
    CargoMetadata {
        #[source]
        source: cargo_metadata::Error,
    },

    #[error("failed to read current directory")]
    CurrentDir {
        #[source]
        source: std::io::Error,
    },

    #[error("package could not be resolved. pass --package")]
    PackageResolution,

    #[error("package '{package}' was not found in workspace")]
    PackageNotFound { package: String },

    #[error("invalid package metadata for {package}")]
    InvalidPackageMetadata {
        package: String,
        #[source]
        source: serde_json::Error,
    },

    #[error("missing required value: {name}")]
    MissingRequired { name: &'static str },

    #[error("path does not exist: {path}")]
    PathNotFound { path: PathBuf },

    #[error("expected file path but got non-file: {path}")]
    NotFile { path: PathBuf },

    #[error("expected directory path but got non-directory: {path}")]
    NotDirectory { path: PathBuf },

    #[error("I/O failed for path {path}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("failed to serialize Info.plist")]
    Plist {
        #[source]
        source: plist::Error,
    },

    #[error("command failed: {program}")]
    CommandFailed {
        program: &'static str,
        stderr: String,
    },

    #[error("command execution failed: {program}")]
    CommandExec {
        program: &'static str,
        #[source]
        source: std::io::Error,
    },

    #[error("invalid utf-8 from command output")]
    Utf8 {
        #[source]
        source: std::string::FromUtf8Error,
    },
}
