//! Chrome-specific data types for IPC events and commands, with conversions to/from generic `cbf` data types.

pub mod background;
pub mod browsing_context_open;
pub mod choice_menu;
pub mod context_menu;
pub mod custom_scheme;
pub mod download;
pub mod drag;
pub mod extension;
pub mod ids;
pub mod ime;
pub mod input;
pub mod ipc;
pub mod lifecycle;
pub mod mouse;
pub mod profile;
pub mod prompt_ui;
pub mod surface;
pub mod tab_open;
pub mod visibility;
pub mod window_open;

// Chrome-specific API policy:
// Keep raw/internal chrome vocabulary (`ChromeCommand` / `IpcEvent` / `ChromeEvent`)
// independent from direct `cbf::...` imports.
// This policy was established by issue #59.
