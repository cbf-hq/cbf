use super::embedded_assets::{APP_HOST, APP_SCHEME};

pub(crate) fn toolbar_ui_url() -> Result<String, String> {
    custom_scheme_url("ui.html")
}

pub(crate) fn overlay_test_ui_url(passthrough_only_overlay_region: bool) -> Result<String, String> {
    let mode = if passthrough_only_overlay_region {
        "passthrough-listed-regions"
    } else {
        "consume-listed-regions"
    };
    Ok(format!(
        "{}?hit-test-mode={mode}",
        custom_scheme_url("overlay.html")?
    ))
}

fn custom_scheme_url(file_name: &str) -> Result<String, String> {
    Ok(format!("{APP_SCHEME}://{APP_HOST}/{file_name}"))
}
