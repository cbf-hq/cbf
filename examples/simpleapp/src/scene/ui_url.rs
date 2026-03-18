use std::{fs, path::PathBuf};

pub(crate) fn toolbar_ui_url() -> Result<String, String> {
    local_file_url("ui.html")
}

pub(crate) fn overlay_test_ui_url() -> Result<String, String> {
    local_file_url("overlay.html")
}

fn local_file_url(file_name: &str) -> Result<String, String> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join(file_name);
    let path = fs::canonicalize(path)
        .map_err(|err| format!("failed to resolve {file_name} path: {err}"))?;
    Ok(format!("file://{}", path.to_string_lossy()))
}
