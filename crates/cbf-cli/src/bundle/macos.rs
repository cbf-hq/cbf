use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use plist::{Dictionary, Value};

use crate::config::ResolvedConfig;
use crate::error::{CliError, Result};
use crate::plist::InfoPlist;

const FRAMEWORKS_RPATH: &str = "@executable_path/../Frameworks";
const RUNTIME_DIR_NAME: &str = "CBF Runtime";
const MAIN_RUNTIME_EXECUTABLE_NAME: &str = "Chromium";
const BRIDGE_DYLIB_NAME: &str = "libcbf_bridge.dylib";

#[derive(Debug)]
pub struct BundleOutput {
    pub app_path: PathBuf,
    pub executable_path: PathBuf,
    pub bridge_path: PathBuf,
    pub runtime_path: PathBuf,
}

struct HelperSpec {
    source_name: &'static str,
    suffix: &'static str,
    identifier_suffix: &'static str,
}

const HELPER_SPECS: [HelperSpec; 5] = [
    HelperSpec {
        source_name: "Chromium Helper",
        suffix: "Helper",
        identifier_suffix: "helper",
    },
    HelperSpec {
        source_name: "Chromium Helper (Renderer)",
        suffix: "Helper (Renderer)",
        identifier_suffix: "helper.renderer",
    },
    HelperSpec {
        source_name: "Chromium Helper (GPU)",
        suffix: "Helper (GPU)",
        identifier_suffix: "helper.gpu",
    },
    HelperSpec {
        source_name: "Chromium Helper (Plugin)",
        suffix: "Helper (Plugin)",
        identifier_suffix: "helper.plugin",
    },
    HelperSpec {
        source_name: "Chromium Helper (Alerts)",
        suffix: "Helper (Alerts)",
        identifier_suffix: "helper.alerts",
    },
];

pub fn bundle(config: &ResolvedConfig) -> Result<BundleOutput> {
    fs::create_dir_all(&config.out_dir).map_err(|source| CliError::Io {
        path: config.out_dir.clone(),
        source,
    })?;

    let app_path = config.out_dir.join(format!("{}.app", config.app_name));

    if app_path.exists() {
        eprintln!(
            "warning: output bundle already exists and will be overwritten: {}",
            app_path.display()
        );
        fs::remove_dir_all(&app_path).map_err(|source| CliError::Io {
            path: app_path.clone(),
            source,
        })?;
    }

    let contents_dir = app_path.join("Contents");
    let macos_dir = contents_dir.join("MacOS");
    let frameworks_dir = contents_dir.join("Frameworks");
    let runtime_dir = contents_dir.join(RUNTIME_DIR_NAME);
    let resources_dir = contents_dir.join("Resources");

    create_dir(&macos_dir)?;
    create_dir(&frameworks_dir)?;
    create_dir(&runtime_dir)?;
    create_dir(&resources_dir)?;

    let executable_path = macos_dir.join(&config.executable_name);
    copy_file(&config.executable_source_path, &executable_path)?;
    set_executable_permissions(&executable_path)?;

    let bridge_path = frameworks_dir.join(BRIDGE_DYLIB_NAME);
    copy_file(&config.bridge_dylib_source_path, &bridge_path)?;

    let runtime_path = runtime_dir.join(format!("{}.app", config.runtime_app_name));
    copy_dir_recursive(&config.chromium_app_source_path, &runtime_path)?;
    rebrand_runtime_bundle(&runtime_path, config)?;

    let icon_file_name = config
        .app_icon_source_path
        .as_ref()
        .map(|icon| icon.file_name().unwrap_or_else(|| OsStr::new("icon.icns")))
        .map(|name| name.to_string_lossy().to_string());

    if let Some(icon_src) = &config.app_icon_source_path {
        let icon_dst = resources_dir.join(
            icon_src
                .file_name()
                .unwrap_or_else(|| OsStr::new("icon.icns")),
        );
        copy_file(icon_src, &icon_dst)?;
    }

    let plist = InfoPlist {
        CFBundleDisplayName: config.app_name.clone(),
        CFBundleExecutable: config.executable_name.clone(),
        CFBundleIdentifier: config.bundle_identifier.clone(),
        CFBundleName: config.app_name.clone(),
        CFBundlePackageType: "APPL".to_owned(),
        CFBundleVersion: config.bundle_version.clone(),
        CFBundleShortVersionString: config.short_bundle_version.clone(),
        CFBundleIconFile: icon_file_name,
        LSApplicationCategoryType: config.category.clone(),
        LSMinimumSystemVersion: config.minimum_system_version.clone(),
    };

    let plist_path = contents_dir.join("Info.plist");
    plist::to_file_xml(&plist_path, &plist).map_err(|source| CliError::Plist { source })?;

    ensure_frameworks_rpath(&executable_path)?;

    if let Some(identity) = &config.codesign_identity {
        run_command(
            Command::new("codesign")
                .arg("--force")
                .arg("--deep")
                .arg("--sign")
                .arg(identity)
                .arg(&app_path),
            "codesign",
        )?;
        validate_signed_bundle(&app_path)?;
    }

    Ok(BundleOutput {
        app_path,
        executable_path,
        bridge_path,
        runtime_path,
    })
}

fn rebrand_runtime_bundle(runtime_path: &Path, config: &ResolvedConfig) -> Result<()> {
    let runtime_plist_path = runtime_path.join("Contents").join("Info.plist");
    let mut runtime_plist = read_plist_dictionary(&runtime_plist_path)?;

    let runtime_executable_path = runtime_path
        .join("Contents")
        .join("MacOS")
        .join(MAIN_RUNTIME_EXECUTABLE_NAME);
    let renamed_runtime_executable_path = runtime_path
        .join("Contents")
        .join("MacOS")
        .join(&config.runtime_app_name);
    rename_path(&runtime_executable_path, &renamed_runtime_executable_path)?;
    set_executable_permissions(&renamed_runtime_executable_path)?;

    set_plist_string(
        &mut runtime_plist,
        "CFBundleDisplayName",
        config.runtime_app_name.clone(),
    );
    set_plist_string(
        &mut runtime_plist,
        "CFBundleName",
        config.runtime_app_name.clone(),
    );
    set_plist_string(
        &mut runtime_plist,
        "CFBundleExecutable",
        config.runtime_app_name.clone(),
    );
    set_plist_string(
        &mut runtime_plist,
        "CFBundleIdentifier",
        config.runtime_bundle_identifier.clone(),
    );

    install_bundle_icon(
        runtime_path,
        &mut runtime_plist,
        config.runtime_icon_source_path.as_deref(),
        true,
    )?;
    write_plist_dictionary(&runtime_plist_path, runtime_plist)?;

    let helper_root = locate_helper_root(runtime_path)?;
    for spec in HELPER_SPECS {
        rebrand_helper_bundle(
            &helper_root,
            spec,
            &config.runtime_app_name,
            &config.runtime_bundle_identifier,
            config.runtime_icon_source_path.as_deref(),
        )?;
    }

    Ok(())
}

fn rebrand_helper_bundle(
    helper_root: &Path,
    spec: HelperSpec,
    runtime_app_name: &str,
    runtime_bundle_identifier: &str,
    runtime_icon_source_path: Option<&Path>,
) -> Result<()> {
    let source_app_path = helper_root.join(format!("{}.app", spec.source_name));
    if !source_app_path.exists() {
        return Ok(());
    }

    let target_name = format!("{runtime_app_name} {}", spec.suffix);
    let target_app_path = helper_root.join(format!("{}.app", target_name));
    rename_path(&source_app_path, &target_app_path)?;

    let old_executable_path = target_app_path
        .join("Contents")
        .join("MacOS")
        .join(spec.source_name);
    let new_executable_path = target_app_path
        .join("Contents")
        .join("MacOS")
        .join(&target_name);
    rename_path(&old_executable_path, &new_executable_path)?;
    set_executable_permissions(&new_executable_path)?;

    let plist_path = target_app_path.join("Contents").join("Info.plist");
    let mut plist = read_plist_dictionary(&plist_path)?;
    set_plist_string(&mut plist, "CFBundleDisplayName", target_name.clone());
    set_plist_string(&mut plist, "CFBundleName", target_name.clone());
    set_plist_string(&mut plist, "CFBundleExecutable", target_name);
    set_plist_string(
        &mut plist,
        "CFBundleIdentifier",
        format!("{runtime_bundle_identifier}.{}", spec.identifier_suffix),
    );
    install_bundle_icon(
        &target_app_path,
        &mut plist,
        runtime_icon_source_path,
        false,
    )?;
    write_plist_dictionary(&plist_path, plist)?;

    Ok(())
}

fn locate_helper_root(runtime_path: &Path) -> Result<PathBuf> {
    let versions_dir = runtime_path
        .join("Contents")
        .join("Frameworks")
        .join("Chromium Framework.framework")
        .join("Versions");

    for entry in fs::read_dir(&versions_dir).map_err(|source| CliError::Io {
        path: versions_dir.clone(),
        source,
    })? {
        let entry = entry.map_err(|source| CliError::Io {
            path: versions_dir.clone(),
            source,
        })?;
        let file_type = entry.file_type().map_err(|source| CliError::Io {
            path: versions_dir.clone(),
            source,
        })?;
        if !file_type.is_dir() {
            continue;
        }
        let path = entry.path();
        let helper_root = path.join("Helpers");
        if helper_root.is_dir() {
            return Ok(helper_root);
        }
    }

    Err(CliError::PathNotFound { path: versions_dir })
}

fn install_bundle_icon(
    bundle_path: &Path,
    plist: &mut Dictionary,
    icon_source_path: Option<&Path>,
    allow_new_icon_file: bool,
) -> Result<()> {
    let Some(icon_source_path) = icon_source_path else {
        return Ok(());
    };

    let resources_dir = bundle_path.join("Contents").join("Resources");
    if !resources_dir.is_dir() {
        return Ok(());
    }

    let existing_icon_file = plist_string(plist, "CFBundleIconFile");
    let icon_file_name = if let Some(existing_icon_file) = existing_icon_file {
        let candidate = resources_dir.join(&existing_icon_file);
        if candidate.exists() || allow_new_icon_file {
            existing_icon_file
        } else {
            return Ok(());
        }
    } else if allow_new_icon_file {
        icon_source_path
            .file_name()
            .unwrap_or_else(|| OsStr::new("app.icns"))
            .to_string_lossy()
            .to_string()
    } else {
        return Ok(());
    };

    copy_file(icon_source_path, &resources_dir.join(&icon_file_name))?;
    set_plist_string(plist, "CFBundleIconFile", icon_file_name);
    Ok(())
}

fn read_plist_dictionary(path: &Path) -> Result<Dictionary> {
    let value = Value::from_file(path).map_err(|source| CliError::Plist { source })?;
    match value {
        Value::Dictionary(dictionary) => Ok(dictionary),
        _ => Err(CliError::CommandFailed {
            program: "plist",
            stderr: format!("expected plist dictionary: {}", path.display()),
        }),
    }
}

fn write_plist_dictionary(path: &Path, dictionary: Dictionary) -> Result<()> {
    plist::to_file_xml(path, &dictionary).map_err(|source| CliError::Plist { source })
}

fn plist_string(dictionary: &Dictionary, key: &str) -> Option<String> {
    dictionary.get(key)?.as_string().map(ToOwned::to_owned)
}

fn set_plist_string(dictionary: &mut Dictionary, key: &str, value: String) {
    dictionary.insert(key.to_owned(), Value::String(value));
}

fn validate_signed_bundle(app_path: &Path) -> Result<()> {
    run_command(
        Command::new("codesign")
            .arg("--verify")
            .arg("--deep")
            .arg("--strict")
            .arg("--verbose=2")
            .arg(app_path),
        "codesign",
    )?;
    run_command(
        Command::new("spctl")
            .arg("--assess")
            .arg("--type")
            .arg("execute")
            .arg("--verbose")
            .arg(app_path),
        "spctl",
    )
}

fn rename_path(src: &Path, dst: &Path) -> Result<()> {
    fs::rename(src, dst).map_err(|source| CliError::Io {
        path: dst.to_path_buf(),
        source,
    })
}

fn ensure_frameworks_rpath(executable_path: &Path) -> Result<()> {
    let output = Command::new("otool")
        .arg("-l")
        .arg(executable_path)
        .output()
        .map_err(|source| CliError::CommandExec {
            program: "otool",
            source,
        })?;

    if !output.status.success() {
        return Err(CliError::CommandFailed {
            program: "otool",
            stderr: String::from_utf8(output.stderr).map_err(|source| CliError::Utf8 { source })?,
        });
    }

    let stdout = String::from_utf8(output.stdout).map_err(|source| CliError::Utf8 { source })?;
    if stdout.contains(FRAMEWORKS_RPATH) {
        return Ok(());
    }

    run_command(
        Command::new("install_name_tool")
            .arg("-add_rpath")
            .arg(FRAMEWORKS_RPATH)
            .arg(executable_path),
        "install_name_tool",
    )?;

    let verify = Command::new("otool")
        .arg("-l")
        .arg(executable_path)
        .output()
        .map_err(|source| CliError::CommandExec {
            program: "otool",
            source,
        })?;

    if !verify.status.success() {
        return Err(CliError::CommandFailed {
            program: "otool",
            stderr: String::from_utf8(verify.stderr).map_err(|source| CliError::Utf8 { source })?,
        });
    }

    let verify_stdout =
        String::from_utf8(verify.stdout).map_err(|source| CliError::Utf8 { source })?;
    if !verify_stdout.contains(FRAMEWORKS_RPATH) {
        return Err(CliError::CommandFailed {
            program: "install_name_tool",
            stderr: "failed to add required rpath".to_owned(),
        });
    }

    Ok(())
}

fn run_command(command: &mut Command, program: &'static str) -> Result<()> {
    let output = command
        .output()
        .map_err(|source| CliError::CommandExec { program, source })?;
    if output.status.success() {
        return Ok(());
    }

    let stderr = String::from_utf8(output.stderr).map_err(|source| CliError::Utf8 { source })?;
    Err(CliError::CommandFailed { program, stderr })
}

fn create_dir(path: &Path) -> Result<()> {
    fs::create_dir_all(path).map_err(|source| CliError::Io {
        path: path.to_path_buf(),
        source,
    })
}

fn copy_file(src: &Path, dst: &Path) -> Result<()> {
    fs::copy(src, dst).map_err(|source| CliError::Io {
        path: dst.to_path_buf(),
        source,
    })?;

    Ok(())
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<()> {
    create_dir(dst)?;

    for entry in fs::read_dir(src).map_err(|source| CliError::Io {
        path: src.to_path_buf(),
        source,
    })? {
        let entry = entry.map_err(|source| CliError::Io {
            path: src.to_path_buf(),
            source,
        })?;

        let entry_path = entry.path();
        let target = dst.join(entry.file_name());

        let metadata = fs::symlink_metadata(&entry_path).map_err(|source| CliError::Io {
            path: entry_path.clone(),
            source,
        })?;
        let file_type = metadata.file_type();

        if file_type.is_symlink() {
            let link_target = fs::read_link(&entry_path).map_err(|source| CliError::Io {
                path: entry_path.clone(),
                source,
            })?;
            create_symlink(&link_target, &target)?;
        } else if file_type.is_dir() {
            copy_dir_recursive(&entry_path, &target)?;
        } else if file_type.is_file() {
            copy_file(&entry_path, &target)?;
        } else {
            return Err(CliError::UnsupportedFileType { path: entry_path });
        }
    }

    Ok(())
}

fn set_executable_permissions(path: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let mut permissions = fs::metadata(path)
            .map_err(|source| CliError::Io {
                path: path.to_path_buf(),
                source,
            })?
            .permissions();

        permissions.set_mode(0o755);
        fs::set_permissions(path, permissions).map_err(|source| CliError::Io {
            path: path.to_path_buf(),
            source,
        })?;
    }

    Ok(())
}

fn create_symlink(source: &Path, destination: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(source, destination).map_err(|source| CliError::Io {
            path: destination.to_path_buf(),
            source,
        })?;
        Ok(())
    }

    #[cfg(not(unix))]
    {
        let _ = source;
        let _ = destination;
        unreachable!("macOS bundling is unix-only")
    }
}

#[cfg(test)]
mod tests {
    use super::{install_bundle_icon, plist_string, set_plist_string};
    use plist::{Dictionary, Value};
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_path(name: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("cbf-cli-{name}-{unique}"))
    }

    #[test]
    fn install_bundle_icon_preserves_existing_helper_icon_name() {
        let root = temp_path("helper-icon");
        let resources = root.join("Contents").join("Resources");
        fs::create_dir_all(&resources).unwrap();

        let icon_source = root.join("runtime.icns");
        fs::write(&icon_source, b"runtime-icon").unwrap();
        fs::write(resources.join("app.icns"), b"old-icon").unwrap();

        let mut plist = Dictionary::new();
        set_plist_string(&mut plist, "CFBundleIconFile", "app.icns".to_owned());

        install_bundle_icon(&root, &mut plist, Some(&icon_source), false).unwrap();

        assert_eq!(
            plist_string(&plist, "CFBundleIconFile").as_deref(),
            Some("app.icns")
        );
        assert_eq!(
            fs::read(resources.join("app.icns")).unwrap(),
            b"runtime-icon"
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn install_bundle_icon_skips_helper_without_existing_icon() {
        let root = temp_path("helper-no-icon");
        let resources = root.join("Contents").join("Resources");
        fs::create_dir_all(&resources).unwrap();

        let icon_source = root.join("runtime.icns");
        fs::write(&icon_source, b"runtime-icon").unwrap();

        let mut plist = Dictionary::new();
        plist.insert(
            "CFBundleName".to_owned(),
            Value::String("No Icon".to_owned()),
        );

        install_bundle_icon(&root, &mut plist, Some(&icon_source), false).unwrap();

        assert!(fs::read_dir(&resources).unwrap().next().is_none());
        assert!(plist_string(&plist, "CFBundleIconFile").is_none());

        let _ = fs::remove_dir_all(root);
    }
}
