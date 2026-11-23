//! Bridge between the Win32 host and this app when the host embeds it.
//!
//! The host owns the top-level window and Win32 behavior; it writes the
//! wanted island state here and the app polls it on the engine thread, so
//! no NSH signal is touched from the host's UI thread.

use std::sync::{
    atomic::{AtomicBool, Ordering},
    OnceLock,
};

use nsh_core::prelude::*;

use std::time::Duration;

static EXPANDED: AtomicBool = AtomicBool::new(false);
static POINTER_THROUGH: AtomicBool = AtomicBool::new(false);
static ON_EXPAND: OnceLock<fn(bool)> = OnceLock::new();

/// The host installs the function it uses to resize its window.
pub fn install(on_expand: fn(bool)) {
    let _ = ON_EXPAND.set(on_expand);
}

/// Called by the app's click handlers: expands or collapses the host window.
pub fn request_expanded(open: bool) {
    EXPANDED.store(open, Ordering::Relaxed);
    if let Some(on_expand) = ON_EXPAND.get() {
        on_expand(open);
    }
}

/// Called by the host's tray menu and global shortcut.
pub fn set_host_expanded(open: bool) {
    EXPANDED.store(open, Ordering::Relaxed);
}

pub fn wanted_expanded() -> bool {
    EXPANDED.load(Ordering::Relaxed)
}

pub fn set_pointer_through(through: bool) {
    POINTER_THROUGH.store(through, Ordering::Relaxed);
}

pub fn pointer_through() -> bool {
    POINTER_THROUGH.load(Ordering::Relaxed)
}

/// Polls the host's state on the engine thread and applies it to the UI.
pub fn start(expanded: Signal<bool>) {
    spawn(async move {
        let mut through = false;
        loop {
            timer(Duration::from_millis(60)).await;
            let wanted = wanted_expanded();
            if wanted != expanded.get_untracked() {
                expanded.set(wanted);
            }
            let next = pointer_through();
            if next != through {
                through = next;
                let mut attributes = nsh_core::desktop::WindowAttributes::default();
                attributes.pointer_events = Some(if next {
                    nsh_core::desktop::WindowPointerEvents::None
                } else {
                    nsh_core::desktop::WindowPointerEvents::Auto
                });
                attributes.focusable = Some(!next);
                attributes.activate = Some(!next);
                nsh_core::element::desktop::current_window().set(attributes);
            }
        }
    });
}
