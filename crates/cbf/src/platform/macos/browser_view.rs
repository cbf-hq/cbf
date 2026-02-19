//! macOS `NSView` implementation used to host Chromium rendering surfaces.
//!
//! This module defines `BrowserViewMac` and related delegate/event types for
//! translating native macOS input, IME, drag-and-drop, and context menu events.

#![allow(non_snake_case)]

use std::{
    cell::{Cell, RefCell},
    ffi::c_void,
    ptr::NonNull,
};

use objc2::{
    AnyThread, DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send,
    rc::Retained,
    runtime::{AnyObject, NSObject, ProtocolObject},
    sel,
};
use objc2_app_kit::{
    NSApplication, NSControlStateValueOff, NSControlStateValueOn, NSDragOperation,
    NSDraggingContext, NSDraggingItem, NSDraggingSession, NSDraggingSource, NSEvent,
    NSEventModifierFlags, NSEventType, NSImage, NSMenu, NSMenuItem, NSMenuItemBadge,
    NSPasteboardWriting, NSResponder, NSTextInputClient, NSView,
};
use objc2_core_foundation::{CGPoint, CGRect, CGSize};
use objc2_foundation::{
    NSArray, NSAttributedString, NSAttributedStringKey, NSData, NSNotFound, NSObjectProtocol,
    NSPoint, NSRange, NSRangePointer, NSRect, NSString, NSUInteger,
};
use objc2_quartz_core::CATransaction;

use crate::{
    data::{
        context_menu::{ContextMenu, ContextMenuIcon, ContextMenuItem, ContextMenuItemType},
        drag::DragStartRequest,
        ime::{ImeBoundsUpdate, ImeCompositionBounds, ImeRect, ImeTextRange, TextSelectionBounds},
        key::KeyEvent,
        mouse::{MouseEvent, MouseWheelEvent, PointerType},
    },
    ffi::{
        convert_nsevent_to_key_event, convert_nsevent_to_mouse_event,
        convert_nsevent_to_mouse_wheel_event,
    },
};

use super::bindings::{CALayerHost, ContextId};

/// Callback interface for BrowserViewMac input and menu events.
pub trait BrowserViewMacDelegate {
    /// Called when a key event is translated from macOS input.
    fn on_key_event(&self, view: &BrowserViewMac, event: KeyEvent, commands: Vec<String>);
    /// Called when an IME event is produced by the view.
    fn on_ime_event(&self, view: &BrowserViewMac, event: BrowserViewMacImeEvent);
    /// Called when plain character input is received.
    fn on_char_event(&self, view: &BrowserViewMac, text: String);
    /// Called when a mouse event is translated from macOS input.
    fn on_mouse_event(&self, view: &BrowserViewMac, event: MouseEvent);
    /// Called when a mouse wheel event is translated from macOS input.
    fn on_mouse_wheel_event(&self, view: &BrowserViewMac, event: MouseWheelEvent);
    /// Called when a context menu command is selected.
    fn on_context_menu_command(&self, view: &BrowserViewMac, menu_id: u64, command_id: i32);
    /// Called when a context menu is dismissed.
    fn on_context_menu_dismissed(&self, view: &BrowserViewMac, menu_id: u64);
    /// Called when NSResponder focus state for BrowserViewMac changed.
    fn on_focus_changed(&self, view: &BrowserViewMac, focused: bool);
    /// Called when native drag session moves.
    fn on_native_drag_update(
        &self,
        _view: &BrowserViewMac,
        _event: BrowserViewMacNativeDragUpdate,
    ) {
    }
    /// Called when native drag session ends with drop.
    fn on_native_drag_drop(&self, _view: &BrowserViewMac, _event: BrowserViewMacNativeDragDrop) {}
    /// Called when native drag session is cancelled.
    fn on_native_drag_cancel(&self, _view: &BrowserViewMac, _session_id: u64) {}
}

/// Configuration for constructing a BrowserViewMac instance.
pub struct BrowserViewMacConfig {
    pub frame: CGRect,
    pub delegate: Box<dyn BrowserViewMacDelegate>,
}

const NO_MENU_ID: u64 = 0;
const NO_COMMAND_ID: i32 = i32::MIN;

/// IME events emitted by BrowserViewMac.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BrowserViewMacImeEvent {
    /// Update IME composition text and selection.
    SetComposition {
        text: String,
        selection: Option<ImeTextRange>,
        replacement: Option<ImeTextRange>,
    },
    /// Commit IME composition text.
    CommitText {
        text: String,
        replacement: Option<ImeTextRange>,
        relative_caret_position: i32,
    },
    /// Finish IME composing with optional selection retention.
    FinishComposingText { keep_selection: bool },
}

#[derive(Debug, Clone, PartialEq)]
pub struct BrowserViewMacNativeDragUpdate {
    pub session_id: u64,
    pub modifiers: u32,
    pub position_in_widget_x: f32,
    pub position_in_widget_y: f32,
    pub position_in_screen_x: f32,
    pub position_in_screen_y: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BrowserViewMacNativeDragDrop {
    pub session_id: u64,
    pub modifiers: u32,
    pub position_in_widget_x: f32,
    pub position_in_widget_y: f32,
    pub position_in_screen_x: f32,
    pub position_in_screen_y: f32,
}

/// Internal state stored alongside the macOS view instance.
pub struct BrowserViewMacIvars {
    delegate: Box<dyn BrowserViewMacDelegate>,
    has_marked_text: Cell<bool>,
    marked_range: Cell<NSRange>,
    selected_range: Cell<NSRange>,
    ime_handled: Cell<bool>,
    suppress_key_up: Cell<bool>,
    ime_insert_expected: Cell<bool>,
    ime_bounds: RefCell<Option<ImeBoundsUpdate>>,
    browser_layer: Retained<CALayerHost>,
    browser_layer_frame: Cell<CGRect>,
    edit_commands: RefCell<Vec<String>>,
    context_menu_id: Cell<u64>,
    context_menu_selected_command_id: Cell<i32>,
    active_drag_source: RefCell<Option<Retained<AnyObject>>>,
}

define_class!(
    /// macOS NSView that hosts the Chromium rendering surface.
    #[unsafe(super(NSView, NSResponder, NSObject))]
    #[thread_kind = objc2::MainThreadOnly]
    #[name = "BrowserViewMac"]
    #[ivars = BrowserViewMacIvars]
    pub struct BrowserViewMac;

    impl BrowserViewMac {
        #[unsafe(method(keyDown:))]
        fn key_down(&self, event: &NSEvent) {
            self.ivars().ime_handled.set(false);
            self.ivars().suppress_key_up.set(false);
            self.ivars().edit_commands.borrow_mut().clear();

            let events = NSArray::arrayWithObject(event);
            self.interpretKeyEvents(&events);

            if self.ivars().ime_handled.get() {
                self.ivars().suppress_key_up.set(true);
                return;
            }

            self.forward_key_event(event);
        }

        #[unsafe(method(keyUp:))]
        fn key_up(&self, event: &NSEvent) {
            if self.ivars().suppress_key_up.replace(false) {
                return;
            }
            // KeyUp doesn't carry commands from interpretKeyEvents in the same way,
            // or at least we don't call interpretKeyEvents for KeyUp usually.
            self.forward_key_event(event);
        }

        #[unsafe(method(flagsChanged:))]
        fn flags_changed(&self, event: &NSEvent) {
            self.forward_key_event(event);
        }

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
            self.ivars().delegate.on_focus_changed(self, true);
            true
        }

        #[unsafe(method(resignFirstResponder))]
        fn resign_first_responder(&self) -> bool {
            self.ivars().delegate.on_focus_changed(self, false);
            true
        }

        #[unsafe(method(mouseDown:))]
        fn mouse_down(&self, event: &NSEvent) {
            self.forward_mouse_event(event);
        }

        #[unsafe(method(rightMouseDown:))]
        fn right_mouse_down(&self, event: &NSEvent) {
            self.forward_mouse_event(event);
        }

        #[unsafe(method(otherMouseDown:))]
        fn other_mouse_down(&self, event: &NSEvent) {
            self.forward_mouse_event(event);
        }

        #[unsafe(method(mouseUp:))]
        fn mouse_up(&self, event: &NSEvent) {
            self.forward_mouse_event(event);
        }

        #[unsafe(method(rightMouseUp:))]
        fn right_mouse_up(&self, event: &NSEvent) {
            self.forward_mouse_event(event);
        }

        #[unsafe(method(otherMouseUp:))]
        fn other_mouse_up(&self, event: &NSEvent) {
            self.forward_mouse_event(event);
        }

        #[unsafe(method(mouseMoved:))]
        fn mouse_moved(&self, event: &NSEvent) {
            self.forward_mouse_event(event);
        }

        #[unsafe(method(mouseDragged:))]
        fn mouse_dragged(&self, event: &NSEvent) {
            self.forward_mouse_event(event);
        }

        #[unsafe(method(rightMouseDragged:))]
        fn right_mouse_dragged(&self, event: &NSEvent) {
            self.forward_mouse_event(event);
        }

        #[unsafe(method(otherMouseDragged:))]
        fn other_mouse_dragged(&self, event: &NSEvent) {
            self.forward_mouse_event(event);
        }

        #[unsafe(method(mouseEntered:))]
        fn mouse_entered(&self, event: &NSEvent) {
            self.forward_mouse_event(event);
        }

        #[unsafe(method(mouseExited:))]
        fn mouse_exited(&self, event: &NSEvent) {
            self.forward_mouse_event(event);
        }

        #[unsafe(method(scrollWheel:))]
        fn scroll_wheel(&self, event: &NSEvent) {
            self.forward_mouse_wheel_event(event);
        }

        #[unsafe(method(contextMenuItemSelected:))]
        fn context_menu_item_selected(&self, sender: &AnyObject) {
            let tag: isize = unsafe { msg_send![sender, tag] };
            self.ivars()
                .context_menu_selected_command_id
                .set(tag as i32);
        }
    }

    unsafe impl NSTextInputClient for BrowserViewMac {
        #[unsafe(method(insertText:replacementRange:))]
        unsafe fn insertText_replacementRange(&self, string: &AnyObject, replacement_range: NSRange) {
            let Some(text) = extract_may_be_ns_attributed_string(string) else { return };

            if self.ivars().ime_insert_expected.get() {
                self.mark_ime_handled();
                self.ivars().ime_insert_expected.set(false);
                self.update_marked_state(false, ns_not_found_range(), ns_not_found_range());
                self.send_mac_ime_event(BrowserViewMacImeEvent::CommitText {
                    text,
                    replacement: nsrange_to_text_range(replacement_range),
                    relative_caret_position: 0,
                });
            } else {
                // Normal text input (not via IME composition).
                // We send a Char event. We do NOT mark IME handled, so that the
                // corresponding KeyDown event (if any) is also sent by key_down.
                self.send_mac_char_event(text);
            }
        }

        #[unsafe(method(doCommandBySelector:))]
        unsafe fn doCommandBySelector(&self, selector: objc2::runtime::Sel) {
            let mut command = selector.name().to_str().unwrap().to_string();
            if let Some(stripped) = command.strip_suffix(':') {
                command = stripped.to_string();
            }
            if command.to_ascii_lowercase().starts_with("insert") {
                // Chromium ignores insert* commands during key down to avoid
                // tab inserting text instead of moving focus.
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
            if let Some(text) = extract_may_be_ns_attributed_string(string) {
                self.mark_ime_handled();
                self.ivars().ime_insert_expected.set(true);
                let marked_range = composition_range_for_text(&text);
                self.update_marked_state(true, marked_range, selected_range);
                self.send_mac_ime_event(BrowserViewMacImeEvent::SetComposition {
                    text,
                    selection: nsrange_to_text_range(selected_range),
                    replacement: nsrange_to_text_range(replacement_range),
                });
            }
        }

        #[unsafe(method(unmarkText))]
        fn unmarkText(&self) {
            self.mark_ime_handled();
            let had_marked = self.ivars().has_marked_text.get();
            self.ivars().ime_insert_expected.set(had_marked);
            self.update_marked_state(false, ns_not_found_range(), ns_not_found_range());
            self.send_mac_ime_event(BrowserViewMacImeEvent::FinishComposingText {
                keep_selection: false,
            });
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

        // NOTE: The intended return type is `Option<Retained<NSAttributedString>>`,
        //   not `*const NSObject`, but since it does not satisfy the trait boundary,
        //   it returns a raw pointer.
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

        // NOTE: The intended return type is `Retained<NSArray<NSAttributedStringKey>>`,
        //   not `*const NSObject`, but since it does not satisfy the trait boundary,
        //   it returns a raw pointer.
        #[unsafe(method(validAttributesForMarkedText))]
        fn validAttributesForMarkedText(&self) -> *const AnyObject {
            let array: Retained<NSArray<NSAttributedStringKey>> = NSArray::new();
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
);

struct HostDragSourceIvars {
    view: Retained<BrowserViewMac>,
    session_id: u64,
    operation_mask: NSDragOperation,
}

define_class!(
    #[unsafe(super(NSObject))]
    #[thread_kind = objc2::MainThreadOnly]
    #[ivars = HostDragSourceIvars]
    struct HostDragSource;

    unsafe impl NSObjectProtocol for HostDragSource {}

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
            self.ivars()
                .view
                .emit_native_drag_update(self.ivars().session_id, screen_point);
        }

        #[unsafe(method(draggingSession:endedAtPoint:operation:))]
        fn draggingSession_endedAtPoint_operation(
            &self,
            _session: &NSDraggingSession,
            screen_point: NSPoint,
            operation: NSDragOperation,
        ) {
            let should_treat_as_drop =
                !operation.is_empty() || self.ivars().view.contains_screen_point(screen_point);
            if should_treat_as_drop {
                self.ivars()
                    .view
                    .emit_native_drag_drop(self.ivars().session_id, screen_point);
            } else {
                self.ivars()
                    .view
                    .emit_native_drag_cancel(self.ivars().session_id);
            }
            self.ivars().view.ivars().active_drag_source.replace(None);
        }
    }
);

impl HostDragSource {
    fn new(
        mtm: MainThreadMarker,
        view: Retained<BrowserViewMac>,
        session_id: u64,
        operation_mask: NSDragOperation,
    ) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(HostDragSourceIvars {
            view,
            session_id,
            operation_mask,
        });
        unsafe { msg_send![super(this), init] }
    }
}

impl BrowserViewMac {
    /// Create a new BrowserViewMac on the main thread.
    pub fn new(mtm: MainThreadMarker, config: BrowserViewMacConfig) -> Retained<Self> {
        let browser_layer = CALayerHost::init(CALayerHost::alloc());
        browser_layer.setFrame(config.frame);
        browser_layer.setGeometryFlipped(true);

        let this = Self::alloc(mtm).set_ivars(BrowserViewMacIvars {
            delegate: config.delegate,
            has_marked_text: Cell::new(false),
            marked_range: Cell::new(ns_not_found_range()),
            selected_range: Cell::new(ns_not_found_range()),
            ime_handled: Cell::new(false),
            suppress_key_up: Cell::new(false),
            ime_insert_expected: Cell::new(false),
            ime_bounds: RefCell::new(None),
            browser_layer,
            browser_layer_frame: Cell::new(config.frame),
            edit_commands: RefCell::new(Vec::new()),
            context_menu_id: Cell::new(NO_MENU_ID),
            context_menu_selected_command_id: Cell::new(NO_COMMAND_ID),
            active_drag_source: RefCell::new(None),
        });
        let this: Retained<Self> = unsafe { msg_send![super(this), init] };

        this.setFrame(config.frame);
        this.setWantsLayer(true);
        this.layer()
            .expect("BrowserViewMac must have a layer")
            .addSublayer(&this.ivars().browser_layer);

        this
    }

    /// Set the CALayerHost context id used to display Chromium content.
    pub fn set_context_id(&self, context_id: ContextId) {
        unsafe {
            self.ivars().browser_layer.setContextId(context_id);
        }
    }

    /// Update the layer frame without implicit animations.
    pub fn set_layer_frame(&self, frame: CGRect) {
        // Disable implicit animations for layer frame origin update.
        CATransaction::begin();
        CATransaction::setDisableActions(true);
        self.ivars().browser_layer.setFrame(frame);
        CATransaction::commit();

        self.ivars().browser_layer_frame.set(frame);
    }

    /// Update IME bounds so macOS can place candidate windows correctly.
    pub fn set_ime_bounds(&self, update: ImeBoundsUpdate) {
        self.ivars().ime_bounds.replace(Some(update));
    }

    /// Access the underlying CALayerHost used for rendering.
    pub fn browser_layer(&self) -> &CALayerHost {
        &self.ivars().browser_layer
    }

    /// Show a context menu built from backend menu data.
    pub fn show_context_menu(&self, menu: ContextMenu) {
        if self.window().is_none() {
            return;
        }

        self.ivars().context_menu_id.set(menu.menu_id);
        self.ivars()
            .context_menu_selected_command_id
            .set(NO_COMMAND_ID);

        let mtm = MainThreadMarker::new().expect("BrowserViewMac must be on main thread");
        let ns_menu = build_ns_menu(mtm, &menu.items, self);

        let bounds = self.bounds();
        let x = menu.x as f64;
        let y = if self.isFlipped() {
            menu.y as f64
        } else {
            (bounds.size.height - menu.y as f64).max(0.0)
        };
        let location = NSPoint::new(x, y);

        _ = ns_menu.popUpMenuPositioningItem_atLocation_inView(None, location, Some(self));

        let menu_id = self.ivars().context_menu_id.replace(NO_MENU_ID);
        let command_id = self
            .ivars()
            .context_menu_selected_command_id
            .replace(NO_COMMAND_ID);

        if menu_id == NO_MENU_ID {
            return;
        }

        if command_id == NO_COMMAND_ID {
            self.ivars()
                .delegate
                .on_context_menu_dismissed(self, menu_id);
        } else {
            self.ivars()
                .delegate
                .on_context_menu_command(self, menu_id, command_id);
        }
    }

    /// Start a native macOS drag session for host-owned drag lifecycle.
    pub fn start_native_drag_session(&self, request: &DragStartRequest) -> bool {
        let Some(window) = self.window() else {
            return false;
        };
        let Some(image_data) = request.image.as_ref() else {
            return false;
        };
        if image_data.png_bytes.is_empty() {
            return false;
        }

        let data = unsafe {
            NSData::initWithBytes_length(
                NSData::alloc(),
                image_data.png_bytes.as_ptr().cast(),
                image_data.png_bytes.len() as NSUInteger,
            )
        };
        let Some(image) = NSImage::initWithData(NSImage::alloc(), &data) else {
            return false;
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
            "Atelier"
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

        let mtm = MainThreadMarker::new().expect("BrowserViewMac must be on main thread");
        let drag_event = NSApplication::sharedApplication(mtm)
            .currentEvent()
            .or_else(|| {
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
            return false;
        };

        let operation_mask = NSDragOperation::from_bits_truncate(request.allowed_operations.bits() as _);
        let source = HostDragSource::new(
            mtm,
            Retained::from(self),
            request.session_id,
            operation_mask,
        );
        let source_ref: &ProtocolObject<dyn NSDraggingSource> = ProtocolObject::from_ref(&*source);
        let items = NSArray::arrayWithObject(&*dragging_item);

        self.beginDraggingSessionWithItems_event_source(&items, &drag_event, source_ref);

        self.ivars()
            .active_drag_source
            .replace(Some(source.into_super().into()));

        true
    }

    fn forward_key_event(&self, event: &NSEvent) {
        let nsevent_ptr = NonNull::from(event).cast::<c_void>();
        // Key conversion only needs keyboard fields here; routing target is resolved later.
        let key_event = convert_nsevent_to_key_event(0, nsevent_ptr);

        // Capture commands from doCommandBySelector
        let commands = std::mem::take(&mut *self.ivars().edit_commands.borrow_mut());

        self.ivars()
            .delegate
            .on_key_event(self, key_event, commands);
    }

    fn forward_mouse_event(&self, event: &NSEvent) {
        let nsevent_ptr = NonNull::from(event).cast::<c_void>();
        let nsview_ptr = NonNull::from(self).cast::<c_void>();
        let mouse_event =
            convert_nsevent_to_mouse_event(0, nsevent_ptr, nsview_ptr, PointerType::Mouse, false);
        self.ivars().delegate.on_mouse_event(self, mouse_event);
    }

    fn forward_mouse_wheel_event(&self, event: &NSEvent) {
        let nsevent_ptr = NonNull::from(event).cast::<c_void>();
        let nsview_ptr = NonNull::from(self).cast::<c_void>();
        let wheel_event = convert_nsevent_to_mouse_wheel_event(0, nsevent_ptr, nsview_ptr);
        self.ivars()
            .delegate
            .on_mouse_wheel_event(self, wheel_event);
    }

    fn send_mac_ime_event(&self, event: BrowserViewMacImeEvent) {
        self.ivars().delegate.on_ime_event(self, event);
    }

    fn send_mac_char_event(&self, text: String) {
        self.ivars().delegate.on_char_event(self, text);
    }

    fn update_marked_state(
        &self,
        has_marked_text: bool,
        marked_range: NSRange,
        selected_range: NSRange,
    ) {
        let ivars = self.ivars();
        ivars.has_marked_text.set(has_marked_text);
        ivars.marked_range.set(marked_range);
        ivars.selected_range.set(selected_range);
    }

    fn mark_ime_handled(&self) {
        self.ivars().ime_handled.set(true);
    }

    fn ime_candidate_rect(&self, range: NSRange) -> CGRect {
        let fallback = if let Some(window) = self.window() {
            window.frame()
        } else {
            CGRect::new(CGPoint::new(0.0, 0.0), CGSize::new(0.0, 0.0))
        };

        let bounds = self.ivars().ime_bounds.borrow();
        let Some(bounds) = bounds.as_ref() else {
            return fallback;
        };

        let layer_frame = self.ivars().browser_layer_frame.get();

        if let Some(composition) = bounds.composition.as_ref()
            && let Some(rect) = rect_for_composition_range(range, composition)
        {
            let rect = flip_rect_in_layer(rect, layer_frame.size.height);
            return self.to_screen_rect(offset_rect(
                rect,
                layer_frame.origin.x,
                layer_frame.origin.y,
            ));
        }

        if let Some(selection) = bounds.selection.as_ref() {
            let rect = flip_rect_in_layer(rect_from_selection(selection), layer_frame.size.height);
            return self.to_screen_rect(offset_rect(
                rect,
                layer_frame.origin.x,
                layer_frame.origin.y,
            ));
        }

        fallback
    }

    fn to_screen_rect(&self, rect: CGRect) -> CGRect {
        let window_rect = self.convertRect_toView(rect, None);

        if let Some(window) = self.window() {
            window.convertRectToScreen(window_rect)
        } else {
            window_rect
        }
    }

    fn emit_native_drag_update(&self, session_id: u64, screen_point: NSPoint) {
        let (widget_x, widget_y, screen_x, screen_y) = self.drag_points(screen_point);
        let modifiers = self
            .window()
            .and_then(|_| MainThreadMarker::new())
            .and_then(|mtm| NSApplication::sharedApplication(mtm).currentEvent())
            .map(|event| event.modifierFlags().bits() as u32)
            .unwrap_or(0);
        self.ivars().delegate.on_native_drag_update(
            self,
            BrowserViewMacNativeDragUpdate {
                session_id,
                modifiers,
                position_in_widget_x: widget_x,
                position_in_widget_y: widget_y,
                position_in_screen_x: screen_x,
                position_in_screen_y: screen_y,
            },
        );
    }

    fn emit_native_drag_drop(&self, session_id: u64, screen_point: NSPoint) {
        let (widget_x, widget_y, screen_x, screen_y) = self.drag_points(screen_point);
        let modifiers = self
            .window()
            .and_then(|_| MainThreadMarker::new())
            .and_then(|mtm| NSApplication::sharedApplication(mtm).currentEvent())
            .map(|event| event.modifierFlags().bits() as u32)
            .unwrap_or(0);
        self.ivars().delegate.on_native_drag_drop(
            self,
            BrowserViewMacNativeDragDrop {
                session_id,
                modifiers,
                position_in_widget_x: widget_x,
                position_in_widget_y: widget_y,
                position_in_screen_x: screen_x,
                position_in_screen_y: screen_y,
            },
        );
    }

    fn emit_native_drag_cancel(&self, session_id: u64) {
        self.ivars()
            .delegate
            .on_native_drag_cancel(self, session_id);
    }

    fn drag_points(&self, screen_point: NSPoint) -> (f32, f32, f32, f32) {
        let mut local_point = NSPoint::new(0.0, 0.0);
        if let Some(window) = self.window() {
            let base_point = window.convertPointFromScreen(screen_point);
            local_point = self.convertPoint_fromView(base_point, None);
        }
        let bounds = self.bounds();
        let widget_x = local_point.x as f32;
        let widget_y = (bounds.size.height - local_point.y).max(0.0) as f32;
        (
            widget_x,
            widget_y,
            screen_point.x as f32,
            screen_point.y as f32,
        )
    }

    fn contains_screen_point(&self, screen_point: NSPoint) -> bool {
        let Some(window) = self.window() else {
            return false;
        };
        let base_point = window.convertPointFromScreen(screen_point);
        let local_point = self.convertPoint_fromView(base_point, None);
        let bounds = self.bounds();
        local_point.x >= bounds.origin.x
            && local_point.x <= bounds.origin.x + bounds.size.width
            && local_point.y >= bounds.origin.y
            && local_point.y <= bounds.origin.y + bounds.size.height
    }
}

#[inline]
fn ns_not_found_range() -> NSRange {
    NSRange::new(NSNotFound as usize, 0)
}

fn build_ns_menu(
    mtm: MainThreadMarker,
    items: &[ContextMenuItem],
    target: &BrowserViewMac,
) -> Retained<NSMenu> {
    let title = NSString::from_str("");
    let menu = NSMenu::initWithTitle(NSMenu::alloc(mtm), &title);

    for item in items {
        if !item.visible {
            continue;
        }

        if let Some(menu_item) = build_ns_menu_item(mtm, item, target) {
            menu.addItem(&menu_item);
        }
    }

    menu
}

fn build_ns_menu_item(
    mtm: MainThreadMarker,
    item: &ContextMenuItem,
    target: &BrowserViewMac,
) -> Option<Retained<NSMenuItem>> {
    let title_text = menu_item_title(item);

    let menu_item = match item.r#type {
        ContextMenuItemType::Separator => {
            return Some(NSMenuItem::separatorItem(mtm));
        }
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
        let submenu = build_ns_menu(mtm, &item.submenu, target);
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
        if item.checked {
            menu_item.setState(NSControlStateValueOn);
        } else {
            menu_item.setState(NSControlStateValueOff);
        }
    }

    Some(menu_item)
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

fn nsrange_to_text_range(range: NSRange) -> Option<ImeTextRange> {
    if range.location == NSNotFound as usize {
        return None;
    }

    let start = range.location.min(i32::MAX as usize) as i32;
    let end = range.end().min(i32::MAX as usize).max(start as usize) as i32;

    Some(ImeTextRange { start, end })
}

fn composition_range_for_text(text: &str) -> NSRange {
    let length = text.encode_utf16().count();
    NSRange::new(0, length)
}

fn extract_may_be_ns_attributed_string(value: &AnyObject) -> Option<String> {
    let mut text = if let Some(attributed) = value.downcast_ref::<NSAttributedString>() {
        Some(attributed.string().to_string())
    } else {
        value
            .downcast_ref::<NSString>()
            .map(|ns_string| ns_string.to_string())
    }?;

    // Sanitize the text by removing control characters.
    text = text.chars().filter(|c| !c.is_control()).collect::<String>();

    if text.is_empty() {
        return None;
    }

    Some(text)
}

fn rect_for_composition_range(
    range: NSRange,
    composition: &ImeCompositionBounds,
) -> Option<CGRect> {
    if composition.range_start < 0 || composition.range_end < composition.range_start {
        return None;
    }
    if composition.character_bounds.is_empty() {
        return None;
    }
    if range.location == NSNotFound as usize {
        return None;
    }

    let start = range.location.min(i32::MAX as usize) as i32;
    let end = range.end().min(i32::MAX as usize).max(range.location) as i32;

    if start < composition.range_start || end > composition.range_end {
        return None;
    }

    let local_start = (start - composition.range_start) as usize;
    if local_start >= composition.character_bounds.len() {
        return None;
    }

    if range.length == 0 {
        return Some(rect_from_ime(&composition.character_bounds[local_start]));
    }

    let local_end = (end - composition.range_start) as usize;
    let clamped_end = local_end.min(composition.character_bounds.len());
    if clamped_end <= local_start {
        return Some(rect_from_ime(&composition.character_bounds[local_start]));
    }

    let mut rect = rect_from_ime(&composition.character_bounds[local_start]);
    for bounds in &composition.character_bounds[local_start + 1..clamped_end] {
        rect = union_rect(rect, rect_from_ime(bounds));
    }

    Some(rect)
}

fn rect_from_selection(selection: &TextSelectionBounds) -> CGRect {
    rect_from_ime(&selection.caret_rect)
}

fn rect_from_ime(rect: &ImeRect) -> CGRect {
    CGRect::new(
        CGPoint::new(rect.x as f64, rect.y as f64),
        CGSize::new(rect.width as f64, rect.height as f64),
    )
}

fn union_rect(a: CGRect, b: CGRect) -> CGRect {
    let min_x = a.origin.x.min(b.origin.x);
    let min_y = a.origin.y.min(b.origin.y);
    let max_x = (a.origin.x + a.size.width).max(b.origin.x + b.size.width);
    let max_y = (a.origin.y + a.size.height).max(b.origin.y + b.size.height);

    CGRect::new(
        CGPoint::new(min_x, min_y),
        CGSize::new((max_x - min_x).max(0.0), (max_y - min_y).max(0.0)),
    )
}

fn offset_rect(rect: CGRect, offset_x: f64, offset_y: f64) -> CGRect {
    CGRect::new(
        CGPoint::new(rect.origin.x + offset_x, rect.origin.y + offset_y),
        rect.size,
    )
}

fn flip_rect_in_layer(rect: CGRect, layer_height: f64) -> CGRect {
    let flipped_y = (layer_height - (rect.origin.y + rect.size.height)).max(0.0);
    CGRect::new(CGPoint::new(rect.origin.x, flipped_y), rect.size)
}
