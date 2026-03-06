use cbf::{
    data::{
        browsing_context_open::BrowsingContextOpenResponse,
        context_menu::ContextMenu,
        drag::{DragDrop, DragStartRequest, DragUpdate},
        extension::{AuxiliaryWindowResponse, ExtensionInfo},
        ime::{ConfirmCompositionBehavior, ImeBoundsUpdate, ImeCommitText, ImeComposition},
        mouse::MouseEvent,
        profile::ProfileInfo,
        window_open::WindowOpenResponse,
    },
    error::BackendErrorInfo,
    event::{BackendStopReason, BeforeUnloadReason},
};

pub type ChromeAuxiliaryWindowResponse = AuxiliaryWindowResponse;
pub type ChromeBackendErrorInfo = BackendErrorInfo;
pub type ChromeBackendStopReason = BackendStopReason;
pub type ChromeBeforeUnloadReason = BeforeUnloadReason;
pub type ChromeBrowsingContextOpenResponse = BrowsingContextOpenResponse;
pub type ChromeConfirmCompositionBehavior = ConfirmCompositionBehavior;
pub type ChromeContextMenu = ContextMenu;
pub type ChromeDragDrop = DragDrop;
pub type ChromeDragStartRequest = DragStartRequest;
pub type ChromeDragUpdate = DragUpdate;
pub type ChromeExtensionInfo = ExtensionInfo;
pub type ChromeImeBoundsUpdate = ImeBoundsUpdate;
pub type ChromeImeCommitText = ImeCommitText;
pub type ChromeImeComposition = ImeComposition;
pub type ChromeMouseEvent = MouseEvent;
pub type ChromeProfileInfo = ProfileInfo;
pub type ChromeWindowOpenResponse = WindowOpenResponse;
