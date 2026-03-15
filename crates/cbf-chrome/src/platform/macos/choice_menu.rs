//! Default macOS presenter for host-owned choice menus.

use objc2::{MainThreadMarker, MainThreadOnly, rc::Retained, sel};
use objc2_app_kit::{NSControlStateValueOff, NSControlStateValueOn, NSMenu, NSMenuItem};
use objc2_foundation::{NSPoint, NSString};

use crate::data::choice_menu::{ChromeChoiceMenu, ChromeChoiceMenuItem, ChromeChoiceMenuItemType};

use super::browser_view::BrowserViewMac;

pub trait ChromeChoiceMenuPresenter {
    fn show_choice_menu(&self, view: &BrowserViewMac, menu: &ChromeChoiceMenu) -> Option<Vec<i32>>;
}

#[derive(Debug, Default, Clone, Copy)]
pub struct MacChoiceMenuPresenter;

impl ChromeChoiceMenuPresenter for MacChoiceMenuPresenter {
    fn show_choice_menu(&self, view: &BrowserViewMac, menu: &ChromeChoiceMenu) -> Option<Vec<i32>> {
        let mtm = MainThreadMarker::new().expect("BrowserViewMac must be on main thread");
        let mut next_selectable_index = 0;
        let ns_menu = build_ns_menu(
            mtm,
            &menu.items,
            view,
            menu.selected_item,
            &mut next_selectable_index,
        );
        let positioning_item = find_item_with_tag(&ns_menu, menu.selected_item as isize);

        let bounds = view.bounds();
        let x = menu.x as f64;
        let y = if view.isFlipped() {
            menu.y as f64
        } else {
            (bounds.size.height - menu.y as f64).max(0.0)
        };
        let location = NSPoint::new(x, y);

        let _ = ns_menu.popUpMenuPositioningItem_atLocation_inView(
            positioning_item.as_deref(),
            location,
            Some(view),
        );
        view.take_choice_menu_result().map(|action| vec![action])
    }
}

fn build_ns_menu(
    mtm: MainThreadMarker,
    items: &[ChromeChoiceMenuItem],
    target: &BrowserViewMac,
    selected_index: i32,
    next_selectable_index: &mut i32,
) -> Retained<NSMenu> {
    let title = NSString::from_str("");
    let menu = NSMenu::initWithTitle(NSMenu::alloc(mtm), &title);

    for item in items {
        if let Some(menu_item) =
            build_ns_menu_item(mtm, item, target, selected_index, next_selectable_index)
        {
            menu.addItem(&menu_item);
        }
    }

    menu
}

fn build_ns_menu_item(
    mtm: MainThreadMarker,
    item: &ChromeChoiceMenuItem,
    target: &BrowserViewMac,
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
            let submenu = build_ns_menu(
                mtm,
                &item.children,
                target,
                selected_index,
                next_selectable_index,
            );
            menu_item.setSubmenu(Some(&submenu));
            menu_item.setEnabled(item.enabled);
            if let Some(tool_tip) = item.tool_tip.as_deref().filter(|s| !s.is_empty()) {
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
            if let Some(tool_tip) = item.tool_tip.as_deref().filter(|s| !s.is_empty()) {
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
