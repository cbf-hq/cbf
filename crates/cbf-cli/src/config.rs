use std::fs;
use std::path::{Path, PathBuf};

use cargo_metadata::MetadataCommand;
use serde::Deserialize;

use crate::cli::MacosBundleArgs;
use crate::error::{CliError, Result};

const BRIDGE_DYLIB_NAME: &str = "libcbf_bridge.dylib";

#[derive(Debug, Deserialize, Default)]
struct RootMetadata {
    cbf: Option<CbfMetadata>,
}

#[derive(Debug, Deserialize, Default)]
struct CbfMetadata {
    #[serde(rename = "macos-bundle")]
    macos_bundle: Option<MacosBundleMetadata>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
struct MacosBundleMetadata {
    app_name: Option<String>,
    bundle_identifier: Option<String>,
    icon: Option<PathBuf>,
    runtime_app_name: Option<String>,
    runtime_bundle_identifier: Option<String>,
    runtime_icon: Option<PathBuf>,
    category: Option<String>,
    minimum_system_version: Option<String>,
}

#[derive(Debug)]
pub struct ResolvedConfig {
    pub app_name: String,
    pub executable_name: String,
    pub executable_source_path: PathBuf,
    pub chromium_app_source_path: PathBuf,
    pub bridge_dylib_source_path: PathBuf,
    pub app_icon_source_path: Option<PathBuf>,
    pub runtime_app_name: String,
    pub runtime_bundle_identifier: String,
    pub runtime_icon_source_path: Option<PathBuf>,
    pub out_dir: PathBuf,
    pub bundle_identifier: String,
    pub bundle_version: String,
    pub short_bundle_version: String,
    pub category: Option<String>,
    pub minimum_system_version: Option<String>,
    pub codesign_identity: Option<String>,
    pub warnings: Vec<String>,
}

impl ResolvedConfig {
    pub fn resolve(args: MacosBundleArgs) -> Result<Self> {
        let metadata = MetadataCommand::new()
            .exec()
            .map_err(|source| CliError::CargoMetadata { source })?;

        let current_dir =
            std::env::current_dir().map_err(|source| CliError::CurrentDir { source })?;

        let package = resolve_package(&metadata, args.package.as_deref(), &current_dir)?;
        let root_metadata: RootMetadata = serde_json::from_value(package.metadata.clone())
            .map_err(|source| CliError::InvalidPackageMetadata {
                package: package.name.to_string(),
                source,
            })?;

        let macos_meta = root_metadata
            .cbf
            .and_then(|cbf| cbf.macos_bundle)
            .unwrap_or_default();

        let executable_source_path = args.bin_path;
        assert_file(&executable_source_path)?;

        let executable_name = executable_source_path
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or(CliError::MissingRequired {
                name: "--bin-path (file name)",
            })?
            .to_owned();

        let chromium_app_source_path = args.chromium_app.ok_or(CliError::MissingRequired {
            name: "--chromium-app or CBF_CHROMIUM_APP",
        })?;
        assert_directory(&chromium_app_source_path)?;

        let bridge_lib_dir = args.bridge_lib_dir.ok_or(CliError::MissingRequired {
            name: "--bridge-lib-dir or CBF_BRIDGE_LIB_DIR",
        })?;
        assert_directory(&bridge_lib_dir)?;

        let bridge_dylib_source_path = bridge_lib_dir.join(BRIDGE_DYLIB_NAME);
        assert_file(&bridge_dylib_source_path)?;

        let app_icon_source_path = match args.icon.or(macos_meta.icon.clone()) {
            Some(icon) => {
                let resolved =
                    resolve_path_from_manifest_dir(package.manifest_path.as_std_path(), &icon);
                assert_file(&resolved)?;
                Some(resolved)
            }
            None => None,
        };

        let out_dir = absolutize_from(&current_dir, &args.out_dir);

        let app_name = args
            .app_name
            .or(macos_meta.app_name)
            .unwrap_or_else(|| package.name.to_string());

        let mut warnings = Vec::new();

        let bundle_identifier = match args.bundle_identifier.or(macos_meta.bundle_identifier) {
            Some(value) => value,
            None => {
                let generated = generate_bundle_identifier(&package.name);
                warnings.push(format!(
                    "bundle identifier was not provided. using generated value '{generated}'"
                ));
                generated
            }
        };

        let runtime_app_name = args
            .runtime_app_name
            .or(macos_meta.runtime_app_name)
            .unwrap_or_else(|| format!("{app_name} Engine"));

        let runtime_bundle_identifier = args
            .runtime_bundle_identifier
            .or(macos_meta.runtime_bundle_identifier)
            .unwrap_or_else(|| format!("{bundle_identifier}.runtime"));

        let runtime_icon_source_path = match args.runtime_icon.or(macos_meta.runtime_icon) {
            Some(icon) => {
                let resolved =
                    resolve_path_from_manifest_dir(package.manifest_path.as_std_path(), &icon);
                assert_file(&resolved)?;
                Some(resolved)
            }
            None => app_icon_source_path.clone(),
        };

        if app_icon_source_path.is_none() {
            warnings
                .push("icon was not provided; bundle will not contain CFBundleIconFile".to_owned());
        }

        Ok(Self {
            app_name,
            executable_name,
            executable_source_path,
            chromium_app_source_path,
            bridge_dylib_source_path,
            app_icon_source_path,
            runtime_app_name,
            runtime_bundle_identifier,
            runtime_icon_source_path,
            out_dir,
            bundle_identifier,
            bundle_version: package.version.to_string(),
            short_bundle_version: package.version.to_string(),
            category: macos_meta.category,
            minimum_system_version: macos_meta.minimum_system_version,
            codesign_identity: args.codesign_identity,
            warnings,
        })
    }
}

fn resolve_package<'a>(
    metadata: &'a cargo_metadata::Metadata,
    package_name: Option<&str>,
    current_dir: &Path,
) -> Result<&'a cargo_metadata::Package> {
    if let Some(name) = package_name {
        return metadata
            .packages
            .iter()
            .find(|package| package.name == name)
            .ok_or_else(|| CliError::PackageNotFound {
                package: name.to_owned(),
            });
    }

    if let Some(root) = metadata.root_package() {
        return Ok(root);
    }

    let current_dir = fs::canonicalize(current_dir).map_err(|source| CliError::Io {
        path: current_dir.to_path_buf(),
        source,
    })?;

    let mut candidates: Vec<&cargo_metadata::Package> = metadata
        .packages
        .iter()
        .filter(|package| {
            let manifest_dir = package
                .manifest_path
                .as_std_path()
                .parent()
                .map(Path::to_path_buf);

            manifest_dir
                .and_then(|path| fs::canonicalize(path).ok())
                .is_some_and(|path| path == current_dir)
        })
        .collect();

    if candidates.len() == 1 {
        return Ok(candidates.remove(0));
    }

    if metadata.packages.len() == 1 {
        return Ok(&metadata.packages[0]);
    }

    Err(CliError::PackageResolution)
}

fn resolve_path_from_manifest_dir(manifest_path: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        return path.to_path_buf();
    }

    let manifest_dir = manifest_path.parent().unwrap_or_else(|| Path::new("."));
    manifest_dir.join(path)
}

fn absolutize_from(base: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        base.join(path)
    }
}

fn assert_file(path: &Path) -> Result<()> {
    if !path.exists() {
        return Err(CliError::PathNotFound {
            path: path.to_path_buf(),
        });
    }
    if !path.is_file() {
        return Err(CliError::NotFile {
            path: path.to_path_buf(),
        });
    }
    Ok(())
}

fn assert_directory(path: &Path) -> Result<()> {
    if !path.exists() {
        return Err(CliError::PathNotFound {
            path: path.to_path_buf(),
        });
    }
    if !path.is_dir() {
        return Err(CliError::NotDirectory {
            path: path.to_path_buf(),
        });
    }
    Ok(())
}

fn generate_bundle_identifier(package_name: &str) -> String {
    let mut sanitized = String::with_capacity(package_name.len());
    for ch in package_name.chars() {
        if ch.is_ascii_alphanumeric() || ch == '.' || ch == '-' {
            sanitized.push(ch.to_ascii_lowercase());
        } else {
            sanitized.push('-');
        }
    }

    while sanitized.contains("--") {
        sanitized = sanitized.replace("--", "-");
    }

    sanitized = sanitized.trim_matches('-').to_owned();

    if sanitized.is_empty() {
        "io.github.cbf-app".to_owned()
    } else {
        format!("io.github.{sanitized}")
    }
}

#[cfg(test)]
mod tests {
    use super::generate_bundle_identifier;

    #[test]
    fn bundle_identifier_is_sanitized() {
        assert_eq!(generate_bundle_identifier("My_App"), "io.github.my-app");
        assert_eq!(
            generate_bundle_identifier("cbf.sample"),
            "io.github.cbf.sample"
        );
        assert_eq!(generate_bundle_identifier("---"), "io.github.cbf-app");
    }

    #[test]
    fn runtime_bundle_identifier_defaults_from_host_identifier() {
        assert_eq!(
            format!("{}.runtime", "com.example.myapp"),
            "com.example.myapp.runtime"
        );
    }
}
