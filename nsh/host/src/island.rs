//! Host-side bounds, tuck, fullscreen and monitor/DPI probes.

use std::sync::atomic::{AtomicBool, AtomicI32, AtomicIsize, Ordering};

use windows::{
    Win32::{
        Foundation::{HWND, LPARAM, POINT, RECT},
        Graphics::Gdi::{
            EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR, MONITOR_DEFAULTTONEAREST,
            MONITORINFO, MonitorFromWindow,
        },
        UI::{
            HiDpi::GetDpiForWindow,
            Shell::SHQueryUserNotificationState,
            WindowsAndMessaging::{
                GetClientRect, GetCursorPos, GetForegroundWindow, GetWindowLongPtrW, GetWindowRect,
                GWL_EXSTYLE, HWND_TOPMOST, SWP_NOACTIVATE, SWP_SHOWWINDOW, SW_HIDE,
                SW_SHOWNOACTIVATE, SetWindowPos, ShowWindow, WS_EX_TOPMOST, WindowFromPoint,
            },
        },
    },
    core::BOOL,
};

use crate::host::{EXPANDED, app_hwnd};

const COLLAPSED_WIDTH: f32 = 320.0;
const COLLAPSED_HEIGHT: f32 = 40.0;
const EXPANDED_WIDTH: f32 = 620.0;
const EXPANDED_HEIGHT: f32 = 340.0;
const TOP_GAP: f32 = 8.0;
const TUCKED_VISIBLE_EDGE: f32 = 10.0;

static LAST_MONITOR: AtomicIsize = AtomicIsize::new(0);
static LAST_DPI: AtomicI32 = AtomicI32::new(0);
static LAST_TOP: AtomicI32 = AtomicI32::new(-1);
static LAST_HIT: AtomicI32 = AtomicI32::new(-1);
static LAST_FG: AtomicI32 = AtomicI32::new(-1);
static LAST_FOREGROUND: AtomicIsize = AtomicIsize::new(0);
static TUCKED: AtomicBool = AtomicBool::new(false);
static FULLSCREEN_HIDDEN: AtomicBool = AtomicBool::new(false);

fn size(open: bool) -> (f32, f32) {
    if open {
        (EXPANDED_WIDTH, EXPANDED_HEIGHT)
    } else {
        (COLLAPSED_WIDTH, COLLAPSED_HEIGHT)
    }
}

pub fn apply(open: bool) {
    let app = app_hwnd();
    if app.0.is_null() || FULLSCREEN_HIDDEN.load(Ordering::Relaxed) {
        return;
    }
    // SAFETY: the window exists; the monitor calls only read.
    let monitor = unsafe { MonitorFromWindow(app, MONITOR_DEFAULTTONEAREST) };
    center_on(monitor, open);
}

fn center_on(monitor: HMONITOR, open: bool) {
    let app = app_hwnd();
    let mut info = MONITORINFO {
        cbSize: std::mem::size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };
    // SAFETY: info is a valid MONITORINFO.
    unsafe {
        let _ = GetMonitorInfoW(monitor, &mut info);
    }
    let dpi = unsafe { GetDpiForWindow(app) };
    let scale = dpi as f32 / 96.0;
    let (width, height) = size(open);
    let w = (width * scale).round() as i32;
    let h = (height * scale).round() as i32;
    let work = info.rcWork;
    let x = work.left + ((work.right - work.left) - w) / 2;
    let y = if !open && TUCKED.load(Ordering::Relaxed) {
        work.top - ((COLLAPSED_HEIGHT - TUCKED_VISIBLE_EDGE) * scale).round() as i32
    } else {
        work.top + (TOP_GAP * scale).round() as i32
    };
    // SAFETY: positioning the live view window.
    let placed =
        unsafe { SetWindowPos(app, Some(HWND_TOPMOST), x, y, w, h, SWP_NOACTIVATE | SWP_SHOWWINDOW) };
    if let Err(error) = placed {
        log::warn!("SetWindowPos: {error}");
    }
    log::info!(
        "island open={open} tucked={} dpi={dpi} work=({},{})-({},{}) at=({x},{y}) {}x{}",
        TUCKED.load(Ordering::Relaxed),
        work.left,
        work.top,
        work.right,
        work.bottom,
        w,
        h
    );
    LAST_MONITOR.store(monitor.0 as isize, Ordering::Relaxed);
    LAST_DPI.store(dpi as i32, Ordering::Relaxed);
    LAST_TOP.store(1, Ordering::Relaxed);
}

/// Tucks the collapsed island above the top edge, or reveals it.
pub fn toggle_tuck() {
    if EXPANDED.load(Ordering::Relaxed) {
        crate::host::set_expanded(false);
    }
    let tucked = !TUCKED.load(Ordering::Relaxed);
    TUCKED.store(tucked, Ordering::Relaxed);
    log::info!("tuck: {tucked}");
    center_on(
        unsafe { MonitorFromWindow(app_hwnd(), MONITOR_DEFAULTTONEAREST) },
        false,
    );
}

fn fullscreen_active() -> bool {
    let app = app_hwnd();
    let foreground = unsafe { GetForegroundWindow() };
    if foreground == app {
        return false;
    }
    // SAFETY: a plain shell query.
    let state = unsafe { SHQueryUserNotificationState() };
    match state {
        Ok(state) => matches!(state.0, 2..=4),
        Err(_) => false,
    }
}

fn check_fullscreen() {
    let app = app_hwnd();
    if app.0.is_null() {
        return;
    }
    let active = fullscreen_active();
    let hidden = FULLSCREEN_HIDDEN.load(Ordering::Relaxed);
    if active && !hidden {
        // SAFETY: the live view window.
        unsafe {
            let _ = ShowWindow(app, SW_HIDE);
        }
        FULLSCREEN_HIDDEN.store(true, Ordering::Relaxed);
        log::info!("fullscreen: island hidden");
    } else if !active && hidden {
        FULLSCREEN_HIDDEN.store(false, Ordering::Relaxed);
        // SAFETY: the live view window.
        unsafe {
            let _ = ShowWindow(app, SW_SHOWNOACTIVATE);
        }
        center_on(
            unsafe { MonitorFromWindow(app, MONITOR_DEFAULTTONEAREST) },
            EXPANDED.load(Ordering::Relaxed),
        );
        log::info!("fullscreen: island shown");
    }
}

fn reveal_if_hovered() {
    if EXPANDED.load(Ordering::Relaxed) || !TUCKED.load(Ordering::Relaxed) {
        return;
    }
    let app = app_hwnd();
    let mut rect = RECT::default();
    let mut point = POINT::default();
    // SAFETY: reading the live window and the cursor.
    unsafe {
        let _ = GetWindowRect(app, &mut rect);
        let _ = GetCursorPos(&mut point);
    }
    if point.x >= rect.left
        && point.x < rect.right
        && point.y >= rect.bottom - 16
        && point.y < rect.bottom + 2
    {
        TUCKED.store(false, Ordering::Relaxed);
        log::info!("tuck: revealed by hover");
        center_on(
            unsafe { MonitorFromWindow(app, MONITOR_DEFAULTTONEAREST) },
            false,
        );
    }
}

pub fn watch() {
    let app = app_hwnd();
    if app.0.is_null() {
        return;
    }
    check_fullscreen();
    if FULLSCREEN_HIDDEN.load(Ordering::Relaxed) {
        return;
    }
    reveal_if_hovered();
    // SAFETY: the live view window; the calls only read state.
    let monitor = unsafe { MonitorFromWindow(app, MONITOR_DEFAULTTONEAREST) };
    let dpi = unsafe { GetDpiForWindow(app) };
    if monitor.0 as isize != LAST_MONITOR.load(Ordering::Relaxed)
        || dpi as i32 != LAST_DPI.load(Ordering::Relaxed)
    {
        log::info!("display change: re-centering");
        center_on(monitor, EXPANDED.load(Ordering::Relaxed));
    }
    let scale = dpi as f32 / 96.0;
    let (width, height) = size(EXPANDED.load(Ordering::Relaxed));
    let expected_w = (width * scale).round() as i32;
    let expected_h = (height * scale).round() as i32;
    let mut client = RECT::default();
    unsafe {
        let _ = GetClientRect(app, &mut client);
    }
    if client.right != expected_w || client.bottom != expected_h {
        log::info!(
            "client {}x{} != {}x{}: re-applying",
            client.right, client.bottom, expected_w, expected_h
        );
        center_on(monitor, EXPANDED.load(Ordering::Relaxed));
        return;
    }
    let mut rect = RECT::default();
    unsafe {
        let _ = GetWindowRect(app, &mut rect);
    }
    let center = POINT {
        x: (rect.left + rect.right) / 2,
        y: (rect.top + rect.bottom) / 2,
    };
    let hit = unsafe { WindowFromPoint(center) };
    let hit_app = if hit == app { 1 } else { 0 };
    if hit_app != LAST_HIT.load(Ordering::Relaxed) {
        log::info!("hit test at center: app={} hwnd={:?}", hit_app == 1, hit.0);
        LAST_HIT.store(hit_app, Ordering::Relaxed);
    }
    let foreground = unsafe { GetForegroundWindow() };
    if foreground != app && !foreground.0.is_null() {
        LAST_FOREGROUND.store(foreground.0 as isize, Ordering::Relaxed);
    }
    let fg = if foreground == app { 1 } else { 0 };
    if fg != LAST_FG.load(Ordering::Relaxed) {
        log::info!("foreground: app={}", fg == 1);
        LAST_FG.store(fg, Ordering::Relaxed);
    }
    let style = unsafe { GetWindowLongPtrW(app, GWL_EXSTYLE) } as u32;
    let topmost = style & WS_EX_TOPMOST.0 != 0;
    let top_int = i32::from(topmost);
    if top_int != LAST_TOP.load(Ordering::Relaxed) {
        log::warn!("topmost changed: {topmost}");
        LAST_TOP.store(top_int, Ordering::Relaxed);
    }
}

pub fn last_foreground() -> HWND {
    HWND(LAST_FOREGROUND.load(Ordering::Relaxed) as *mut _)
}

/// Moves the island to the next monitor, for the mixed-DPI probe.
pub fn next_monitor() {
    let mut monitors: Vec<HMONITOR> = Vec::new();
    // SAFETY: the callback only pushes into monitors.
    unsafe {
        let _ = EnumDisplayMonitors(
            None,
            None,
            Some(collect_monitor),
            LPARAM(&mut monitors as *mut _ as isize),
        );
    }
    if monitors.len() < 2 {
        log::info!("only one monitor");
        return;
    }
    let current = LAST_MONITOR.load(Ordering::Relaxed) as usize;
    let index = monitors
        .iter()
        .position(|m| m.0 as usize == current)
        .map_or(0, |position| (position + 1) % monitors.len());
    center_on(monitors[index], EXPANDED.load(Ordering::Relaxed));
}

unsafe extern "system" fn collect_monitor(
    monitor: HMONITOR,
    _dc: HDC,
    _rect: *mut RECT,
    data: LPARAM,
) -> BOOL {
    // SAFETY: data is the Vec<HMONITOR> from next_monitor.
    let monitors = unsafe { &mut *(data.0 as *mut Vec<HMONITOR>) };
    monitors.push(monitor);
    BOOL(1)
}
