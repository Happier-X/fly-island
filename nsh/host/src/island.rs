//! Host-side bounds and monitor/DPI probes for the island window.

use std::sync::atomic::{AtomicI32, AtomicIsize, Ordering};

use windows::{
    Win32::{
        Foundation::{LPARAM, POINT, RECT},
        Graphics::Gdi::{
            EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR, MONITOR_DEFAULTTONEAREST,
            MONITORINFO, MonitorFromWindow,
        },
        UI::HiDpi::GetDpiForWindow,
        UI::WindowsAndMessaging::{
            GetClientRect, GetForegroundWindow, GetWindowLongPtrW, GetWindowRect, GWL_EXSTYLE,
            HWND_TOPMOST, SWP_NOACTIVATE, SWP_SHOWWINDOW, WindowFromPoint,
            SetWindowPos, WS_EX_TOPMOST,
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

static LAST_MONITOR: AtomicIsize = AtomicIsize::new(0);
static LAST_DPI: AtomicI32 = AtomicI32::new(0);
static LAST_TOP: AtomicI32 = AtomicI32::new(-1);
static LAST_HIT: AtomicI32 = AtomicI32::new(-1);
static LAST_FG: AtomicI32 = AtomicI32::new(-1);

fn size(open: bool) -> (f32, f32) {
    if open {
        (EXPANDED_WIDTH, EXPANDED_HEIGHT)
    } else {
        (COLLAPSED_WIDTH, COLLAPSED_HEIGHT)
    }
}

pub fn apply(open: bool) {
    let app = app_hwnd();
    if app.0.is_null() {
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
    let y = work.top + (TOP_GAP * scale).round() as i32;
    // SAFETY: positioning the live view window.
    let placed =
        unsafe { SetWindowPos(app, Some(HWND_TOPMOST), x, y, w, h, SWP_NOACTIVATE | SWP_SHOWWINDOW) };
    if let Err(error) = placed {
        log::warn!("SetWindowPos: {error}");
    }
    log::info!(
        "island open={open} dpi={dpi} work=({},{})-({},{}) at=({x},{y}) {}x{}",
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

pub fn watch() {
    let app = app_hwnd();
    if app.0.is_null() {
        return;
    }
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
    let fg = if unsafe { GetForegroundWindow() } == app { 1 } else { 0 };
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
    true.into()
}
