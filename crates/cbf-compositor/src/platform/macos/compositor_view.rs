//! macOS `NSView` compositor that hosts multiple Chromium rendering surfaces.
//!
//! `CompositorViewMac` owns one responder/view and manages one `CALayerHost`
//! per scene item so browser surfaces can be stacked inside a single native
//! view while key, mouse, IME, menu, and drag events are routed centrally.

use std::{
    cell::{Cell, RefCell},
    collections::{HashMap, HashSet},
    ffi::c_void,
    ptr::NonNull,
    rc::Rc,
};

use cbf::{
    command::BrowserCommand,
    data::{
        context_menu::{ContextMenu, ContextMenuIcon, ContextMenuItem, ContextMenuItemType},
        drag::{
            DragData, DragOperation, DragOperations, DragStartRequest, ExternalDragDrop,
            ExternalDragEnter, ExternalDragUpdate,
        },
        edit::EditAction,
        ids::BrowsingContextId,
        ime::{
            ConfirmCompositionBehavior, ImeCommitText, ImeComposition, ImeTextRange, ImeTextSpan,
            ImeTextSpanType,
        },
        key::{KeyEvent, KeyEventType},
        mouse::{MouseEvent, MouseEventType, MouseWheelEvent, PointerType},
        transient_browsing_context::{TransientImeCommitText, TransientImeComposition},
    },
};
use cbf_chrome::{
    bridge::{
        convert_nsevent_to_key_event, convert_nsevent_to_mouse_event,
        convert_nsevent_to_mouse_wheel_event, convert_nspasteboard_to_drag_data,
    },
    data::choice_menu::{
        ChromeChoiceMenu, ChromeChoiceMenuItem, ChromeChoiceMenuItemType,
        ChromeChoiceMenuSelectionMode,
    },
    platform::macos::bindings::CALayerHost,
};
use objc2::{
    AnyThread, DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send,
    rc::Retained,
    runtime::{AnyObject, Bool, NSObject, ProtocolObject},
    sel,
};
use objc2_app_kit::{
    NSApplication, NSAutoresizingMaskOptions, NSControlStateValueOff, NSControlStateValueOn,
    NSDragOperation, NSDraggingContext, NSDraggingDestination, NSDraggingInfo, NSDraggingItem,
    NSDraggingSession, NSDraggingSource, NSEvent, NSEventModifierFlags, NSEventType, NSImage,
    NSMenu, NSMenuItem, NSMenuItemBadge, NSPasteboardTypeFileURL, NSPasteboardTypeHTML,
    NSPasteboardTypeRTF, NSPasteboardTypeString, NSPasteboardTypeURL, NSPasteboardWriting,
    NSResponder, NSTextInputClient, NSTrackingArea, NSTrackingAreaOptions, NSView,
};
use objc2_core_foundation::{CGPoint, CGRect, CGSize};
use objc2_foundation::{
    NSArray, NSAttributedString, NSData, NSNotFound, NSObjectProtocol, NSPoint, NSRange,
    NSRangePointer, NSRect, NSString, NSUInteger,
};
use objc2_quartz_core::CATransaction;

use crate::{
    error::CompositorError,
    model::{CompositionItemId, SurfaceTarget},
    platform::{
        host::{PlatformInputState, PlatformSceneItem, PlatformSurfaceHandle},
        macos::{
            hit_test::{slot_hit_test_contains_point, topmost_item_at_point},
            ime::candidate_rect_for_slot,
            surface_slot::SurfaceSlot,
        },
    },
};

pub(crate) type CommandCallback = Rc<RefCell<Box<dyn FnMut(BrowserCommand)>>>;
pub(crate) type SharedInputState = Rc<RefCell<PlatformInputState>>;

const NO_MENU_ID: u64 = 0;
const NO_COMMAND_ID: i32 = i32::MIN;
const NO_CHOICE_MENU_REQUEST_ID: u64 = 0;
const NO_CHOICE_MENU_ACTION: i32 = i32::MIN;

pub(crate) struct CompositorViewMacIvars {
    command_callback: CommandCallback,
    input_state: SharedInputState,
    // Runtime scene state keyed by compositor item id. The order vector below
    // stores the current front-to-back stacking order for hit-testing and
    // layer reordering.
    slots: RefCell<HashMap<CompositionItemId, SurfaceSlot>>,
    order: RefCell<Vec<CompositionItemId>>,
    // AppKit IME state stays inside this single responder so marked text
    // handling does not need to be split across per-surface child views.
    has_marked_text: Cell<bool>,
    marked_range: Cell<NSRange>,
    selected_range: Cell<NSRange>,
    is_handling_key_down: Cell<bool>,
    ime_handled: Cell<bool>,
    suppress_key_up: Cell<bool>,
    ime_insert_expected: Cell<bool>,
    unmark_text_called: Cell<bool>,
    saw_insert_command: Cell<bool>,
    sent_char_event: Cell<bool>,
    text_to_be_inserted: RefCell<String>,
    edit_commands: RefCell<Vec<String>>,
    pending_char_event: RefCell<Option<KeyEvent>>,
    context_menu_id: Cell<u64>,
    context_menu_selected_command_id: Cell<i32>,
    choice_menu_request_id: Cell<u64>,
    choice_menu_selected_action: Cell<i32>,
    active_drag_source: RefCell<Option<Retained<NSObject>>>,
    external_drag_state: RefCell<Option<ExternalDragSessionState>>,
}

struct HostDragSourceIvars {
    view: Retained<CompositorViewMac>,
    item_id: CompositionItemId,
    browsing_context_id: cbf::data::ids::BrowsingContextId,
    session_id: u64,
    allowed_operations: DragOperations,
    operation_mask: NSDragOperation,
}

#[derive(Debug, Clone)]
struct ExternalDragSessionState {
    item_id: CompositionItemId,
    browsing_context_id: BrowsingContextId,
    data: DragData,
    allowed_operations: DragOperations,
    operation: DragOperation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HoverDispatch {
    Leave(CompositionItemId),
    Enter(CompositionItemId),
    Move(CompositionItemId),
}

define_class!(
    #[unsafe(super(NSView, NSResponder, NSObject))]
    #[thread_kind = objc2::MainThreadOnly]
    #[name = "CompositorViewMac"]
    #[ivars = CompositorViewMacIvars]
    pub(crate) struct CompositorViewMac;

    impl CompositorViewMac {
        #[unsafe(method(isFlipped))]
        fn is_flipped(&self) -> bool {
            false
        }

        #[unsafe(method(acceptsFirstResponder))]
        fn accepts_first_responder(&self) -> bool {
            true
        }

        #[unsafe(method(becomeFirstResponder))]
        fn become_first_responder(&self) -> bool {
            if let Some((_, target)) = self.active_target() {
                self.emit_focus(target, true);
            }
            true
        }

        #[unsafe(method(resignFirstResponder))]
        fn resign_first_responder(&self) -> bool {
            if let Some((_, target)) = self.active_target() {
                self.emit_focus(target, false);
            }
            true
        }

        #[unsafe(method(keyDown:))]
        fn key_down(&self, event: &NSEvent) {
            if self.active_target().is_none() {
                return;
            }

            // Match the previous single-surface AppKit text-input flow:
            // collect key/edit state first, let interpretKeyEvents drive
            // NSTextInputClient callbacks, then forward the residual key event
            // if IME did not fully consume it.
            let had_marked_text = self.ivars().has_marked_text.get();
            self.ivars().is_handling_key_down.set(true);
            self.ivars().ime_handled.set(false);
            self.ivars().suppress_key_up.set(false);
            self.ivars().ime_insert_expected.set(false);
            self.ivars().unmark_text_called.set(false);
            self.ivars().saw_insert_command.set(false);
            self.ivars().sent_char_event.set(false);
            self.ivars().text_to_be_inserted.borrow_mut().clear();
            self.ivars().edit_commands.borrow_mut().clear();
            self.ivars().pending_char_event.borrow_mut().take();

            self.ivars()
                .pending_char_event
                .replace(Some(self.convert_key_event(event)));

            let events = NSArray::arrayWithObject(event);
            self.interpretKeyEvents(&events);
            self.ivars().is_handling_key_down.set(false);

            let text_to_be_inserted = std::mem::take(&mut *self.ivars().text_to_be_inserted.borrow_mut());
            let text_inserted = !text_to_be_inserted.is_empty();
            let text_inserted_as_commit = text_to_be_inserted.encode_utf16().count()
                > if self.ivars().has_marked_text.get() || had_marked_text {
                    0
                } else {
                    1
                };

            if self.ivars().ime_handled.get() {
                if text_inserted {
                    self.send_inserted_text(text_to_be_inserted, text_inserted_as_commit);
                } else if had_marked_text && self.ivars().unmark_text_called.get() {
                    self.send_finish_composing(false);
                }
                self.ivars().pending_char_event.borrow_mut().take();
                self.ivars().suppress_key_up.set(true);
                return;
            }

            if had_marked_text && should_ignore_accelerator_with_marked_text(event) {
                self.ivars().pending_char_event.borrow_mut().take();
                self.ivars().suppress_key_up.set(true);
                return;
            }

            self.forward_key_event(event);
            if text_inserted {
                self.send_inserted_text(text_to_be_inserted, text_inserted_as_commit);
            } else if had_marked_text && self.ivars().unmark_text_called.get() {
                self.send_finish_composing(false);
            }
            if self.ivars().saw_insert_command.get() && !self.ivars().sent_char_event.get() {
                let pending_char_event = self.ivars().pending_char_event.borrow().clone();
                if let Some(text) = synthesized_char_text(pending_char_event.as_ref())
                    && let Some(event) = build_char_event(pending_char_event, text)
                {
                    self.send_char_event(event);
                }
            }
            self.ivars().pending_char_event.borrow_mut().take();
        }

        #[unsafe(method(keyUp:))]
        fn key_up(&self, event: &NSEvent) {
            if self.ivars().suppress_key_up.replace(false) {
                return;
            }
            if self.active_target().is_none() {
                return;
            }
            self.forward_key_event(event);
        }

        #[unsafe(method(flagsChanged:))]
        fn flags_changed(&self, event: &NSEvent) {
            if self.active_target().is_none() {
                return;
            }
            self.forward_key_event(event);
        }

        #[unsafe(method(undo:))]
        fn undo(&self, _sender: &AnyObject) {
            self.send_edit_action(EditAction::Undo);
        }

        #[unsafe(method(redo:))]
        fn redo(&self, _sender: &AnyObject) {
            self.send_edit_action(EditAction::Redo);
        }

        #[unsafe(method(cut:))]
        fn cut(&self, _sender: &AnyObject) {
            self.send_edit_action(EditAction::Cut);
        }

        #[unsafe(method(copy:))]
        fn copy(&self, _sender: &AnyObject) {
            self.send_edit_action(EditAction::Copy);
        }

        #[unsafe(method(paste:))]
        fn paste(&self, _sender: &AnyObject) {
            self.send_edit_action(EditAction::Paste);
        }

        #[unsafe(method(selectAll:))]
        fn select_all(&self, _sender: &AnyObject) {
            self.send_edit_action(EditAction::SelectAll);
        }

        #[unsafe(method(contextMenuItemSelected:))]
        fn context_menu_item_selected(&self, sender: &NSMenuItem) {
            let command_id = sender.tag();
            self.ivars()
                .context_menu_selected_command_id
                .set(command_id as i32);
        }

        #[unsafe(method(choiceMenuItemSelected:))]
        fn choice_menu_item_selected(&self, sender: &NSMenuItem) {
            let action = sender.tag();
            self.ivars().choice_menu_selected_action.set(action as i32);
        }

        #[unsafe(method(mouseDown:))]
        fn mouse_down(&self, event: &NSEvent) {
            self.ensure_first_responder();
            self.forward_mouse_event(event, MouseEventType::Down);
        }

        #[unsafe(method(rightMouseDown:))]
        fn right_mouse_down(&self, event: &NSEvent) {
            self.ensure_first_responder();
            self.forward_mouse_event(event, MouseEventType::Down);
        }

        #[unsafe(method(otherMouseDown:))]
        fn other_mouse_down(&self, event: &NSEvent) {
            self.ensure_first_responder();
            self.forward_mouse_event(event, MouseEventType::Down);
        }

        #[unsafe(method(mouseUp:))]
        fn mouse_up(&self, event: &NSEvent) {
            self.forward_mouse_event(event, MouseEventType::Up);
        }

        #[unsafe(method(rightMouseUp:))]
        fn right_mouse_up(&self, event: &NSEvent) {
            self.forward_mouse_event(event, MouseEventType::Up);
        }

        #[unsafe(method(otherMouseUp:))]
        fn other_mouse_up(&self, event: &NSEvent) {
            self.forward_mouse_event(event, MouseEventType::Up);
        }

        #[unsafe(method(mouseMoved:))]
        fn mouse_moved(&self, event: &NSEvent) {
            self.forward_mouse_event(event, MouseEventType::Move);
        }

        #[unsafe(method(mouseDragged:))]
        fn mouse_dragged(&self, event: &NSEvent) {
            self.forward_mouse_event(event, MouseEventType::Move);
        }

        #[unsafe(method(rightMouseDragged:))]
        fn right_mouse_dragged(&self, event: &NSEvent) {
            self.forward_mouse_event(event, MouseEventType::Move);
        }

        #[unsafe(method(otherMouseDragged:))]
        fn other_mouse_dragged(&self, event: &NSEvent) {
            self.forward_mouse_event(event, MouseEventType::Move);
        }

        #[unsafe(method(mouseExited:))]
        fn mouse_exited(&self, event: &NSEvent) {
            self.clear_hover_target(event);
        }

        #[unsafe(method(scrollWheel:))]
        fn scroll_wheel(&self, event: &NSEvent) {
            let Some((item_id, target)) = self.mouse_target(event, false) else {
                return;
            };

            let mut wheel_event = self.convert_mouse_wheel_event(event);
            self.translate_wheel_event(item_id, &mut wheel_event);
            self.send_mouse_wheel_event(target, wheel_event);
        }
    }

    #[allow(non_snake_case)]
    unsafe impl NSTextInputClient for CompositorViewMac {
        #[unsafe(method(insertText:replacementRange:))]
        unsafe fn insertText_replacementRange(
            &self,
            string: &AnyObject,
            replacement_range: NSRange,
        ) {
            let Some(text) = extract_insert_text(string) else {
                return;
            };

            if self.ivars().is_handling_key_down.get() && replacement_range.location == NSNotFound as usize {
                if self.ivars().ime_insert_expected.get() {
                    self.mark_ime_handled();
                    self.ivars().ime_insert_expected.set(false);
                    self.update_marked_state(false, ns_not_found_range(), ns_not_found_range());
                }
                self.ivars().text_to_be_inserted.borrow_mut().push_str(&text);
            } else if self.ivars().ime_insert_expected.get() {
                self.mark_ime_handled();
                self.ivars().ime_insert_expected.set(false);
                self.update_marked_state(false, ns_not_found_range(), ns_not_found_range());
                self.send_commit_text(
                    text,
                    nsrange_to_text_range(replacement_range),
                    0,
                );
            } else {
                self.send_commit_text(text, nsrange_to_text_range(replacement_range), 0);
            }
        }

        #[unsafe(method(doCommandBySelector:))]
        unsafe fn doCommandBySelector(&self, selector: objc2::runtime::Sel) {
            let mut command = selector.name().to_str().unwrap().to_string();
            if let Some(stripped) = command.strip_suffix(':') {
                command = stripped.to_string();
            }
            if command.to_ascii_lowercase().starts_with("insert") {
                self.ivars().saw_insert_command.set(true);
                return;
            }
            self.ivars().edit_commands.borrow_mut().push(command);
        }

        #[unsafe(method(setMarkedText:selectedRange:replacementRange:))]
        unsafe fn setMarkedText_selectedRange_replacementRange(
            &self,
            string: &AnyObject,
            selected_range: NSRange,
            replacement_range: NSRange,
        ) {
            let Some(text) = extract_insert_text(string) else {
                return;
            };
            self.mark_ime_handled();
            self.ivars().ime_insert_expected.set(true);
            let marked_range = composition_range_for_text(&text);
            self.update_marked_state(true, marked_range, selected_range);
            self.send_set_composition(
                text,
                nsrange_to_text_range(selected_range),
                nsrange_to_text_range(replacement_range),
            );
        }

        #[unsafe(method(unmarkText))]
        fn unmarkText(&self) {
            let had_marked = self.ivars().has_marked_text.get();
            self.ivars().ime_insert_expected.set(had_marked);
            self.update_marked_state(false, ns_not_found_range(), ns_not_found_range());
            if self.ivars().is_handling_key_down.get() {
                self.ivars().unmark_text_called.set(true);
            } else {
                self.send_finish_composing(false);
            }
        }

        #[unsafe(method(selectedRange))]
        fn selectedRange(&self) -> NSRange {
            self.ivars().selected_range.get()
        }

        #[unsafe(method(markedRange))]
        fn markedRange(&self) -> NSRange {
            if self.ivars().has_marked_text.get() {
                self.ivars().marked_range.get()
            } else {
                ns_not_found_range()
            }
        }

        #[unsafe(method(hasMarkedText))]
        fn hasMarkedText(&self) -> bool {
            self.ivars().has_marked_text.get()
        }

        #[unsafe(method(attributedSubstringForProposedRange:actualRange:))]
        unsafe fn attributedSubstringForProposedRange_actualRange(
            &self,
            range: NSRange,
            actual_range: NSRangePointer,
        ) -> *const AnyObject {
            if !actual_range.is_null() {
                unsafe { actual_range.write(range) };
            }
            std::ptr::null()
        }

        #[unsafe(method(validAttributesForMarkedText))]
        fn validAttributesForMarkedText(&self) -> *const AnyObject {
            let array: Retained<NSArray<NSString>> = NSArray::new();
            Retained::autorelease_return(array) as _
        }

        #[unsafe(method(firstRectForCharacterRange:actualRange:))]
        unsafe fn firstRectForCharacterRange_actualRange(
            &self,
            range: NSRange,
            actual_range: NSRangePointer,
        ) -> NSRect {
            if !actual_range.is_null() {
                unsafe { actual_range.write(range) };
            }
            self.ime_candidate_rect(range)
        }

        #[unsafe(method(characterIndexForPoint:))]
        fn characterIndexForPoint(&self, _point: NSPoint) -> NSUInteger {
            NSNotFound as NSUInteger
        }
    }

    unsafe impl NSObjectProtocol for CompositorViewMac {}

    #[allow(non_snake_case)]
    unsafe impl NSDraggingDestination for CompositorViewMac {
        #[unsafe(method(draggingEntered:))]
        fn draggingEntered(&self, sender: &ProtocolObject<dyn NSDraggingInfo>) -> NSDragOperation {
            let Some((local_point, screen_point)) = self.dragging_points(sender) else {
                return NSDragOperation::None;
            };
            let Some((item_id, browsing_context_id)) = self.drag_target_at_point(local_point) else {
                return NSDragOperation::None;
            };

            let pasteboard = sender.draggingPasteboard();
            let data =
                convert_nspasteboard_to_drag_data(NonNull::from(&*pasteboard).cast::<c_void>());
            let allowed_operations = drag_operations_from_ns(sender.draggingSourceOperationMask());

            self.begin_external_drag_session(
                item_id,
                browsing_context_id,
                data,
                allowed_operations,
                screen_point,
            );
            NSDragOperation::Copy
        }

        #[unsafe(method(draggingUpdated:))]
        fn draggingUpdated(&self, sender: &ProtocolObject<dyn NSDraggingInfo>) -> NSDragOperation {
            let Some((local_point, screen_point)) = self.dragging_points(sender) else {
                self.leave_external_drag_session();
                return NSDragOperation::None;
            };
            let next_target = self.drag_target_at_point(local_point);
            let allowed_operations = drag_operations_from_ns(sender.draggingSourceOperationMask());
            let current_state = self.ivars().external_drag_state.borrow().clone();

            match (current_state, next_target) {
                (Some(state), Some((item_id, browsing_context_id)))
                    if state.item_id == item_id
                        && state.browsing_context_id == browsing_context_id =>
                {
                    self.update_external_drag_session(
                        item_id,
                        browsing_context_id,
                        allowed_operations,
                        screen_point,
                    );
                    self.current_external_drag_operation()
                }
                (Some(state), Some((item_id, browsing_context_id))) => {
                    self.leave_external_drag_session();
                    self.begin_external_drag_session(
                        item_id,
                        browsing_context_id,
                        state.data,
                        allowed_operations,
                        screen_point,
                    );
                    NSDragOperation::Copy
                }
                (None, Some((item_id, browsing_context_id))) => {
                    let pasteboard = sender.draggingPasteboard();
                    let data = convert_nspasteboard_to_drag_data(
                        NonNull::from(&*pasteboard).cast::<c_void>(),
                    );
                    self.begin_external_drag_session(
                        item_id,
                        browsing_context_id,
                        data,
                        allowed_operations,
                        screen_point,
                    );
                    NSDragOperation::Copy
                }
                (Some(_), None) => {
                    self.leave_external_drag_session();
                    NSDragOperation::None
                }
                (None, None) => NSDragOperation::None,
            }
        }

        #[unsafe(method(draggingExited:))]
        fn draggingExited(&self, _sender: Option<&ProtocolObject<dyn NSDraggingInfo>>) {
            self.leave_external_drag_session();
        }

        #[unsafe(method(performDragOperation:))]
        fn performDragOperation(&self, sender: &ProtocolObject<dyn NSDraggingInfo>) -> Bool {
            let Some((local_point, screen_point)) = self.dragging_points(sender) else {
                self.leave_external_drag_session();
                return Bool::NO;
            };
            let next_target = self.drag_target_at_point(local_point);
            let current_state = self.ivars().external_drag_state.borrow().clone();

            match (current_state, next_target) {
                (Some(state), Some((item_id, browsing_context_id)))
                    if state.item_id == item_id
                        && state.browsing_context_id == browsing_context_id =>
                {
                    self.drop_external_drag_session(item_id, browsing_context_id, screen_point);
                    Bool::YES
                }
                (Some(state), Some((item_id, browsing_context_id))) => {
                    self.leave_external_drag_session();
                    self.begin_external_drag_session(
                        item_id,
                        browsing_context_id,
                        state.data,
                        drag_operations_from_ns(sender.draggingSourceOperationMask()),
                        screen_point,
                    );
                    self.drop_external_drag_session(item_id, browsing_context_id, screen_point);
                    Bool::YES
                }
                (Some(_), None) => {
                    self.leave_external_drag_session();
                    Bool::NO
                }
                (None, _) => Bool::NO,
            }
        }
    }
);

define_class!(
    #[unsafe(super(NSObject))]
    #[thread_kind = objc2::MainThreadOnly]
    #[ivars = HostDragSourceIvars]
    struct HostDragSource;

    unsafe impl NSObjectProtocol for HostDragSource {}

    #[allow(non_snake_case)]
    unsafe impl NSDraggingSource for HostDragSource {
        #[unsafe(method(draggingSession:sourceOperationMaskForDraggingContext:))]
        fn draggingSession_sourceOperationMaskForDraggingContext(
            &self,
            _session: &NSDraggingSession,
            _context: NSDraggingContext,
        ) -> NSDragOperation {
            self.ivars().operation_mask
        }

        #[unsafe(method(draggingSession:movedToPoint:))]
        fn draggingSession_movedToPoint(
            &self,
            _session: &NSDraggingSession,
            screen_point: NSPoint,
        ) {
            self.ivars().view.emit_native_drag_update(
                self.ivars().item_id,
                self.ivars().browsing_context_id,
                self.ivars().session_id,
                self.ivars().allowed_operations,
                screen_point,
            );
        }

        #[unsafe(method(draggingSession:endedAtPoint:operation:))]
        fn draggingSession_endedAtPoint_operation(
            &self,
            _session: &NSDraggingSession,
            screen_point: NSPoint,
            operation: NSDragOperation,
        ) {
            // AppKit owns the native drag loop and may consume the matching
            // mouse-up, so clear compositor-side capture explicitly when the
            // drag session ends.
            let mut input_state = self.ivars().view.ivars().input_state.borrow_mut();
            input_state.pointer_capture_item_id = None;
            drop(input_state);
            self.ivars().view.ivars().active_drag_source.replace(None);

            let treat_as_drop = operation != NSDragOperation::None
                || self.ivars().view.is_same_context_drag_drop_point(
                    self.ivars().browsing_context_id,
                    screen_point,
                );
            if !treat_as_drop {
                self.ivars().view.emit_native_drag_cancel(
                    self.ivars().session_id,
                    self.ivars().browsing_context_id,
                );
            } else {
                self.ivars().view.emit_native_drag_drop(
                    self.ivars().item_id,
                    self.ivars().browsing_context_id,
                    self.ivars().session_id,
                    screen_point,
                );
            }
        }
    }
);

impl HostDragSource {
    fn new(
        mtm: MainThreadMarker,
        view: Retained<CompositorViewMac>,
        item_id: CompositionItemId,
        browsing_context_id: cbf::data::ids::BrowsingContextId,
        session_id: u64,
        allowed_operations: DragOperations,
        operation_mask: NSDragOperation,
    ) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(HostDragSourceIvars {
            view,
            item_id,
            browsing_context_id,
            session_id,
            allowed_operations,
            operation_mask,
        });
        unsafe { msg_send![super(this), init] }
    }
}

impl CompositorViewMac {
    pub(crate) fn attach_to_host(
        mtm: MainThreadMarker,
        host_view: &NSView,
        frame: CGRect,
        input_state: SharedInputState,
        command_callback: CommandCallback,
    ) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(CompositorViewMacIvars {
            command_callback,
            input_state,
            slots: RefCell::new(HashMap::new()),
            order: RefCell::new(Vec::new()),
            has_marked_text: Cell::new(false),
            marked_range: Cell::new(ns_not_found_range()),
            selected_range: Cell::new(ns_not_found_range()),
            is_handling_key_down: Cell::new(false),
            ime_handled: Cell::new(false),
            suppress_key_up: Cell::new(false),
            ime_insert_expected: Cell::new(false),
            unmark_text_called: Cell::new(false),
            saw_insert_command: Cell::new(false),
            sent_char_event: Cell::new(false),
            text_to_be_inserted: RefCell::new(String::new()),
            edit_commands: RefCell::new(Vec::new()),
            pending_char_event: RefCell::new(None),
            context_menu_id: Cell::new(NO_MENU_ID),
            context_menu_selected_command_id: Cell::new(NO_COMMAND_ID),
            choice_menu_request_id: Cell::new(NO_CHOICE_MENU_REQUEST_ID),
            choice_menu_selected_action: Cell::new(NO_CHOICE_MENU_ACTION),
            active_drag_source: RefCell::new(None),
            external_drag_state: RefCell::new(None),
        });
        let this: Retained<Self> = unsafe { msg_send![super(this), init] };

        // The compositor view stretches with the content view and becomes the
        // single native entrypoint for all embedded browser surfaces.
        this.setFrame(frame);
        this.setAutoresizingMask(
            NSAutoresizingMaskOptions::ViewWidthSizable
                | NSAutoresizingMaskOptions::ViewHeightSizable,
        );
        this.setWantsLayer(true);
        this.install_tracking_area();
        let dragged_types = unsafe {
            NSArray::from_slice(&[
                NSPasteboardTypeFileURL,
                NSPasteboardTypeHTML,
                NSPasteboardTypeRTF,
                NSPasteboardTypeString,
                NSPasteboardTypeURL,
            ])
        };
        this.registerForDraggedTypes(&dragged_types);
        host_view.addSubview(&this);

        this
    }

    pub(crate) fn set_external_drag_operation(
        &self,
        target: SurfaceTarget,
        operation: DragOperation,
    ) {
        let SurfaceTarget::BrowsingContext(browsing_context_id) = target else {
            return;
        };
        if let Some(state) = self.ivars().external_drag_state.borrow_mut().as_mut()
            && state.browsing_context_id == browsing_context_id
        {
            state.operation = operation;
        }
    }

    pub(crate) fn replace_scene(&self, items: &[PlatformSceneItem]) {
        // Apply the latest scene snapshot by removing stale items, then
        // upserting the surviving/current ones before rebuilding z-order.
        let desired_ids = items
            .iter()
            .map(|item| item.item_id)
            .collect::<HashSet<_>>();
        let stale_ids = self
            .ivars()
            .slots
            .borrow()
            .keys()
            .copied()
            .filter(|item_id| !desired_ids.contains(item_id))
            .collect::<Vec<_>>();

        for item_id in stale_ids {
            self.remove_item(item_id);
        }

        let ordered_items = items.to_vec();

        for item in &ordered_items {
            self.upsert_item(item);
        }

        self.ivars()
            .order
            .replace(ordered_items.iter().map(|item| item.item_id).collect());
        self.reorder_sublayers();
    }

    pub(crate) fn show_context_menu(
        &self,
        target: SurfaceTarget,
        menu: ContextMenu,
    ) -> Result<(), crate::error::CompositorError> {
        // Menus are anchored in slot-local coordinates, so resolve the target
        // slot first and then offset the popup location into compositor space.
        let Some((item_id, slot)) = self.find_slot_for_target(target) else {
            return Err(crate::error::CompositorError::UnknownTarget);
        };
        if self.window().is_none() {
            return Ok(());
        }

        self.ivars().context_menu_id.set(menu.menu_id);
        self.ivars()
            .context_menu_selected_command_id
            .set(NO_COMMAND_ID);

        let mtm = MainThreadMarker::new().expect("CompositorViewMac must be on main thread");
        let ns_menu = build_context_ns_menu(mtm, &menu.items, self);
        let location = slot_menu_location(slot.bounds, menu.x, menu.y, self.isFlipped());
        let positioning_item = None;

        _ = ns_menu.popUpMenuPositioningItem_atLocation_inView(
            positioning_item,
            location,
            Some(self),
        );

        // AppKit may consume the matching mouse-up while the menu is open,
        // leaving compositor-side pointer capture stuck on the pre-menu item.
        self.ivars()
            .input_state
            .borrow_mut()
            .pointer_capture_item_id = None;

        let menu_id = self.ivars().context_menu_id.replace(NO_MENU_ID);
        let command_id = self
            .ivars()
            .context_menu_selected_command_id
            .replace(NO_COMMAND_ID);

        if menu_id == NO_MENU_ID {
            return Ok(());
        }

        if command_id == NO_COMMAND_ID {
            self.emit(BrowserCommand::DismissContextMenu { menu_id });
        } else {
            self.emit(BrowserCommand::ExecuteContextMenuCommand {
                menu_id,
                command_id,
                event_flags: 0,
            });
        }

        self.focus_item(item_id, target);
        Ok(())
    }

    pub(crate) fn show_choice_menu(
        &self,
        target: SurfaceTarget,
        menu: ChromeChoiceMenu,
    ) -> Result<(), crate::error::CompositorError> {
        // Choice menus follow the same slot-local anchoring model as context
        // menus, but selection/dismissal is bridged back through BrowserCommand.
        let Some((item_id, slot)) = self.find_slot_for_target(target) else {
            return Err(crate::error::CompositorError::UnknownTarget);
        };
        if self.window().is_none() {
            self.emit(BrowserCommand::DismissChoiceMenu {
                request_id: menu.request_id,
            });
            return Ok(());
        }
        if matches!(menu.selection_mode, ChromeChoiceMenuSelectionMode::Multiple) {
            self.emit(BrowserCommand::DismissChoiceMenu {
                request_id: menu.request_id,
            });
            return Ok(());
        }

        self.ivars().choice_menu_request_id.set(menu.request_id);
        self.ivars()
            .choice_menu_selected_action
            .set(NO_CHOICE_MENU_ACTION);

        let mtm = MainThreadMarker::new().expect("CompositorViewMac must be on main thread");
        let mut next_selectable_index = 0;
        let ns_menu = build_choice_ns_menu(
            mtm,
            &menu.items,
            self,
            menu.selected_item,
            &mut next_selectable_index,
        );
        let positioning_item = find_item_with_tag(&ns_menu, menu.selected_item as isize);
        let location = slot_menu_location(slot.bounds, menu.x, menu.y, self.isFlipped());

        _ = ns_menu.popUpMenuPositioningItem_atLocation_inView(
            positioning_item.as_deref(),
            location,
            Some(self),
        );

        // `<select>` popups use the same AppKit menu path and can also swallow
        // the release event that would normally clear pointer capture.
        self.ivars()
            .input_state
            .borrow_mut()
            .pointer_capture_item_id = None;

        let request_id = self
            .ivars()
            .choice_menu_request_id
            .replace(NO_CHOICE_MENU_REQUEST_ID);
        let action = self
            .ivars()
            .choice_menu_selected_action
            .replace(NO_CHOICE_MENU_ACTION);

        if request_id != NO_CHOICE_MENU_REQUEST_ID {
            if action == NO_CHOICE_MENU_ACTION {
                self.emit(BrowserCommand::DismissChoiceMenu { request_id });
            } else {
                self.emit(BrowserCommand::AcceptChoiceMenuSelection {
                    request_id,
                    indices: vec![action],
                });
            }
        }

        self.focus_item(item_id, target);
        Ok(())
    }

    pub(crate) fn start_native_drag_session(
        &self,
        target: SurfaceTarget,
        request: &DragStartRequest,
    ) -> Result<bool, crate::error::CompositorError> {
        // Host-owned drag sessions still need a concrete source item so drag
        // coordinates can be converted back into the target surface space.
        let Some((item_id, slot)) = self.find_slot_for_target(target) else {
            return Err(crate::error::CompositorError::UnknownTarget);
        };
        let Some(window) = self.window() else {
            return Ok(false);
        };
        let Some(image_data) = request.image.as_ref() else {
            return Ok(false);
        };
        if image_data.png_bytes.is_empty() {
            return Ok(false);
        }

        let data = unsafe {
            NSData::initWithBytes_length(
                NSData::alloc(),
                image_data.png_bytes.as_ptr().cast(),
                image_data.png_bytes.len() as NSUInteger,
            )
        };
        let Some(image) = NSImage::initWithData(NSImage::alloc(), &data) else {
            return Ok(false);
        };
        let image_scale = if image_data.scale > 0.0 {
            image_data.scale as f64
        } else {
            1.0
        };
        image.setSize(CGSize::new(
            image_data.pixel_width as f64 / image_scale,
            image_data.pixel_height as f64 / image_scale,
        ));

        let writer_string = if !request.data.text.is_empty() {
            request.data.text.as_str()
        } else if let Some(url_info) = request.data.url_infos.first() {
            url_info.url.as_str()
        } else if !request.source_origin.is_empty() {
            request.source_origin.as_str()
        } else {
            "CBF"
        };
        let writer = NSString::from_str(writer_string);
        let writer_ref: &ProtocolObject<dyn NSPasteboardWriting> =
            ProtocolObject::from_ref(&*writer);
        let dragging_item =
            NSDraggingItem::initWithPasteboardWriter(NSDraggingItem::alloc(), writer_ref);

        let mut mouse_location = window.mouseLocationOutsideOfEventStream();
        mouse_location = self.convertPoint_fromView(mouse_location, None);
        let image_size = image.size();
        let drag_frame = NSRect::new(
            NSPoint::new(
                mouse_location.x - image_data.cursor_offset_x as f64,
                mouse_location.y - image_size.height + image_data.cursor_offset_y as f64,
            ),
            image_size,
        );
        unsafe {
            dragging_item.setDraggingFrame_contents(drag_frame, Some(&*image));
        }

        let mtm = MainThreadMarker::new().expect("CompositorViewMac must be on main thread");
        let drag_event = NSApplication::sharedApplication(mtm).currentEvent().or_else(|| {
            let screen_mouse = window.mouseLocationOutsideOfEventStream();
            NSEvent::mouseEventWithType_location_modifierFlags_timestamp_windowNumber_context_eventNumber_clickCount_pressure(
                NSEventType::LeftMouseDragged,
                screen_mouse,
                NSEventModifierFlags::empty(),
                0.0,
                window.windowNumber(),
                None,
                0,
                1,
                1.0,
            )
        });
        let Some(drag_event) = drag_event else {
            return Ok(false);
        };

        let operation_mask = ns_drag_operations_from_generic(request.allowed_operations);
        let source = HostDragSource::new(
            mtm,
            Retained::from(self),
            item_id,
            request.browsing_context_id,
            request.session_id,
            request.allowed_operations,
            operation_mask,
        );
        let source_ref: &ProtocolObject<dyn NSDraggingSource> = ProtocolObject::from_ref(&*source);
        let items = NSArray::arrayWithObject(&*dragging_item);

        self.beginDraggingSessionWithItems_event_source(&items, &drag_event, source_ref);
        self.focus_item(item_id, target);
        self.ivars()
            .active_drag_source
            .replace(Some(source.into_super()));

        _ = slot;

        Ok(true)
    }

    fn upsert_item(&self, item: &PlatformSceneItem) {
        // Reuse CALayerHost instances across scene updates so surface handles,
        // IME bounds, and visibility can be updated without rebuilding layers
        // every frame.
        let mut slots = self.ivars().slots.borrow_mut();

        let slot = slots.entry(item.item_id).or_insert_with(|| {
            let layer = CALayerHost::init(CALayerHost::alloc());
            layer.setGeometryFlipped(true);
            self.layer()
                .expect("CompositorViewMac must have a root layer")
                .addSublayer(&layer);

            SurfaceSlot {
                target: item.target,
                layer,
                bounds: rect_to_cgrect(item.bounds),
                visible: item.visible,
                hit_test: item.hit_test,
                hit_test_snapshot: item.hit_test_snapshot.clone(),
                surface: item.surface.clone(),
                ime_bounds: item.ime_bounds.clone(),
            }
        });

        slot.target = item.target;
        slot.bounds = rect_to_cgrect(item.bounds);
        slot.visible = item.visible;
        slot.hit_test = item.hit_test;
        slot.hit_test_snapshot = item.hit_test_snapshot.clone();
        slot.surface = item.surface.clone();
        slot.ime_bounds = item.ime_bounds.clone();

        CATransaction::begin();
        CATransaction::setDisableActions(true);
        slot.layer.setFrame(slot.bounds);
        slot.layer.setHidden(!item.visible);
        CATransaction::commit();

        match slot.surface {
            Some(PlatformSurfaceHandle::MacCaContextId(context_id)) => unsafe {
                slot.layer.setContextId(context_id);
            },
            None => unsafe {
                slot.layer.setContextId(0);
            },
        }
    }

    fn reorder_sublayers(&self) {
        let slots = self.ivars().slots.borrow();
        let order = self.ivars().order.borrow();
        let Some(root_layer) = self.layer() else {
            return;
        };

        CATransaction::begin();
        CATransaction::setDisableActions(true);
        for item_id in order.iter().rev() {
            if let Some(slot) = slots.get(item_id) {
                slot.layer.removeFromSuperlayer();
                root_layer.addSublayer(&slot.layer);
            }
        }
        CATransaction::commit();
    }

    fn remove_item(&self, item_id: CompositionItemId) {
        let removed = self.ivars().slots.borrow_mut().remove(&item_id);
        self.ivars()
            .order
            .borrow_mut()
            .retain(|candidate| *candidate != item_id);
        if let Some(slot) = removed {
            slot.layer.removeFromSuperlayer();
        }

        let mut input_state = self.ivars().input_state.borrow_mut();
        if input_state.active_item_id == Some(item_id) {
            input_state.active_item_id = None;
        }
        if input_state.hover_item_id == Some(item_id) {
            input_state.hover_item_id = None;
        }
        if input_state.pointer_capture_item_id == Some(item_id) {
            input_state.pointer_capture_item_id = None;
        }
    }

    fn emit(&self, command: BrowserCommand) {
        (self.ivars().command_callback.borrow_mut())(command);
    }

    fn ensure_first_responder(&self) {
        if let Some(window) = self.window() {
            _ = window.makeFirstResponder(Some(self));
        }
    }

    fn active_target(&self) -> Option<(CompositionItemId, SurfaceTarget)> {
        let active_item_id = self.ivars().input_state.borrow().active_item_id?;
        self.item_target(active_item_id)
    }

    fn item_target(
        &self,
        item_id: CompositionItemId,
    ) -> Option<(CompositionItemId, SurfaceTarget)> {
        let slots = self.ivars().slots.borrow();
        let slot = slots.get(&item_id)?;
        slot.visible.then_some((item_id, slot.target))
    }

    fn find_slot_for_target(
        &self,
        target: SurfaceTarget,
    ) -> Option<(CompositionItemId, SurfaceSlot)> {
        // Prefer the active item when multiple items map to the same target,
        // then fall back to the topmost visible item for menu/drag anchoring.
        if let Some(active_item_id) = self.ivars().input_state.borrow().active_item_id
            && let Some(slot) = self.ivars().slots.borrow().get(&active_item_id)
            && slot.target == target
        {
            return Some((active_item_id, slot.clone()));
        }

        let order = self.ivars().order.borrow();
        let slots = self.ivars().slots.borrow();
        for item_id in order.iter() {
            if let Some(slot) = slots.get(item_id)
                && slot.target == target
                && slot.visible
            {
                return Some((*item_id, slot.clone()));
            }
        }

        None
    }

    fn emit_focus(&self, target: SurfaceTarget, focused: bool) {
        match target {
            SurfaceTarget::BrowsingContext(browsing_context_id) => {
                self.emit(BrowserCommand::SetBrowsingContextFocus {
                    browsing_context_id,
                    focused,
                });
            }
            SurfaceTarget::TransientBrowsingContext(transient_browsing_context_id) => {
                self.emit(BrowserCommand::SetTransientBrowsingContextFocus {
                    transient_browsing_context_id,
                    focused,
                });
            }
        }
    }

    pub(crate) fn set_programmatic_active_item(
        &self,
        item_id: Option<CompositionItemId>,
    ) -> Result<(), CompositorError> {
        self.ensure_first_responder();
        match item_id {
            Some(item_id) => {
                let Some((item_id, target)) = self.item_target(item_id) else {
                    return Err(CompositorError::UnknownItem);
                };
                self.focus_item(item_id, target);
                Ok(())
            }
            None => {
                let previous = self.ivars().input_state.borrow().active_item_id;
                if let Some(previous_item_id) = previous
                    && let Some((_, previous_target)) = self.item_target(previous_item_id)
                {
                    self.emit_focus(previous_target, false);
                }
                self.ivars().input_state.borrow_mut().active_item_id = None;
                Ok(())
            }
        }
    }

    fn focus_item(&self, item_id: CompositionItemId, target: SurfaceTarget) {
        let previous = self.ivars().input_state.borrow().active_item_id;
        if previous == Some(item_id) {
            return;
        }

        if let Some(previous_item_id) = previous
            && let Some((_, previous_target)) = self.item_target(previous_item_id)
        {
            self.emit_focus(previous_target, false);
        }

        self.ivars().input_state.borrow_mut().active_item_id = Some(item_id);
        self.emit_focus(target, true);
    }

    fn send_edit_action(&self, action: EditAction) {
        let Some((_, target)) = self.active_target() else {
            return;
        };

        match target {
            SurfaceTarget::BrowsingContext(browsing_context_id) => {
                self.emit(BrowserCommand::ExecuteEditAction {
                    browsing_context_id,
                    action,
                });
            }
            SurfaceTarget::TransientBrowsingContext(transient_browsing_context_id) => {
                self.emit(
                    BrowserCommand::ExecuteEditActionInTransientBrowsingContext {
                        transient_browsing_context_id,
                        action,
                    },
                );
            }
        }
    }

    fn send_key_event(&self, target: SurfaceTarget, event: KeyEvent, commands: Vec<String>) {
        match target {
            SurfaceTarget::BrowsingContext(browsing_context_id) => {
                self.emit(BrowserCommand::SendKeyEvent {
                    browsing_context_id,
                    event,
                    commands,
                });
            }
            SurfaceTarget::TransientBrowsingContext(transient_browsing_context_id) => {
                self.emit(BrowserCommand::SendKeyEventToTransientBrowsingContext {
                    transient_browsing_context_id,
                    event,
                    commands,
                });
            }
        }
    }

    fn send_char_event(&self, event: KeyEvent) {
        let Some((_, target)) = self.active_target() else {
            return;
        };
        self.send_key_event(target, event, Vec::new());
    }

    fn send_mouse_event(&self, target: SurfaceTarget, event: MouseEvent) {
        match target {
            SurfaceTarget::BrowsingContext(browsing_context_id) => {
                self.emit(BrowserCommand::SendMouseEvent {
                    browsing_context_id,
                    event,
                });
            }
            SurfaceTarget::TransientBrowsingContext(transient_browsing_context_id) => {
                self.emit(BrowserCommand::SendMouseEventToTransientBrowsingContext {
                    transient_browsing_context_id,
                    event,
                });
            }
        }
    }

    fn send_mouse_wheel_event(&self, target: SurfaceTarget, event: MouseWheelEvent) {
        match target {
            SurfaceTarget::BrowsingContext(browsing_context_id) => {
                self.emit(BrowserCommand::SendMouseWheelEvent {
                    browsing_context_id,
                    event,
                });
            }
            SurfaceTarget::TransientBrowsingContext(transient_browsing_context_id) => {
                self.emit(
                    BrowserCommand::SendMouseWheelEventToTransientBrowsingContext {
                        transient_browsing_context_id,
                        event,
                    },
                );
            }
        }
    }

    fn send_set_composition(
        &self,
        text: String,
        selection: Option<ImeTextRange>,
        replacement: Option<ImeTextRange>,
    ) {
        let Some((_, target)) = self.active_target() else {
            return;
        };

        let (selection_start, selection_end) = selection_range(selection, &text);
        match target {
            SurfaceTarget::BrowsingContext(browsing_context_id) => {
                self.emit(BrowserCommand::SetComposition {
                    composition: ImeComposition {
                        browsing_context_id,
                        text: text.clone(),
                        selection_start,
                        selection_end,
                        replacement_range: replacement,
                        spans: composition_span(&text),
                    },
                });
            }
            SurfaceTarget::TransientBrowsingContext(transient_browsing_context_id) => {
                self.emit(BrowserCommand::SetTransientComposition {
                    composition: TransientImeComposition {
                        transient_browsing_context_id,
                        text: text.clone(),
                        selection_start,
                        selection_end,
                        replacement_range: replacement,
                        spans: composition_span(&text),
                    },
                });
            }
        }
    }

    fn send_commit_text(
        &self,
        text: String,
        replacement: Option<ImeTextRange>,
        relative_caret_position: i32,
    ) {
        let Some((_, target)) = self.active_target() else {
            return;
        };

        match target {
            SurfaceTarget::BrowsingContext(browsing_context_id) => {
                self.emit(BrowserCommand::CommitText {
                    commit: ImeCommitText {
                        browsing_context_id,
                        text,
                        relative_caret_position,
                        replacement_range: replacement,
                        spans: Vec::new(),
                    },
                });
            }
            SurfaceTarget::TransientBrowsingContext(transient_browsing_context_id) => {
                self.emit(BrowserCommand::CommitTransientText {
                    commit: TransientImeCommitText {
                        transient_browsing_context_id,
                        text,
                        relative_caret_position,
                        replacement_range: replacement,
                        spans: Vec::new(),
                    },
                });
            }
        }
    }

    fn send_finish_composing(&self, keep_selection: bool) {
        let Some((_, target)) = self.active_target() else {
            return;
        };
        let behavior = if keep_selection {
            ConfirmCompositionBehavior::KeepSelection
        } else {
            ConfirmCompositionBehavior::DoNotKeepSelection
        };
        match target {
            SurfaceTarget::BrowsingContext(browsing_context_id) => {
                self.emit(BrowserCommand::FinishComposingText {
                    browsing_context_id,
                    behavior,
                });
            }
            SurfaceTarget::TransientBrowsingContext(transient_browsing_context_id) => {
                self.emit(
                    BrowserCommand::FinishComposingTextInTransientBrowsingContext {
                        transient_browsing_context_id,
                        behavior,
                    },
                );
            }
        }
    }

    fn send_inserted_text(&self, text: String, as_commit: bool) {
        if as_commit {
            self.send_commit_text(text, None, 0);
            return;
        }

        let pending_char_event = self.ivars().pending_char_event.borrow().clone();
        if let Some(event) = build_char_event(pending_char_event, text) {
            self.ivars().sent_char_event.set(true);
            self.send_char_event(event);
        }
    }

    fn forward_key_event(&self, event: &NSEvent) {
        let Some((_, target)) = self.active_target() else {
            return;
        };
        let key_event = self.convert_key_event(event);
        let commands = std::mem::take(&mut *self.ivars().edit_commands.borrow_mut());
        self.send_key_event(target, key_event, commands);
    }

    fn forward_mouse_event(&self, event: &NSEvent, event_type: MouseEventType) {
        if event_type == MouseEventType::Move
            && self
                .ivars()
                .input_state
                .borrow()
                .pointer_capture_item_id
                .is_none()
        {
            self.forward_hover_move_event(event);
            return;
        }

        let mouse_down = event_type == MouseEventType::Down;
        let Some((item_id, target)) = self.mouse_target(event, mouse_down) else {
            return;
        };

        if mouse_down {
            self.focus_item(item_id, target);
            self.ivars()
                .input_state
                .borrow_mut()
                .pointer_capture_item_id = Some(item_id);
        } else if event_type == MouseEventType::Up {
            let mut input_state = self.ivars().input_state.borrow_mut();
            if input_state.pointer_capture_item_id == Some(item_id) {
                input_state.pointer_capture_item_id = None;
            }
        }

        self.send_translated_mouse_event(item_id, target, event, event_type);
    }

    fn mouse_target(
        &self,
        event: &NSEvent,
        update_active: bool,
    ) -> Option<(CompositionItemId, SurfaceTarget)> {
        // Pointer capture wins while a drag is in progress; otherwise resolve
        // the topmost visible item whose hit-test policy matches the cursor.
        if let Some(item_id) = self.ivars().input_state.borrow().pointer_capture_item_id {
            if let Some(target) = self.item_target(item_id) {
                return Some(target);
            }
            self.ivars()
                .input_state
                .borrow_mut()
                .pointer_capture_item_id = None;
        }

        let point = self.local_point(event);
        let slots = self.ivars().slots.borrow();
        let order = self.ivars().order.borrow();
        let item_id = topmost_item_at_point(&order, &slots, point)?;
        let target = slots.get(&item_id).map(|slot| (item_id, slot.target))?;
        drop(order);
        drop(slots);

        if update_active {
            self.focus_item(target.0, target.1);
        }

        Some(target)
    }

    fn hover_target(&self, event: &NSEvent) -> Option<(CompositionItemId, SurfaceTarget)> {
        let point = self.local_point(event);
        self.target_at_point(point)
    }

    fn target_at_point(&self, point: CGPoint) -> Option<(CompositionItemId, SurfaceTarget)> {
        let slots = self.ivars().slots.borrow();
        let order = self.ivars().order.borrow();
        let item_id = topmost_item_at_point(&order, &slots, point)?;
        slots.get(&item_id).map(|slot| (item_id, slot.target))
    }

    fn forward_hover_move_event(&self, event: &NSEvent) {
        let previous_item_id = self.ivars().input_state.borrow().hover_item_id;
        let next = self.hover_target(event);
        let next_item_id = next.map(|(item_id, _)| item_id);

        for dispatch in hover_transition(previous_item_id, next_item_id) {
            match dispatch {
                HoverDispatch::Leave(item_id) => {
                    if let Some((_, target)) = self.item_target(item_id) {
                        self.send_translated_mouse_event(
                            item_id,
                            target,
                            event,
                            MouseEventType::Leave,
                        );
                    }
                }
                HoverDispatch::Enter(item_id) => {
                    if let Some((next_item_id, target)) = next
                        && next_item_id == item_id
                    {
                        self.send_translated_mouse_event(
                            item_id,
                            target,
                            event,
                            MouseEventType::Enter,
                        );
                    }
                }
                HoverDispatch::Move(item_id) => {
                    if let Some((next_item_id, target)) = next
                        && next_item_id == item_id
                    {
                        self.send_translated_mouse_event(
                            item_id,
                            target,
                            event,
                            MouseEventType::Move,
                        );
                    }
                }
            }
        }

        self.ivars().input_state.borrow_mut().hover_item_id = next_item_id;
    }

    fn clear_hover_target(&self, event: &NSEvent) {
        if self
            .ivars()
            .input_state
            .borrow()
            .pointer_capture_item_id
            .is_some()
        {
            return;
        }

        let previous_item_id = self.ivars().input_state.borrow_mut().hover_item_id.take();
        if let Some(item_id) = previous_item_id
            && let Some((_, target)) = self.item_target(item_id)
        {
            self.send_translated_mouse_event(item_id, target, event, MouseEventType::Leave);
        }
    }

    fn local_point(&self, event: &NSEvent) -> CGPoint {
        let point = event.locationInWindow();
        self.convertPoint_fromView(point, None)
    }

    fn dragging_points(
        &self,
        sender: &ProtocolObject<dyn NSDraggingInfo>,
    ) -> Option<(CGPoint, NSPoint)> {
        let window_point = sender.draggingLocation();
        let window = self.window()?;
        let local_point = self.convertPoint_fromView(window_point, None);
        let screen_point = window.convertPointToScreen(window_point);
        Some((local_point, screen_point))
    }

    fn drag_target_at_point(
        &self,
        point: CGPoint,
    ) -> Option<(CompositionItemId, BrowsingContextId)> {
        let slots = self.ivars().slots.borrow();
        let order = self.ivars().order.borrow();
        let item_id = topmost_item_at_point(&order, &slots, point)?;
        let slot = slots.get(&item_id)?;
        if !slot.visible || !slot_hit_test_contains_point(slot, point) {
            return None;
        }
        match slot.target {
            SurfaceTarget::BrowsingContext(browsing_context_id) => {
                Some((item_id, browsing_context_id))
            }
            SurfaceTarget::TransientBrowsingContext(_) => None,
        }
    }

    fn current_modifier_flags(&self) -> u32 {
        self.window()
            .and_then(|_| MainThreadMarker::new())
            .and_then(|mtm| NSApplication::sharedApplication(mtm).currentEvent())
            .map(|event| event.modifierFlags().bits() as u32)
            .unwrap_or(0)
    }

    fn convert_key_event(&self, event: &NSEvent) -> KeyEvent {
        let nsevent_ptr = NonNull::from(event).cast::<c_void>();
        convert_nsevent_to_key_event(0, nsevent_ptr)
    }

    fn emit_native_drag_update(
        &self,
        item_id: CompositionItemId,
        browsing_context_id: cbf::data::ids::BrowsingContextId,
        session_id: u64,
        allowed_operations: DragOperations,
        screen_point: NSPoint,
    ) {
        let (widget_x, widget_y, screen_x, screen_y) = self.drag_points(item_id, screen_point);
        let modifiers = self
            .window()
            .and_then(|_| MainThreadMarker::new())
            .and_then(|mtm| NSApplication::sharedApplication(mtm).currentEvent())
            .map(|event| event.modifierFlags().bits() as u32)
            .unwrap_or(0);
        self.emit(BrowserCommand::SendDragUpdate {
            update: cbf::data::drag::DragUpdate {
                session_id,
                browsing_context_id,
                allowed_operations,
                modifiers,
                position_in_widget_x: widget_x,
                position_in_widget_y: widget_y,
                position_in_screen_x: screen_x,
                position_in_screen_y: screen_y,
            },
        });
    }

    fn emit_native_drag_drop(
        &self,
        item_id: CompositionItemId,
        browsing_context_id: cbf::data::ids::BrowsingContextId,
        session_id: u64,
        screen_point: NSPoint,
    ) {
        let (widget_x, widget_y, screen_x, screen_y) = self.drag_points(item_id, screen_point);
        let modifiers = self
            .window()
            .and_then(|_| MainThreadMarker::new())
            .and_then(|mtm| NSApplication::sharedApplication(mtm).currentEvent())
            .map(|event| event.modifierFlags().bits() as u32)
            .unwrap_or(0);
        self.emit(BrowserCommand::SendDragDrop {
            drop: cbf::data::drag::DragDrop {
                session_id,
                browsing_context_id,
                modifiers,
                position_in_widget_x: widget_x,
                position_in_widget_y: widget_y,
                position_in_screen_x: screen_x,
                position_in_screen_y: screen_y,
            },
        });
    }

    fn emit_native_drag_cancel(
        &self,
        session_id: u64,
        browsing_context_id: cbf::data::ids::BrowsingContextId,
    ) {
        self.emit(BrowserCommand::SendDragCancel {
            session_id,
            browsing_context_id,
        });
    }

    fn begin_external_drag_session(
        &self,
        item_id: CompositionItemId,
        browsing_context_id: BrowsingContextId,
        data: DragData,
        allowed_operations: DragOperations,
        screen_point: NSPoint,
    ) {
        let (widget_x, widget_y, screen_x, screen_y) = self.drag_points(item_id, screen_point);
        let modifiers = self.current_modifier_flags();
        self.ivars()
            .external_drag_state
            .replace(Some(ExternalDragSessionState {
                item_id,
                browsing_context_id,
                data: data.clone(),
                allowed_operations,
                operation: DragOperation::Copy,
            }));
        self.emit(BrowserCommand::SendExternalDragEnter {
            event: ExternalDragEnter {
                browsing_context_id,
                data,
                allowed_operations,
                modifiers,
                position_in_widget_x: widget_x,
                position_in_widget_y: widget_y,
                position_in_screen_x: screen_x,
                position_in_screen_y: screen_y,
            },
        });
    }

    fn update_external_drag_session(
        &self,
        item_id: CompositionItemId,
        browsing_context_id: BrowsingContextId,
        allowed_operations: DragOperations,
        screen_point: NSPoint,
    ) {
        let (widget_x, widget_y, screen_x, screen_y) = self.drag_points(item_id, screen_point);
        let modifiers = self.current_modifier_flags();
        if let Some(state) = self.ivars().external_drag_state.borrow_mut().as_mut() {
            state.item_id = item_id;
            state.browsing_context_id = browsing_context_id;
            state.allowed_operations = allowed_operations;
        }
        self.emit(BrowserCommand::SendExternalDragUpdate {
            event: ExternalDragUpdate {
                browsing_context_id,
                allowed_operations,
                modifiers,
                position_in_widget_x: widget_x,
                position_in_widget_y: widget_y,
                position_in_screen_x: screen_x,
                position_in_screen_y: screen_y,
            },
        });
    }

    fn leave_external_drag_session(&self) {
        let Some(state) = self.ivars().external_drag_state.borrow_mut().take() else {
            return;
        };
        self.emit(BrowserCommand::SendExternalDragLeave {
            browsing_context_id: state.browsing_context_id,
        });
    }

    fn drop_external_drag_session(
        &self,
        item_id: CompositionItemId,
        browsing_context_id: BrowsingContextId,
        screen_point: NSPoint,
    ) {
        let (widget_x, widget_y, screen_x, screen_y) = self.drag_points(item_id, screen_point);
        let modifiers = self.current_modifier_flags();
        self.ivars().external_drag_state.borrow_mut().take();
        self.emit(BrowserCommand::SendExternalDragDrop {
            event: ExternalDragDrop {
                browsing_context_id,
                modifiers,
                position_in_widget_x: widget_x,
                position_in_widget_y: widget_y,
                position_in_screen_x: screen_x,
                position_in_screen_y: screen_y,
            },
        });
    }

    fn current_external_drag_operation(&self) -> NSDragOperation {
        self.ivars()
            .external_drag_state
            .borrow()
            .as_ref()
            .map(|state| ns_drag_operation_from_generic(state.operation))
            .unwrap_or(NSDragOperation::None)
    }

    fn is_same_context_drag_drop_point(
        &self,
        browsing_context_id: BrowsingContextId,
        screen_point: NSPoint,
    ) -> bool {
        let Some(window) = self.window() else {
            return false;
        };
        let base_point = window.convertPointFromScreen(screen_point);
        let local_point = self.convertPoint_fromView(base_point, None);
        self.drag_target_at_point(local_point)
            .map(|(_, target_browsing_context_id)| {
                target_browsing_context_id == browsing_context_id
            })
            .unwrap_or(false)
    }

    fn drag_points(
        &self,
        item_id: CompositionItemId,
        screen_point: NSPoint,
    ) -> (f32, f32, f32, f32) {
        let Some(window) = self.window() else {
            return (0.0, 0.0, screen_point.x as f32, screen_point.y as f32);
        };
        let Some(slot) = self.ivars().slots.borrow().get(&item_id).cloned() else {
            return (0.0, 0.0, screen_point.x as f32, screen_point.y as f32);
        };
        let base_point = window.convertPointFromScreen(screen_point);
        let local_point = self.convertPoint_fromView(base_point, None);
        let widget_x = (local_point.x - slot.bounds.origin.x) as f32;
        let widget_y =
            (slot.bounds.size.height - (local_point.y - slot.bounds.origin.y)).max(0.0) as f32;
        (
            widget_x,
            widget_y,
            screen_point.x as f32,
            screen_point.y as f32,
        )
    }

    fn convert_mouse_event(&self, event: &NSEvent) -> MouseEvent {
        let nsevent_ptr = NonNull::from(event).cast::<c_void>();
        let nsview_ptr = NonNull::from(self).cast::<c_void>();
        convert_nsevent_to_mouse_event(0, nsevent_ptr, nsview_ptr, PointerType::Mouse, false)
    }

    fn convert_mouse_wheel_event(&self, event: &NSEvent) -> MouseWheelEvent {
        let nsevent_ptr = NonNull::from(event).cast::<c_void>();
        let nsview_ptr = NonNull::from(self).cast::<c_void>();
        convert_nsevent_to_mouse_wheel_event(0, nsevent_ptr, nsview_ptr)
    }

    fn send_translated_mouse_event(
        &self,
        item_id: CompositionItemId,
        target: SurfaceTarget,
        event: &NSEvent,
        event_type: MouseEventType,
    ) {
        let mut mouse_event = self.convert_mouse_event(event);
        mouse_event.type_ = event_type;
        self.translate_mouse_event(item_id, &mut mouse_event);
        self.send_mouse_event(target, mouse_event);
    }

    fn translate_mouse_event(&self, item_id: CompositionItemId, event: &mut MouseEvent) {
        if let Some(slot) = self.ivars().slots.borrow().get(&item_id) {
            event.position_in_widget_x -= slot.bounds.origin.x as f32;
            event.position_in_widget_y -= self.slot_top_offset(slot) as f32;
        }
    }

    fn translate_wheel_event(&self, item_id: CompositionItemId, event: &mut MouseWheelEvent) {
        if let Some(slot) = self.ivars().slots.borrow().get(&item_id) {
            event.position_in_widget_x -= slot.bounds.origin.x as f32;
            event.position_in_widget_y -= self.slot_top_offset(slot) as f32;
        }
    }

    fn slot_top_offset(&self, slot: &SurfaceSlot) -> f64 {
        let bounds = self.bounds();
        (bounds.size.height - (slot.bounds.origin.y + slot.bounds.size.height)).max(0.0)
    }

    fn install_tracking_area(&self) {
        let tracking_areas = self.trackingAreas();
        for index in 0..tracking_areas.count() {
            let tracking_area = tracking_areas.objectAtIndex(index);
            self.removeTrackingArea(&tracking_area);
        }

        let options = NSTrackingAreaOptions::MouseEnteredAndExited
            | NSTrackingAreaOptions::MouseMoved
            | NSTrackingAreaOptions::ActiveInKeyWindow
            | NSTrackingAreaOptions::InVisibleRect
            | NSTrackingAreaOptions::EnabledDuringMouseDrag;
        let tracking_area = unsafe {
            NSTrackingArea::initWithRect_options_owner_userInfo(
                NSTrackingArea::alloc(),
                self.bounds(),
                options,
                Some(self),
                None,
            )
        };
        self.addTrackingArea(&tracking_area);
    }

    fn ime_candidate_rect(&self, range: NSRange) -> CGRect {
        // AppKit asks for candidate window placement in compositor coordinates;
        // convert the active slot's browser-reported IME bounds into screen space.
        let fallback = self
            .window()
            .map(|window| window.frame())
            .unwrap_or_else(|| CGRect::new(CGPoint::ZERO, self.frame().size));

        let Some(active_item_id) = self.ivars().input_state.borrow().active_item_id else {
            return fallback;
        };

        let slots = self.ivars().slots.borrow();
        let Some(slot) = slots.get(&active_item_id) else {
            return fallback;
        };

        let Some(rect) = candidate_rect_for_slot(range, slot.bounds, slot.ime_bounds.as_ref())
        else {
            return fallback;
        };

        self.to_screen_rect(rect)
    }

    fn to_screen_rect(&self, rect: CGRect) -> CGRect {
        let window_rect = self.convertRect_toView(rect, None);
        if let Some(window) = self.window() {
            window.convertRectToScreen(window_rect)
        } else {
            window_rect
        }
    }

    fn update_marked_state(
        &self,
        has_marked_text: bool,
        marked_range: NSRange,
        selected_range: NSRange,
    ) {
        self.ivars().has_marked_text.set(has_marked_text);
        self.ivars().marked_range.set(marked_range);
        self.ivars().selected_range.set(selected_range);
    }

    fn mark_ime_handled(&self) {
        self.ivars().ime_handled.set(true);
    }
}

#[inline]
fn ns_not_found_range() -> NSRange {
    NSRange::new(NSNotFound as usize, 0)
}

fn drag_operations_from_ns(operation: NSDragOperation) -> DragOperations {
    let mut bits = DragOperations::NONE.bits();
    if operation.contains(NSDragOperation::Copy) {
        bits |= DragOperations::COPY.bits();
    }
    if operation.contains(NSDragOperation::Link) {
        bits |= DragOperations::LINK.bits();
    }
    if operation.contains(NSDragOperation::Move) {
        bits |= DragOperations::MOVE.bits();
    }
    DragOperations::from_bits(bits)
}

fn ns_drag_operations_from_generic(operations: DragOperations) -> NSDragOperation {
    let mut mask = NSDragOperation::None;
    if operations.contains(DragOperation::Copy) {
        mask |= NSDragOperation::Copy;
    }
    if operations.contains(DragOperation::Link) {
        mask |= NSDragOperation::Link;
    }
    if operations.contains(DragOperation::Move) {
        mask |= NSDragOperation::Move;
    }
    mask
}

fn ns_drag_operation_from_generic(operation: DragOperation) -> NSDragOperation {
    match operation {
        DragOperation::None => NSDragOperation::None,
        DragOperation::Copy => NSDragOperation::Copy,
        DragOperation::Link => NSDragOperation::Link,
        DragOperation::Move => NSDragOperation::Move,
    }
}

fn slot_menu_location(bounds: CGRect, x: i32, y: i32, flipped: bool) -> NSPoint {
    let x = bounds.origin.x + x as f64;
    let y = if flipped {
        bounds.origin.y + y as f64
    } else {
        bounds.origin.y + (bounds.size.height - y as f64).max(0.0)
    };
    NSPoint::new(x, y)
}

fn build_context_ns_menu(
    mtm: MainThreadMarker,
    items: &[ContextMenuItem],
    target: &CompositorViewMac,
) -> Retained<NSMenu> {
    let title = NSString::from_str("");
    let menu = NSMenu::initWithTitle(NSMenu::alloc(mtm), &title);

    for item in items {
        if !item.visible {
            continue;
        }
        if let Some(menu_item) = build_context_ns_menu_item(mtm, item, target) {
            menu.addItem(&menu_item);
        }
    }

    menu
}

fn build_context_ns_menu_item(
    mtm: MainThreadMarker,
    item: &ContextMenuItem,
    target: &CompositorViewMac,
) -> Option<Retained<NSMenuItem>> {
    let title_text = menu_item_title(item);
    let menu_item = match item.r#type {
        ContextMenuItemType::Separator => return Some(NSMenuItem::separatorItem(mtm)),
        ContextMenuItemType::Title => {
            let title = NSString::from_str(&title_text);
            return Some(NSMenuItem::sectionHeaderWithTitle(&title, mtm));
        }
        _ => {
            let title = NSString::from_str(&title_text);
            let key_equivalent = item
                .accelerator
                .as_ref()
                .map(|accel| accel.key_equivalent.as_str())
                .unwrap_or("");
            let key_equivalent = NSString::from_str(key_equivalent);
            let action = if matches!(
                item.r#type,
                ContextMenuItemType::Submenu | ContextMenuItemType::ActionableSubmenu
            ) {
                None
            } else {
                Some(sel!(contextMenuItemSelected:))
            };

            unsafe {
                NSMenuItem::initWithTitle_action_keyEquivalent(
                    NSMenuItem::alloc(mtm),
                    &title,
                    action,
                    &key_equivalent,
                )
            }
        }
    };

    if matches!(
        item.r#type,
        ContextMenuItemType::Submenu | ContextMenuItemType::ActionableSubmenu
    ) {
        let submenu = build_context_ns_menu(mtm, &item.submenu, target);
        menu_item.setSubmenu(Some(&submenu));
    } else {
        unsafe {
            menu_item.setTarget(Some(target));
        }
    }

    menu_item.setEnabled(item.enabled);
    menu_item.setHidden(!item.visible);
    menu_item.setTag(item.command_id as isize);

    if let Some(subtitle) = menu_item_subtitle(item) {
        let subtitle = NSString::from_str(&subtitle);
        menu_item.setSubtitle(Some(&subtitle));
    }
    if !item.minor_text.is_empty() {
        let tooltip = NSString::from_str(&item.minor_text);
        menu_item.setToolTip(Some(&tooltip));
    }
    if let Some(icon) = build_ns_image(item.icon.as_ref().or(item.minor_icon.as_ref())) {
        menu_item.setImage(Some(&icon));
    }
    if item.is_alerted {
        let badge = NSMenuItemBadge::alertsWithCount(1);
        menu_item.setBadge(Some(&badge));
    } else if item.is_new_feature {
        let badge = NSMenuItemBadge::newItemsWithCount(1);
        menu_item.setBadge(Some(&badge));
    }
    if let Some(accel) = item.accelerator.as_ref() {
        let modifier_mask = NSEventModifierFlags::from_bits_truncate(accel.modifier_mask as _);
        menu_item.setKeyEquivalentModifierMask(modifier_mask);
    }
    if matches!(
        item.r#type,
        ContextMenuItemType::Check | ContextMenuItemType::Radio
    ) {
        menu_item.setState(if item.checked {
            NSControlStateValueOn
        } else {
            NSControlStateValueOff
        });
    }

    Some(menu_item)
}

fn build_choice_ns_menu(
    mtm: MainThreadMarker,
    items: &[ChromeChoiceMenuItem],
    target: &CompositorViewMac,
    selected_index: i32,
    next_selectable_index: &mut i32,
) -> Retained<NSMenu> {
    let title = NSString::from_str("");
    let menu = NSMenu::initWithTitle(NSMenu::alloc(mtm), &title);

    for item in items {
        if let Some(menu_item) =
            build_choice_ns_menu_item(mtm, item, target, selected_index, next_selectable_index)
        {
            menu.addItem(&menu_item);
        }
    }

    menu
}

fn build_choice_ns_menu_item(
    mtm: MainThreadMarker,
    item: &ChromeChoiceMenuItem,
    target: &CompositorViewMac,
    selected_index: i32,
    next_selectable_index: &mut i32,
) -> Option<Retained<NSMenuItem>> {
    match item.item_type {
        ChromeChoiceMenuItemType::Separator => Some(NSMenuItem::separatorItem(mtm)),
        ChromeChoiceMenuItemType::Group => {
            let title = NSString::from_str(item.label.as_deref().unwrap_or_default());
            Some(NSMenuItem::sectionHeaderWithTitle(&title, mtm))
        }
        ChromeChoiceMenuItemType::SubMenu => {
            let title = NSString::from_str(item.label.as_deref().unwrap_or_default());
            let menu_item = unsafe {
                NSMenuItem::initWithTitle_action_keyEquivalent(
                    NSMenuItem::alloc(mtm),
                    &title,
                    None,
                    &NSString::from_str(""),
                )
            };
            let submenu = build_choice_ns_menu(
                mtm,
                &item.children,
                target,
                selected_index,
                next_selectable_index,
            );
            menu_item.setSubmenu(Some(&submenu));
            menu_item.setEnabled(item.enabled);
            if let Some(tool_tip) = item.tool_tip.as_deref().filter(|value| !value.is_empty()) {
                menu_item.setToolTip(Some(&NSString::from_str(tool_tip)));
            }
            Some(menu_item)
        }
        ChromeChoiceMenuItemType::Option | ChromeChoiceMenuItemType::CheckableOption => {
            let title = NSString::from_str(item.label.as_deref().unwrap_or_default());
            let menu_item = unsafe {
                NSMenuItem::initWithTitle_action_keyEquivalent(
                    NSMenuItem::alloc(mtm),
                    &title,
                    Some(sel!(choiceMenuItemSelected:)),
                    &NSString::from_str(""),
                )
            };
            unsafe {
                menu_item.setTarget(Some(target));
            }
            let item_index = *next_selectable_index;
            *next_selectable_index += 1;
            menu_item.setEnabled(item.enabled);
            menu_item.setTag(item_index as isize);
            if let Some(tool_tip) = item.tool_tip.as_deref().filter(|value| !value.is_empty()) {
                menu_item.setToolTip(Some(&NSString::from_str(tool_tip)));
            }
            if item_index == selected_index {
                menu_item.setState(NSControlStateValueOn);
            } else if matches!(item.item_type, ChromeChoiceMenuItemType::CheckableOption) {
                menu_item.setState(if item.checked {
                    NSControlStateValueOn
                } else {
                    NSControlStateValueOff
                });
            }
            Some(menu_item)
        }
    }
}

fn find_item_with_tag(menu: &NSMenu, tag: isize) -> Option<Retained<NSMenuItem>> {
    if tag < 0 {
        return None;
    }
    menu.itemWithTag(tag)
}

fn menu_item_title(item: &ContextMenuItem) -> String {
    let mut title = if item.label.is_empty() {
        item.accessible_name.clone()
    } else {
        item.label.clone()
    };
    if item.may_have_mnemonics {
        title = strip_mnemonic(&title);
    }
    title
}

fn menu_item_subtitle(item: &ContextMenuItem) -> Option<String> {
    if !item.secondary_label.is_empty() {
        Some(item.secondary_label.clone())
    } else if !item.minor_text.is_empty() {
        Some(item.minor_text.clone())
    } else {
        None
    }
}

fn strip_mnemonic(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '&' {
            if matches!(chars.peek(), Some('&')) {
                output.push('&');
                chars.next();
            }
            continue;
        }
        output.push(ch);
    }
    output
}

fn build_ns_image(icon: Option<&ContextMenuIcon>) -> Option<Retained<NSImage>> {
    let icon = icon?;
    if icon.png_bytes.is_empty() {
        return None;
    }
    let data = unsafe {
        NSData::initWithBytes_length(
            NSData::alloc(),
            icon.png_bytes.as_ptr().cast(),
            icon.png_bytes.len() as NSUInteger,
        )
    };
    let image = NSImage::initWithData(NSImage::alloc(), &data)?;
    image.setSize(CGSize::new(icon.width as f64, icon.height as f64));
    Some(image)
}

fn rect_to_cgrect(rect: crate::model::Rect) -> CGRect {
    CGRect::new(
        CGPoint::new(rect.x, rect.y),
        CGSize::new(rect.width, rect.height),
    )
}

fn composition_span(text: &str) -> Vec<ImeTextSpan> {
    vec![ImeTextSpan::no_decoration(
        ImeTextSpanType::Composition,
        0,
        text.encode_utf16().count() as u32,
    )]
}

fn selection_range(selection: Option<ImeTextRange>, text: &str) -> (i32, i32) {
    selection
        .map(|range| (range.start, range.end))
        .unwrap_or_else(|| {
            let len = text.encode_utf16().count() as i32;
            (len, len)
        })
}

fn nsrange_to_text_range(range: NSRange) -> Option<ImeTextRange> {
    if range.location == NSNotFound as usize {
        return None;
    }

    let start = range.location.min(i32::MAX as usize) as i32;
    let end = range.end().min(i32::MAX as usize).max(start as usize) as i32;

    Some(ImeTextRange { start, end })
}

fn composition_range_for_text(text: &str) -> NSRange {
    NSRange::new(0, text.encode_utf16().count())
}

fn build_char_event(template: Option<KeyEvent>, text: String) -> Option<KeyEvent> {
    if text.is_empty() {
        return None;
    }

    let mut event =
        template.unwrap_or_else(|| KeyEvent::char_input(0, 0, 0, text.clone(), text.clone()));
    event.type_ = KeyEventType::Char;
    event.text = Some(text.clone());

    if event
        .unmodified_text
        .as_deref()
        .map(str::is_empty)
        .unwrap_or(true)
    {
        event.unmodified_text = Some(text);
    }

    Some(event)
}

fn synthesized_char_text(template: Option<&KeyEvent>) -> Option<String> {
    let event = template?;
    event
        .text
        .as_ref()
        .filter(|text| !text.is_empty())
        .cloned()
        .or_else(|| {
            event
                .unmodified_text
                .as_ref()
                .filter(|text| !text.is_empty())
                .cloned()
        })
}

fn extract_insert_text(value: &AnyObject) -> Option<String> {
    let mut text = if let Some(attributed) = value.downcast_ref::<NSAttributedString>() {
        Some(attributed.string().to_string())
    } else {
        value
            .downcast_ref::<NSString>()
            .map(|ns_string| ns_string.to_string())
    }?;

    // Keep return/newline so Enter still emits a Char event, but drop other
    // control characters that should not be treated as text insertion.
    text = text
        .chars()
        .filter(|c| matches!(c, '\r' | '\n') || !c.is_control())
        .collect::<String>();

    (!text.is_empty()).then_some(text)
}

fn hover_transition(
    previous: Option<CompositionItemId>,
    next: Option<CompositionItemId>,
) -> Vec<HoverDispatch> {
    if previous == next {
        return next.into_iter().map(HoverDispatch::Move).collect();
    }

    let mut dispatches = Vec::with_capacity(3);
    if let Some(item_id) = previous {
        dispatches.push(HoverDispatch::Leave(item_id));
    }
    if let Some(item_id) = next {
        dispatches.push(HoverDispatch::Enter(item_id));
        dispatches.push(HoverDispatch::Move(item_id));
    }
    dispatches
}

// Chromium-derived VKEY constants mirrored for the marked-text accelerator
// guard. `convert_nsevent_to_key_event` fills `KeyEvent.key_code` with
// Chromium-compatible virtual-key values, so this guard must compare against
// that key space instead of raw macOS `NSEvent.keyCode` values. This follows
// Chromium's macOS text-input handling in
// `components/remote_cocoa/app_shim/bridged_content_view.mm`.
const VKEY_TAB: i32 = 0x09;
const VKEY_RETURN: i32 = 0x0D;
const VKEY_ESCAPE: i32 = 0x1B;
const VKEY_PRIOR: i32 = 0x21;
const VKEY_NEXT: i32 = 0x22;
const VKEY_LEFT: i32 = 0x25;
const VKEY_UP: i32 = 0x26;
const VKEY_RIGHT: i32 = 0x27;
const VKEY_DOWN: i32 = 0x28;

// Mirrors Chromium's macOS IME guard for marked text in
// `components/remote_cocoa/app_shim/bridged_content_view.mm`
// `ShouldIgnoreAcceleratorWithMarkedText`. While marked text is active,
// AppKit may consume these confirmation/navigation keys inside IME handling
// without invoking our NSTextInputClient callbacks, so they must not be
// forwarded as page accelerators.
fn should_ignore_accelerator_with_marked_text(event: &NSEvent) -> bool {
    let nsevent_ptr = NonNull::from(event).cast::<c_void>();
    let key_event = convert_nsevent_to_key_event(0, nsevent_ptr);
    matches!(
        key_event.key_code,
        VKEY_RETURN
            | VKEY_TAB
            | VKEY_ESCAPE
            | VKEY_LEFT
            | VKEY_UP
            | VKEY_RIGHT
            | VKEY_DOWN
            | VKEY_PRIOR
            | VKEY_NEXT
    )
}

#[cfg(test)]
mod tests {
    use cbf::data::key::{KeyEvent, KeyEventType};
    use objc2::rc::Retained;
    use objc2_foundation::NSString;

    use super::{
        HoverDispatch, build_char_event, extract_insert_text, hover_transition,
        synthesized_char_text,
    };
    use crate::model::CompositionItemId;

    #[test]
    fn synthesized_char_text_prefers_event_text() {
        let event = KeyEvent::char_input(0, 0, 0, "a", "a");
        assert_eq!(synthesized_char_text(Some(&event)).as_deref(), Some("a"));
    }

    #[test]
    fn build_char_event_marks_char_type() {
        let event = build_char_event(None, "x".into()).unwrap();
        assert_eq!(event.type_, KeyEventType::Char);
        assert_eq!(event.text.as_deref(), Some("x"));
    }

    #[test]
    fn extract_insert_text_accepts_plain_string() {
        let text: Retained<NSString> = NSString::from_str("hello");
        assert_eq!(extract_insert_text(&text).as_deref(), Some("hello"));
    }

    #[test]
    fn hover_transition_emits_leave_enter_move_between_items() {
        let toolbar = CompositionItemId::new(1);
        let page = CompositionItemId::new(2);

        assert_eq!(
            hover_transition(Some(toolbar), Some(page)),
            vec![
                HoverDispatch::Leave(toolbar),
                HoverDispatch::Enter(page),
                HoverDispatch::Move(page),
            ]
        );
    }

    #[test]
    fn hover_transition_does_not_repeat_enter_or_leave_within_same_item() {
        let item = CompositionItemId::new(7);

        assert_eq!(
            hover_transition(Some(item), Some(item)),
            vec![HoverDispatch::Move(item)]
        );
    }

    #[test]
    fn hover_transition_emits_leave_when_pointer_exits_surface() {
        let item = CompositionItemId::new(9);

        assert_eq!(
            hover_transition(Some(item), None),
            vec![HoverDispatch::Leave(item)]
        );
    }

    #[test]
    fn hover_transition_emits_enter_then_move_on_initial_hover() {
        let item = CompositionItemId::new(11);

        assert_eq!(
            hover_transition(None, Some(item)),
            vec![HoverDispatch::Enter(item), HoverDispatch::Move(item)]
        );
    }
}
