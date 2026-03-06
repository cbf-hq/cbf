pub mod browsing_context_open;
pub mod context_menu;
pub mod drag;
pub mod extension;
pub mod ids;
pub mod ime;
pub mod input;
pub mod lifecycle;
pub mod mouse;
pub mod profile;
pub mod prompt_ui;
pub mod surface;
pub mod tab_open;
pub mod window_open;

// Chrome-specific API policy:
// Keep raw/internal chrome vocabulary (`ChromeCommand` / `IpcEvent` / `ChromeEvent`)
// independent from direct `cbf::...` imports by routing generic-model references
// through `cbf-chrome::data::*` type aliases.
//
// Aliases are intentionally split by domain file (not a single `generic.rs`) so
// future Chromium-specific model additions can be localized without introducing
// broad import-path churn across unrelated domains.
//
// This policy was established by issue #59.
