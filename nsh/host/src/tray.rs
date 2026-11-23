//! The host's tray icon and menu. The app's own tray is off in embedded
//! mode; this one is the Win32 fallback for the Phase 2 probe.

use std::mem::zeroed;

use windows::{
    Win32::{
        Foundation::{HWND, LPARAM, WPARAM},
        UI::{
            Shell::{
                NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NOTIFYICONDATAW,
                Shell_NotifyIconW,
            },
            WindowsAndMessaging::{
                AppendMenuW, CreatePopupMenu, DestroyMenu, GetCursorPos, IDI_APPLICATION, LoadIconW,
                MF_SEPARATOR, MF_STRING, PostMessageW, SetForegroundWindow, TPM_RETURNCMD,
                TPM_RIGHTBUTTON, TrackPopupMenu, WM_CLOSE, WM_LBUTTONUP, WM_RBUTTONUP,
            },
        },
    },
    core::PCWSTR,
};

use crate::{
    host::{WM_HOST_TRAY, set_expanded, toggle, toggle_pointer_through},
    island,
};

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(Some(0)).collect()
}

fn icon_data(host: HWND) -> NOTIFYICONDATAW {
    // SAFETY: NOTIFYICONDATAW is plain data; zero is its empty state.
    let mut data: NOTIFYICONDATAW = unsafe { zeroed() };
    data.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
    data.hWnd = host;
    data.uID = 1;
    data.uFlags = NIF_MESSAGE | NIF_ICON | NIF_TIP;
    data.uCallbackMessage = WM_HOST_TRAY;
    data.hIcon = unsafe { LoadIconW(None, IDI_APPLICATION) }.unwrap_or_default();
    for (slot, unit) in data.szTip.iter_mut().zip("焦点岛".encode_utf16()) {
        *slot = unit;
    }
    data
}

pub fn add(host: HWND) {
    let data = icon_data(host);
    // SAFETY: adding this window's icon.
    if !unsafe { Shell_NotifyIconW(NIM_ADD, &data) }.as_bool() {
        log::warn!("Shell_NotifyIconW(NIM_ADD) failed");
    }
}

pub fn remove(host: HWND) {
    let data = icon_data(host);
    // SAFETY: removing this window's icon.
    let _ = unsafe { Shell_NotifyIconW(NIM_DELETE, &data) };
}

pub fn on_message(host: HWND, lparam: LPARAM) {
    let event = lparam.0 as u32 & 0xFFFF;
    if event == WM_LBUTTONUP {
        toggle();
    } else if event == WM_RBUTTONUP {
        menu(host);
    }
}

fn menu(host: HWND) {
    // SAFETY: the menu and window belong to this UI thread.
    unsafe {
        let _ = SetForegroundWindow(host);
        let Ok(menu) = CreatePopupMenu() else {
            return;
        };
        let toggle_item = wide("显示 / 收起岛屿");
        let tuck_item = wide("贴边隐藏 / 露出");
        let through_item = wide("切换点击穿透");
        let monitor_item = wide("移到下一块屏幕");
        let quit_item = wide("退出");
        let _ = AppendMenuW(menu, MF_STRING, 1, PCWSTR(toggle_item.as_ptr()));
        let _ = AppendMenuW(menu, MF_STRING, 2, PCWSTR(through_item.as_ptr()));
        let _ = AppendMenuW(menu, MF_STRING, 5, PCWSTR(tuck_item.as_ptr()));
        let _ = AppendMenuW(menu, MF_STRING, 3, PCWSTR(monitor_item.as_ptr()));
        let _ = AppendMenuW(menu, MF_SEPARATOR, 0, PCWSTR::null());
        let _ = AppendMenuW(menu, MF_STRING, 4, PCWSTR(quit_item.as_ptr()));
        let mut point = zeroed();
        let _ = GetCursorPos(&mut point);
        let command = TrackPopupMenu(
            menu,
            TPM_RETURNCMD | TPM_RIGHTBUTTON,
            point.x,
            point.y,
            Some(0),
            host,
            None,
        )
        .0 as i32;
        let _ = DestroyMenu(menu);
        match command {
            1 => toggle(),
            2 => toggle_pointer_through(),
            3 => island::next_monitor(),
            5 => island::toggle_tuck(),
            4 => {
                let _ = PostMessageW(Some(host), WM_CLOSE, WPARAM(0), LPARAM(0));
            }
            _ => {}
        }
    }
}

/// Used by the click-through probe: show the internal state.
pub fn _state() -> (bool, i32) {
    (
        nsh_app_focusd_island_nsh::embedded::pointer_through(),
        nsh_app_focusd_island_nsh::embedded::wanted_expanded() as i32,
    )
}

fn _set(open: bool) {
    set_expanded(open);
}
