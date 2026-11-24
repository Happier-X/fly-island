//! Phase 2 host: a Win32 message loop around one NSH AppView.
//!
//! NSH's child-window mode turns transparent off, so the island keeps its
//! per-pixel alpha as a top-level AppView; this host owns the HWND after
//! creation, its message loop, bounds, topmost level, tray, hotkeys and the
//! click-through/focus probes.

#![cfg_attr(windows, windows_subsystem = "windows")]

#[cfg(not(windows))]
fn main() {
    eprintln!("the Phase 2 host is Windows-only");
}

#[cfg(windows)]
fn main() -> std::process::ExitCode {
    host::run()
}

#[cfg(windows)]
mod island;
#[cfg(windows)]
mod tray;
#[cfg(windows)]
mod wechat;

#[cfg(windows)]
mod host {
    use std::sync::atomic::{AtomicBool, AtomicIsize, Ordering};

    use nsh_platform_windows::{App, AppOptions, Appearance, AppView, Root};
    use windows::{
        Win32::{
            Foundation::{HWND, LPARAM, LRESULT, WPARAM},
            UI::Input::KeyboardAndMouse::{RegisterHotKey, SetFocus, MOD_CONTROL, MOD_SHIFT},
            System::{
                DataExchange::{AddClipboardFormatListener, RemoveClipboardFormatListener},
                LibraryLoader::GetModuleHandleW,
                Ole::{OleInitialize, OleUninitialize},
                Threading::GetCurrentProcessId,
            },
            UI::{
                HiDpi::{DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, SetProcessDpiAwarenessContext},
                Shell::{DefSubclassProc, DragAcceptFiles, DragFinish, DragQueryFileW, HDROP, SetWindowSubclass},
                WindowsAndMessaging::{
                    CreateWindowExW, DefWindowProcW, DispatchMessageW, GetGUIThreadInfo, GetMessageW,
                    GetWindowThreadProcessId, GUITHREADINFO, IsChild, MSG, PostMessageW,
                    PostQuitMessage, RegisterClassW, SetForegroundWindow,
                    SetTimer, ShowWindow, TranslateMessage, WNDCLASSW, WM_APP, WM_DESTROY,
                    WM_CLIPBOARDUPDATE, WM_DROPFILES, WM_HOTKEY, WM_TIMER, WS_OVERLAPPED, SW_SHOW,
                },
            },
        },
        core::PCWSTR,
    };

    use crate::{island, tray, wechat};

    pub const APP_ID: &str = "com.focusd.island.nsh";
    pub const HOTKEY_TOGGLE: i32 = 1;
    pub const HOTKEY_THROUGH: i32 = 2;
    pub const HOTKEY_MONITOR: i32 = 3;
    pub const HOTKEY_TUCK: i32 = 4;
    pub const WM_HOST_EXPAND: u32 = WM_APP + 1;
    pub const WM_HOST_TRAY: u32 = WM_APP + 2;
    pub const WM_HOST_TIMER: usize = 1;
    pub const WM_HOST_DRAG: u32 = WM_APP + 3;
    pub const WM_HOST_PICK: u32 = WM_APP + 4;

    static HOST_HWND: AtomicIsize = AtomicIsize::new(0);
    static APP_HWND: AtomicIsize = AtomicIsize::new(0);
    pub static EXPANDED: AtomicBool = AtomicBool::new(false);

    pub fn host_hwnd() -> HWND {
        HWND(HOST_HWND.load(Ordering::Relaxed) as *mut _)
    }

    pub fn app_hwnd() -> HWND {
        HWND(APP_HWND.load(Ordering::Relaxed) as *mut _)
    }

    fn wide(text: &str) -> Vec<u16> {
        text.encode_utf16().chain(Some(0)).collect()
    }

    /// Called by the app on its engine thread; the window work is posted to
    /// the host thread.
    pub fn on_pick() {
        let host = host_hwnd();
        if !host.0.is_null() {
            // SAFETY: the host window lives for the process lifetime.
            unsafe {
                let _ = PostMessageW(Some(host), WM_HOST_PICK, WPARAM(0), LPARAM(0));
            }
        }
    }

    pub fn on_drag() {
        let host = host_hwnd();
        if !host.0.is_null() {
            // SAFETY: the host window lives for the process lifetime.
            unsafe {
                let _ = PostMessageW(Some(host), WM_HOST_DRAG, WPARAM(0), LPARAM(0));
            }
        }
    }

    pub fn on_expand(open: bool) {
        let host = host_hwnd();
        if !host.0.is_null() {
            // SAFETY: the host window lives for the whole run.
            unsafe {
                let _ = PostMessageW(Some(host), WM_HOST_EXPAND, WPARAM(open as usize), LPARAM(0));
            }
        }
    }

    pub fn set_expanded(open: bool) {
        EXPANDED.store(open, Ordering::Relaxed);
        nsh_app_focusd_island_nsh::embedded::set_host_expanded(open);
        island::apply(open);
    }

    pub fn toggle() {
        let open = !EXPANDED.load(Ordering::Relaxed);
        set_expanded(open);
        let app = app_hwnd();
        if !app.0.is_null() {
            // SAFETY: the view's window lives for the whole run.
            unsafe {
                let _ = ShowWindow(app, SW_SHOW);
                if open {
                    let _ = SetForegroundWindow(app);
                }
            }
        }
    }

    pub fn toggle_pointer_through() {
        let through = !nsh_app_focusd_island_nsh::embedded::pointer_through();
        nsh_app_focusd_island_nsh::embedded::set_pointer_through(through);
        if through {
            release_focus();
        }
        log::info!("pointer through: {through}");
    }

    fn release_focus() {
        let app = app_hwnd();
        let tid = unsafe { GetWindowThreadProcessId(app, None) };
        let mut info = GUITHREADINFO {
            cbSize: std::mem::size_of::<GUITHREADINFO>() as u32,
            ..Default::default()
        };
        if nsh_app_focusd_island_nsh::focus::input_focused() {
            log::info!("pointer through: island input keeps focus");
            return;
        }
        let known = unsafe { GetGUIThreadInfo(tid, &mut info) }.is_ok();
        if known
            && !info.hwndFocus.0.is_null()
            && info.hwndFocus != app
            && unsafe { IsChild(app, info.hwndFocus) }.as_bool()
        {
            log::info!("pointer through: island input keeps focus");
            return;
        }
        let last = island::last_foreground();
        if !last.0.is_null() && last != app {
            let moved = unsafe { SetForegroundWindow(last) }.as_bool();
            log::info!("pointer through: focus returned to previous foreground ({moved})");
            if moved {
                return;
            }
        }
        unsafe {
            let _ = SetFocus(None);
        }
        log::info!("pointer through: keyboard focus cleared");
    }    unsafe extern "system" fn host_proc(
        hwnd: HWND,
        message: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        if wechat::handle(message, wparam, lparam) {
            return LRESULT(0);
        }
        match message {
            WM_HOST_EXPAND => {
                set_expanded(wparam.0 != 0);
                LRESULT(0)
            }
            WM_HOTKEY => {
                if wparam.0 as i32 == HOTKEY_TOGGLE {
                    toggle();
                } else if wparam.0 as i32 == HOTKEY_THROUGH {
                    toggle_pointer_through();
                } else if wparam.0 as i32 == HOTKEY_MONITOR {
                    island::next_monitor();
                } else if wparam.0 as i32 == HOTKEY_TUCK {
                    island::toggle_tuck();
                }
                LRESULT(0)
            }
            WM_CLIPBOARDUPDATE => {
                nsh_app_focusd_island_nsh::embedded::mark_clipboard_dirty();
                LRESULT(0)
            }
            WM_HOST_PICK => {
                let result = nsh_app_focusd_island_nsh::file_shelf_picker::pick_files(app_hwnd());
                nsh_app_focusd_island_nsh::embedded::set_pick_result(result);
                LRESULT(0)
            }

            WM_HOST_DRAG => {
                let paths = nsh_app_focusd_island_nsh::embedded::take_drag_paths();
                let result = nsh_app_focusd_island_nsh::file_shelf_drag::drag_files(app_hwnd(), &paths)
                    .map(|_| ());
                nsh_app_focusd_island_nsh::embedded::set_drag_result(result);
                LRESULT(0)
            }

            WM_HOST_TRAY => {
                tray::on_message(hwnd, lparam);
                LRESULT(0)
            }
            WM_TIMER if wparam.0 == WM_HOST_TIMER => {
                island::watch();
                wechat::tick();
                if nsh_app_focusd_island_nsh::embedded::take_focus_wechat_request() {
                    let _ = wechat::focus();
                }
                LRESULT(0)
            }
            WM_DESTROY => {
                // SAFETY: the tray icon belongs to this window.
                unsafe {
                    let _ = RemoveClipboardFormatListener(hwnd);
                    tray::remove(hwnd);
                    PostQuitMessage(0);
                }
                LRESULT(0)
            }
            _ => unsafe { DefWindowProcW(hwnd, message, wparam, lparam) },
        }
    }

    unsafe extern "system" fn app_subclass(
        hwnd: HWND,
        message: u32,
        wparam: WPARAM,
        lparam: LPARAM,
        _id: usize,
        _data: usize,
    ) -> LRESULT {
        if message == WM_DROPFILES {
            let drop = HDROP(wparam.0 as *mut _);
            // SAFETY: WM_DROPFILES carries a live HDROP owned by this message.
            let count = unsafe { DragQueryFileW(drop, u32::MAX, None) };
            let mut paths = Vec::new();
            for index in 0..count {
                // SAFETY: `index` is below the count above.
                let length = unsafe { DragQueryFileW(drop, index, None) };
                if length == 0 {
                    continue;
                }
                let mut buffer = vec![0u16; length as usize + 1];
                // SAFETY: the buffer is large enough and stays alive for the call.
                let copied = unsafe { DragQueryFileW(drop, index, Some(&mut buffer)) };
                if copied > 0 {
                    paths.push(String::from_utf16_lossy(&buffer[..copied as usize]));
                }
            }
            // SAFETY: the drop handle is released exactly once here.
            unsafe { DragFinish(drop) };
            log::info!("file drops: {} files", paths.len());
            nsh_app_focusd_island_nsh::embedded::push_dropped_files(paths);
            return LRESULT(0);
        }
        // SAFETY: the subclass forwards everything else to AppView's own procedure.
        unsafe { DefSubclassProc(hwnd, message, wparam, lparam) }
    }

    pub fn run() -> std::process::ExitCode {
        env_logger::init();
        // SAFETY: process setup before any window exists.
        unsafe {
            let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
            let _ = OleInitialize(None);
        }
        log::info!("host pid {}", unsafe { GetCurrentProcessId() });

        let class_name = wide("FocusdIslandHost");
        let title = wide("FocuSD Island host");
        // SAFETY: registering one class before creating its window.
        let host = unsafe {
            let instance = match GetModuleHandleW(None) {
                Ok(module) => module,
                Err(error) => {
                    log::error!("GetModuleHandleW: {error}");
                    return std::process::ExitCode::FAILURE;
                }
            };
            let window_class = WNDCLASSW {
                lpfnWndProc: Some(host_proc),
                hInstance: windows::Win32::Foundation::HINSTANCE(instance.0),
                lpszClassName: PCWSTR(class_name.as_ptr()),
                ..Default::default()
            };
            let _ = RegisterClassW(&window_class);
            match CreateWindowExW(
                Default::default(),
                PCWSTR(class_name.as_ptr()),
                PCWSTR(title.as_ptr()),
                WS_OVERLAPPED,
                0,
                0,
                0,
                0,
                None,
                None,
                Some(windows::Win32::Foundation::HINSTANCE(instance.0)),
                None,
            ) {
                Ok(hwnd) => hwnd,
                Err(error) => {
                    log::error!("CreateWindowExW: {error}");
                    return std::process::ExitCode::FAILURE;
                }
            }
        };
        HOST_HWND.store(host.0 as isize, Ordering::Relaxed);
        if let Err(error) = wechat::register(host) {
            log::warn!("{error}");
        }

        nsh_app_focusd_island_nsh::embedded::install(on_expand);
        nsh_app_focusd_island_nsh::embedded::install_drag(on_drag);
        nsh_app_focusd_island_nsh::embedded::install_pick(on_pick);
        nsh_app_focusd_island_nsh::embedded::set_host_expanded(false);

        let view = match AppView::new(
            None,
            AppOptions {
                title: "FocuSD Island (NSH host)".to_owned(),
                app_id: APP_ID.to_owned(),
                size: (320, 40),
                transparent: true,
                skip_taskbar: true,
                appearance: Appearance::Dark,
                ..Default::default()
            },
        ) {
            Ok(view) => view,
            Err(error) => {
                log::error!("AppView::new: {error}");
                return std::process::ExitCode::FAILURE;
            }
        };
        APP_HWND.store(view.hwnd().0 as isize, Ordering::Relaxed);

        let root = Root::of::<nsh_app_focusd_island_nsh::App>()
            .app_id(APP_ID)
            .translations_json(include_str!(concat!(env!("OUT_DIR"), "/translations.json")))
            .capabilities_json(
                r#"{"windows":{"kinds":["normal","overlay","panel","dialog"],"levels":["normal","floating","aboveDock"],"max":8,"overlay":true},"clipboard":true,"application":true,"fs":{"read":["${dataDir}"]}}"#
            )
            .appearance(Appearance::Dark);
        view.load(App::Rust(root));
        island::log_displays();
        view.show();
        let app = app_hwnd();
        // SAFETY: the AppView window is live and owns the subclass for the process lifetime.
        unsafe {
            DragAcceptFiles(app, true);
            let _ = SetWindowSubclass(app, Some(app_subclass), 1, 0);
            log::info!("file drops accepted on AppView");
        }

        // SAFETY: host window exists; hotkeys and tray belong to this thread.
        unsafe {
            if let Err(error) = RegisterHotKey(
                Some(host),
                HOTKEY_TOGGLE,
                MOD_CONTROL | MOD_SHIFT,
                'I' as u32,
            ) {
                log::warn!("Ctrl+Shift+I not registered: {error}");
            }
            if let Err(error) = RegisterHotKey(
                Some(host),
                HOTKEY_THROUGH,
                MOD_CONTROL | MOD_SHIFT,
                'O' as u32,
            ) {
                log::warn!("Ctrl+Shift+O not registered: {error}");
            }
            if let Err(error) = RegisterHotKey(Some(host), HOTKEY_MONITOR, MOD_CONTROL | MOD_SHIFT, 'M' as u32) {
                log::warn!("Ctrl+Shift+M not registered: {error}");
            }
            if let Err(error) = RegisterHotKey(Some(host), HOTKEY_TUCK, MOD_CONTROL | MOD_SHIFT, 'T' as u32) {
                log::warn!("Ctrl+Shift+T not registered: {error}");
            }
            if AddClipboardFormatListener(host).is_err() {
                log::warn!("AddClipboardFormatListener failed");
            }
            tray::add(host);
            island::apply(false);
            SetTimer(Some(host), WM_HOST_TIMER, 400, None);
        }

        let mut message = MSG::default();
        // SAFETY: the standard message loop for this thread's windows.
        unsafe {
            while GetMessageW(&mut message, None, 0, 0).as_bool() {
                let _ = TranslateMessage(&message);
                DispatchMessageW(&message);
            }
        }
        view.stop();
        // SAFETY: pairs OleInitialize; nothing on this thread uses OLE after.
        unsafe {
            OleUninitialize();
        }
        log::info!("host exited");
        std::process::ExitCode::SUCCESS
    }

}
