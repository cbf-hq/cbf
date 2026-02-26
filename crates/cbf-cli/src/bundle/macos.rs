use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::config::ResolvedConfig;
use crate::error::{CliError, Result};
use crate::plist::InfoPlist;

const FRAMEWORKS_RPATH: &str = "@executable_path/../Frameworks";

#[derive(Debug)]
pub struct BundleOutput {
    pub app_path: PathBuf,
    pub executable_path: PathBuf,
    pub bridge_path: PathBuf,
    pub chromium_path: PathBuf,
}

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
    let resources_dir = contents_dir.join("Resources");

    create_dir(&macos_dir)?;
    create_dir(&frameworks_dir)?;
    create_dir(&resources_dir)?;

    let executable_path = macos_dir.join(&config.executable_name);
    copy_file(&config.executable_source_path, &executable_path)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let mut permissions = fs::metadata(&executable_path)
            .map_err(|source| CliError::Io {
                path: executable_path.clone(),
                source,
            })?
            .permissions();

        permissions.set_mode(0o755);
        fs::set_permissions(&executable_path, permissions).map_err(|source| CliError::Io {
            path: executable_path.clone(),
            source,
        })?;
    }

    let bridge_path = frameworks_dir.join("libcbf_bridge.dylib");
    copy_file(&config.bridge_dylib_source_path, &bridge_path)?;

    let chromium_path = frameworks_dir.join("Chromium.app");
    copy_dir_recursive(&config.chromium_app_source_path, &chromium_path)?;

    let icon_file_name = config
        .icon_source_path
        .as_ref()
        .map(|icon| icon.file_name().unwrap_or_else(|| OsStr::new("icon.icns")))
        .map(|name| name.to_string_lossy().to_string());

    if let Some(icon_src) = &config.icon_source_path {
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
    }

    Ok(BundleOutput {
        app_path,
        executable_path,
        bridge_path,
        chromium_path,
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
