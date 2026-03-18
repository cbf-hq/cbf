use std::{fs, path::PathBuf};

pub(crate) fn toolbar_ui_url() -> Result<String, String> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join("ui.html");
    let path =
        fs::canonicalize(path).map_err(|err| format!("failed to resolve ui.html path: {err}"))?;
    Ok(format!("file://{}", path.to_string_lossy()))
}
