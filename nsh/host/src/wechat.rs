//! WeChat taskbar attention, without reading any chat content.
//!
//! The shell sends HSHELL_FLASH when WeChat flashes its taskbar button. We only
//! match the window's process (Weixin.exe / WeChat.exe) and keep a pending flag;
//! opening the reminder brings that window forward.

use std::{
    path::Path,
    sync::atomic::{AtomicU32, Ordering},
};

use windows::{
    core::{w, BOOL, PWSTR},
    Win32::{
        Foundation::{CloseHandle, HWND, LPARAM},
        System::Threading::{
            AttachThreadInput, GetCurrentThreadId, OpenProcess, QueryFullProcessImageNameW,
            PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
        },
        UI::WindowsAndMessaging::{
            AllowSetForegroundWindow, EnumWindows, GetClassNameW, GetForegroundWindow,
            GetWindowThreadProcessId, IsGUIThread, IsIconic, IsWindowVisible, RegisterShellHookWindow,
            RegisterWindowMessageW, SetForegroundWindow, ShowWindow, HSHELL_REDRAW,
            HSHELL_WINDOWACTIVATED, HSHELL_WINDOWCREATED, SW_RESTORE,
        },
    },
};

use nsh_app_focusd_island_nsh::embedded;

const HSHELL_FLASH: u32 = HSHELL_REDRAW | 0x8000;
const HSHELL_RUDEAPPACTIVATED: u32 = HSHELL_WINDOWACTIVATED | 0x8000;

static HOOK_MESSAGE: AtomicU32 = AtomicU32::new(0);
static TICKS: AtomicU32 = AtomicU32::new(0);

pub fn register(window: HWND) -> Result<(), String> {
    // SAFETY: the argument is a static NUL-terminated string.
    let message = unsafe { RegisterWindowMessageW(w!("SHELLHOOK")) };
    if message == 0 {
        return Err("无法注册微信任务栏事件。".to_string());
    }
    HOOK_MESSAGE.store(message, Ordering::Relaxed);
    log::info!("wechat: hook message {message}");
    // SAFETY: the host window lives for the whole run.
    if !unsafe { RegisterShellHookWindow(window) }.as_bool() {
        return Err("无法监听微信任务栏提醒。".to_string());
    }
    embedded::set_wechat_available(probe_available());
    Ok(())
}

/// Returns true when this message was our shell hook message.
pub fn handle(message: u32, wparam: windows::Win32::Foundation::WPARAM, lparam: LPARAM) -> bool {
    if message == 0 || message != HOOK_MESSAGE.load(Ordering::Relaxed) {
        return false;
    }
    let kind = wparam.0 as u32;
    let target = HWND(lparam.0 as *mut _);
    let target_is_weixin = matches!(kind, HSHELL_FLASH | HSHELL_WINDOWACTIVATED | HSHELL_RUDEAPPACTIVATED | HSHELL_REDRAW | HSHELL_WINDOWCREATED)
        && is_weixin(target);
    if target_is_weixin || matches!(kind, HSHELL_WINDOWACTIVATED | HSHELL_RUDEAPPACTIVATED) {
        embedded::set_wechat_available(probe_available());
    }
    if target_is_weixin {
        match kind {
            HSHELL_FLASH => {
                if !foreground_is_weixin() && embedded::wechat_enabled() {
                    embedded::set_wechat_pending(true, now_ms());
                    log::info!("wechat: taskbar attention");
                }
            }
            HSHELL_WINDOWACTIVATED | HSHELL_RUDEAPPACTIVATED => {
                embedded::clear_wechat_pending();
            }
            _ => {}
        }
    }
    true
}

pub fn tick() {
    if TICKS.fetch_add(1, Ordering::Relaxed) % 60 == 0 {
        embedded::set_wechat_available(probe_available());
    }
}

pub fn focus() -> bool {
    let mut found: Option<HWND> = None;
    // SAFETY: the callback only reads window metadata and writes to the live
    // Option passed through lParam.
    unsafe {
        let _ = EnumWindows(
            Some(visit),
            LPARAM(&mut found as *mut Option<HWND> as isize),
        );
    }
    let Some(window) = found else {
        embedded::set_wechat_available(false);
        return false;
    };
    let process_id = process_id_of(window);
    // SAFETY: plain values; failed calls are harmless.
    unsafe {
        let _ = AllowSetForegroundWindow(process_id);
        if IsIconic(window).as_bool() {
            let _ = ShowWindow(window, SW_RESTORE);
        }
        if GetForegroundWindow() == window {
            embedded::clear_wechat_pending();
            return true;
        }
        if SetForegroundWindow(window).as_bool() && GetForegroundWindow() == window {
            embedded::clear_wechat_pending();
            return true;
        }
        let foreground = GetForegroundWindow();
        let (foreground_thread, own_thread) =
            (GetWindowThreadProcessId(foreground, None), GetCurrentThreadId());
        let attach = foreground_thread != 0 && foreground_thread != own_thread;
        let _ = IsGUIThread(true);
        let attached = attach && AttachThreadInput(own_thread, foreground_thread, true).as_bool();
        let _ = SetForegroundWindow(window);
        if attached {
            let _ = AttachThreadInput(own_thread, foreground_thread, false);
        }
        let focused = GetForegroundWindow() == window;
        if focused {
            embedded::clear_wechat_pending();
        }
        focused
    }
}

pub fn probe_available() -> bool {
    let mut found = false;
    // SAFETY: the callback only reads window metadata.
    unsafe {
        let _ = EnumWindows(Some(visit_available), LPARAM(&mut found as *mut bool as isize));
    }
    found
}

unsafe extern "system" fn visit(window: HWND, context: LPARAM) -> BOOL {
    // SAFETY: the context points at the Option owned by focus().
    let found = unsafe { &mut *(context.0 as *mut Option<HWND>) };
    if found.is_some() {
        return false.into();
    }
    // SAFETY: window metadata only.
    unsafe {
        if !IsWindowVisible(window).as_bool() {
            return true.into();
        }
    }
    if is_weixin(window) {
        *found = Some(window);
        return false.into();
    }
    true.into()
}

unsafe extern "system" fn visit_available(window: HWND, context: LPARAM) -> BOOL {
    // SAFETY: the context points at the bool owned by probe_available().
    let found = unsafe { &mut *(context.0 as *mut bool) };
    if *found {
        return false.into();
    }
    // SAFETY: window metadata only.
    unsafe {
        if IsWindowVisible(window).as_bool() && is_weixin(window) {
            *found = true;
            return false.into();
        }
    }
    true.into()
}

fn is_weixin(window: HWND) -> bool {
    if window.is_invalid() {
        return false;
    }
    let mut class = [0u16; 128];
    // SAFETY: the buffer is writable and the call writes at most its length.
    let length = unsafe { GetClassNameW(window, &mut class) } as usize;
    if length > 0 {
        let class = String::from_utf16_lossy(&class[..length]);
        if class.eq_ignore_ascii_case("mmui::MainWindow") {
            return true;
        }
    }
    let mut title = [0u16; 256];
    // SAFETY: the buffer is writable and the call writes at most its length.
    let length = unsafe { windows::Win32::UI::WindowsAndMessaging::GetWindowTextW(window, &mut title) } as usize;
    if length == 0 {
        return false;
    }
    let title = String::from_utf16_lossy(&title[..length]);
    title.contains("微信") || title.eq_ignore_ascii_case("weixin") || title.eq_ignore_ascii_case("wechat")
}

fn foreground_is_weixin() -> bool {
    // SAFETY: GetForegroundWindow takes no arguments.
    let window = unsafe { GetForegroundWindow() };
    is_weixin(window)
}

fn process_id_of(window: HWND) -> u32 {
    let mut process_id = 0;
    // SAFETY: writes into a valid local; invalid windows yield 0.
    unsafe { GetWindowThreadProcessId(window, Some(&mut process_id)) };
    process_id
}

#[allow(dead_code)]
fn image_stem(process_id: u32) -> Option<String> {
    // SAFETY: a handle returned by OpenProcess is owned here and closed below.
    let handle = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, process_id) }.ok()?;
    let mut path = [0u16; 32768];
    let mut length = path.len() as u32;
    // SAFETY: the handle is live and the buffer is writable.
    let result = unsafe {
        QueryFullProcessImageNameW(
            handle,
            PROCESS_NAME_WIN32,
            PWSTR(path.as_mut_ptr()),
            &mut length,
        )
    };
    // SAFETY: the handle came from OpenProcess and is closed once.
    let _ = unsafe { CloseHandle(handle) };
    result.ok()?;
    Path::new(&String::from_utf16_lossy(&path[..length as usize]))
        .file_stem()
        .map(|name| name.to_string_lossy().to_string())
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
