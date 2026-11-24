//! Bridge between the Win32 host and this app when the host embeds it.
//!
//! The host owns the top-level window and Win32 behavior; it writes the
//! wanted island state here and the app polls it on the engine thread, so
//! no NSH signal is touched from the host's UI thread.

use std::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering},
    Mutex, OnceLock,
};

use nsh_core::prelude::*;

use std::time::Duration;

static EXPANDED: AtomicBool = AtomicBool::new(false);
static POINTER_THROUGH: AtomicBool = AtomicBool::new(false);
static CLIPBOARD_DIRTY: AtomicBool = AtomicBool::new(false);
static WECHAT_PENDING: AtomicBool = AtomicBool::new(false);
static WECHAT_LAST_ATTENTION: AtomicU64 = AtomicU64::new(0);
static WECHAT_ENABLED: AtomicBool = AtomicBool::new(true);
static WECHAT_AVAILABLE: AtomicBool = AtomicBool::new(false);
static WECHAT_FOCUS_REQUEST: AtomicBool = AtomicBool::new(false);
static SETTINGS_REQUEST: AtomicBool = AtomicBool::new(false);
static DROPPED_FILES: Mutex<Vec<String>> = Mutex::new(Vec::new());
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

pub fn request_settings() {
    SETTINGS_REQUEST.store(true, Ordering::Relaxed);
}

pub fn take_settings_request() -> bool {
    SETTINGS_REQUEST.swap(false, Ordering::Relaxed)
}
pub fn push_dropped_files(paths: Vec<String>) {
    if paths.is_empty() {
        return;
    }
    if let Ok(mut pending) = DROPPED_FILES.lock() {
        pending.extend(paths);
    }
}

pub fn take_dropped_files() -> Vec<String> {
    DROPPED_FILES
        .lock()
        .map(|mut pending| std::mem::take(&mut *pending))
        .unwrap_or_default()
}

pub fn mark_clipboard_dirty() {
    CLIPBOARD_DIRTY.store(true, Ordering::Relaxed);
}

pub fn take_clipboard_dirty() -> bool {
    CLIPBOARD_DIRTY.swap(false, Ordering::Relaxed)
}

pub fn set_wechat_enabled(enabled: bool) {
    WECHAT_ENABLED.store(enabled, Ordering::Relaxed);
}

pub fn wechat_enabled() -> bool {
    WECHAT_ENABLED.load(Ordering::Relaxed)
}

pub fn set_wechat_available(available: bool) {
    WECHAT_AVAILABLE.store(available, Ordering::Relaxed);
}

pub fn wechat_available() -> bool {
    WECHAT_AVAILABLE.load(Ordering::Relaxed)
}

pub fn set_wechat_pending(pending: bool, at: u64) {
    WECHAT_PENDING.store(pending, Ordering::Relaxed);
    WECHAT_LAST_ATTENTION.store(at, Ordering::Relaxed);
}

pub fn clear_wechat_pending() {
    WECHAT_PENDING.store(false, Ordering::Relaxed);
}

pub fn wechat_pending() -> Option<u64> {
    if WECHAT_PENDING.load(Ordering::Relaxed) {
        Some(WECHAT_LAST_ATTENTION.load(Ordering::Relaxed))
    } else {
        None
    }
}

pub fn request_focus_wechat() {
    WECHAT_FOCUS_REQUEST.store(true, Ordering::Relaxed);
}

pub fn take_focus_wechat_request() -> bool {
    WECHAT_FOCUS_REQUEST.swap(false, Ordering::Relaxed)
}

pub fn pointer_through() -> bool {
    POINTER_THROUGH.load(Ordering::Relaxed)
}

/// Polls the host's state on the engine thread and applies it to the UI.
pub fn start(expanded: Signal<bool>, settings_open: Signal<bool>) {
    spawn(async move {
        let mut through = false;
        loop {
            timer(Duration::from_millis(60)).await;
            if take_settings_request() {
                settings_open.set(true);
            }
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
                nsh_core::element::desktop::current_window().set(attributes);
            }
        }
    });
}
