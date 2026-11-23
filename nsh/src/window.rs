//! The main window as an island: frameless, translucent, above the taskbar,
//! top-centered on the primary display; a tray icon and one global shortcut.

use std::time::Duration;

use nsh_core::{
    desktop::{WindowAttributes, WindowFrame, WindowFrameStyle, WindowKind, WindowLevel},
    element::desktop::{
        current_window, displays, main_window, quit, register_global_shortcut, set_tray, tray,
        MenuItem,
    },
    prelude::*,
};

use crate::{COLLAPSED_HEIGHT, COLLAPSED_WIDTH, EXPANDED_HEIGHT, EXPANDED_WIDTH};

const SHORTCUT: &str = "mod+shift+i";
const TOP_GAP: f32 = 8.0;

pub fn configure(expanded: Signal<bool>) {
    let mut attributes = WindowAttributes::for_kind(WindowKind::Normal);
    attributes.level = Some(WindowLevel::AboveDock);
    attributes.transparent = Some(true);
    attributes.frame = Some(WindowFrameStyle::None);
    attributes.taskbar = Some(false);
    attributes.resizable = Some(false);
    attributes.shadow = Some(false);
    attributes.activate = Some(false);
    attributes.focusable = Some(true);
    attributes.close_on_press = Some(false);
    let win = current_window();
    win.set(attributes);
    let close_win = win.clone();
    win.on_close_press(move |_| close_win.hide());
    // The platform applies nsh.json's position after the first commit: place
    // again once the frame is up, and on every display change.
    effect(move || {
        let _ = displays();
        set_timeout(Duration::ZERO, move || place(expanded.get()));
    });
}

pub fn set_expanded(expanded: Signal<bool>, open: bool) {
    expanded.set(open);
    place(open);
}

pub fn install_tray(expanded: Signal<bool>) {
    effect(move || {
        let show = t!("island.show");
        let quit_label = t!("island.quit");
        set_tray(
            tray()
                .icon(asset!("tray.svg"))
                .tooltip(t!("island.title"))
                .on_click(move |_| toggle_island(expanded))
                .menu([
                    MenuItem::new(show).on_select(move || toggle_island(expanded)),
                    MenuItem::separator(),
                    MenuItem::new(quit_label).on_select(quit),
                ]),
        );
    });
}

pub fn install_shortcut(expanded: Signal<bool>) {
    let taken = register_global_shortcut(SHORTCUT, move || toggle_island(expanded));
    spawn(async move {
        if !taken.await {
            log::warn!("{SHORTCUT} is taken by another app");
        }
    });
}

fn toggle_island(expanded: Signal<bool>) {
    let win = main_window();
    let visible = win.state().map_or(true, |state| state.visible);
    let open = if visible { !expanded.get() } else { true };
    expanded.set(open);
    place(open);
    win.show();
    if open {
        win.focus();
    }
}

/// Centers the window on the primary display's top edge. NSH has no
/// top-center preset, so the app computes the frame and sets it.
fn place(expanded: bool) {
    let (width, height) = if expanded {
        (EXPANDED_WIDTH, EXPANDED_HEIGHT)
    } else {
        (COLLAPSED_WIDTH, COLLAPSED_HEIGHT)
    };
    let all = displays();
    let area = all
        .iter()
        .find(|display| display.primary)
        .or_else(|| all.first());
    let Some(area) = area else {
        return;
    };
    let visible = area.visible_bounds;
    let x = visible.origin.x.0 + (visible.size.width.0 - width) / 2.0;
    let y = visible.origin.y.0 + TOP_GAP;
    current_window().set_bounds(WindowFrame {
        x: Some(px(x)),
        y: Some(px(y)),
        width: Some(px(width)),
        height: Some(px(height)),
        animated: Some(false),
    });
}
