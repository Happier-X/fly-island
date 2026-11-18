//! Quick launcher: a standalone search panel over installed applications, files in the
//! folders the user picked, and the web. A global shortcut opens it centered on the
//! monitor under the pointer; it closes on Esc, on losing the focus, or on its shortcut.

mod apps;
mod files;
mod icon;

use serde::Serialize;
use std::{
    collections::HashSet,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Mutex, OnceLock,
    },
    thread,
    time::{Duration, Instant},
};
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize, WebviewWindow};
use windows::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, WPARAM},
    UI::WindowsAndMessaging::{WM_APP, WM_HOTKEY},
};

use crate::{
    clipboard_history::{parse_shortcut_binding, ShortcutBinding},
    win32::{
        self, known_folder,
        message_window::{self, WindowMessages},
    },
};

pub(crate) const WINDOW_LABEL: &str = "launcher";

/// Mirrors LAUNCHER_OPENED_EVENT in src/features/quick-launcher/api.ts.
const OPENED_EVENT: &str = "quick-launcher-opened";
/// Mirrors LAUNCHER_INDEX_READY_EVENT in src/features/quick-launcher/api.ts.
const INDEX_READY_EVENT: &str = "quick-launcher-index-ready";

const DEFAULT_SHORTCUT: &str = "Alt+Space";
const HOTKEY_ID: i32 = 0x464C;
const CHANGE_HOTKEY_MESSAGE: u32 = WM_APP + 0x53;

/// The panel's size in logical pixels; it is scaled to the monitor it opens on.
const PANEL_WIDTH: f64 = 680.0;
const PANEL_HEIGHT: f64 = 460.0;
/// Gap kept from the work area's edges on a screen too small for the full panel.
const PANEL_MARGIN: f64 = 24.0;

/// Installed applications change rarely; a stale index is refreshed in the background.
const APP_INDEX_TTL: Duration = Duration::from_secs(300);

static APP: OnceLock<AppHandle> = OnceLock::new();
static HOTKEY_WINDOW: Mutex<Option<isize>> = Mutex::new(None);
static ACTIVE_BINDING: Mutex<Option<ShortcutBinding>> = Mutex::new(None);
/// Off when the launcher is switched off: its shortcut is released so its keys reach
/// other programs again.
static ENABLED: AtomicBool = AtomicBool::new(true);
static APP_BUILDING: AtomicBool = AtomicBool::new(false);

type PendingBinding = (
    Option<ShortcutBinding>,
    mpsc::Sender<Result<String, String>>,
);
static PENDING_BINDING: Mutex<Option<PendingBinding>> = Mutex::new(None);

struct AppIndex {
    entries: Vec<LauncherEntry>,
    built_at: Instant,
}

static APP_INDEX: Mutex<Option<AppIndex>> = Mutex::new(None);

/// One result or index entry. `target` is what opens: a shortcut, file or URL.
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LauncherEntry {
    id: String,
    name: String,
    subtitle: String,
    target: String,
    kind: LauncherEntryKind,
}

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum LauncherEntryKind {
    App,
    File,
}

impl LauncherEntry {
    fn new(kind: LauncherEntryKind, name: String, subtitle: String, target: String) -> Self {
        let prefix = match kind {
            LauncherEntryKind::App => "app",
            LauncherEntryKind::File => "file",
        };
        Self {
            id: format!("{prefix}:{}", target_key(&target)),
            name,
            subtitle,
            target,
            kind,
        }
    }
}

/// Targets compared without case so the same program listed twice appears once.
fn target_key(target: &str) -> String {
    target.trim().to_lowercase()
}

/// The index handed to the panel. `apps_ready` is false while the first enumeration is
/// still running; the panel then listens for INDEX_READY_EVENT.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LauncherIndex {
    apps: Vec<LauncherEntry>,
    files: Vec<LauncherEntry>,
    apps_ready: bool,
}

pub fn init(app: &AppHandle) {
    if APP.set(app.clone()).is_ok() {
        thread::spawn(|| {
            if let Err(error) = run_hotkey_loop() {
                eprintln!("failed to initialize the quick launcher shortcut: {error}");
            }
        });
        // Installed applications are read once in the background, so the first panel
        // opens with them already listed.
        request_app_refresh(app.clone());
    }
}

fn request_app_refresh(app: AppHandle) {
    if APP_BUILDING.swap(true, Ordering::SeqCst) {
        return;
    }
    thread::spawn(move || {
        let entries = apps::enumerate();
        if let Ok(mut index) = APP_INDEX.lock() {
            *index = Some(AppIndex {
                entries,
                built_at: Instant::now(),
            });
        }
        APP_BUILDING.store(false, Ordering::SeqCst);
        let _ = app.emit_to(WINDOW_LABEL, INDEX_READY_EVENT, ());
    });
}

fn app_snapshot() -> (Vec<LauncherEntry>, bool) {
    let stale = APP_INDEX.lock().ok().and_then(|index| {
        index.as_ref().map(|index| {
            (
                index.entries.clone(),
                index.built_at.elapsed() >= APP_INDEX_TTL,
            )
        })
    });
    match stale {
        Some((entries, is_stale)) => {
            if is_stale {
                if let Some(app) = APP.get() {
                    request_app_refresh(app.clone());
                }
            }
            (entries, true)
        }
        None => {
            if let Some(app) = APP.get() {
                request_app_refresh(app.clone());
            }
            (Vec::new(), false)
        }
    }
}

#[tauri::command]
pub async fn get_launcher_index(
    include_apps: bool,
    include_files: bool,
    folders: Vec<String>,
) -> Result<LauncherIndex, String> {
    let (apps, apps_ready) = if include_apps {
        app_snapshot()
    } else {
        (Vec::new(), true)
    };
    let files = if include_files {
        tauri::async_runtime::spawn_blocking(move || files::indexed(&folders))
            .await
            .map_err(|error| format!("无法读取文件索引：{error}"))?
    } else {
        Vec::new()
    };
    Ok(LauncherIndex {
        apps,
        files,
        apps_ready,
    })
}

#[tauri::command]
pub async fn get_launcher_icon(path: String, kind: String) -> Result<Option<String>, String> {
    tauri::async_runtime::spawn_blocking(move || icon::data_url(&path, &kind))
        .await
        .map_err(|error| format!("无法读取图标：{error}"))
}

#[tauri::command]
pub async fn open_launcher_item(target: String) -> Result<(), String> {
    // Start Menu Internet Shortcuts point at a URL rather than a file.
    let lower = target.to_ascii_lowercase();
    if lower.starts_with("http://") || lower.starts_with("https://") {
        return tauri_plugin_opener::open_url(&target, None::<&str>)
            .map_err(|error| format!("无法打开「{target}」：{error}"));
    }
    if !std::path::Path::new(&target).is_absolute() {
        return Err("Expected an absolute path.".to_string());
    }
    tauri_plugin_opener::open_path(&target, None::<&str>)
        .map_err(|error| format!("无法打开「{target}」：{error}"))
}

#[tauri::command]
pub async fn open_launcher_url(url: String) -> Result<(), String> {
    if !url.starts_with("https://") {
        return Err("Expected an HTTPS search URL.".to_string());
    }
    tauri_plugin_opener::open_url(&url, None::<&str>)
        .map_err(|error| format!("无法打开网页：{error}"))
}

/// Desktop, Documents and Downloads, the folders searched until the user picks others.
#[tauri::command]
pub fn get_launcher_default_folders() -> Vec<String> {
    let mut seen = HashSet::new();
    [
        known_folder::desktop(),
        known_folder::documents(),
        known_folder::downloads(),
    ]
    .into_iter()
    .flatten()
    .filter(|folder| folder.is_dir())
    .map(|folder| folder.to_string_lossy().into_owned())
    .filter(|folder| seen.insert(folder.to_lowercase()))
    .collect()
}

fn launcher_window(app: &AppHandle) -> Result<WebviewWindow, String> {
    app.get_webview_window(WINDOW_LABEL)
        .ok_or_else(|| "Quick launcher window was not found.".to_string())
}

#[tauri::command]
pub async fn hide_quick_launcher(app: AppHandle) -> Result<(), String> {
    launcher_window(&app)?
        .hide()
        .map_err(|error| error.to_string())
}

/// Opens the panel on the monitor under the pointer, or hides it when it is already up.
fn toggle_quick_launcher(app: AppHandle) -> Result<(), String> {
    let window = launcher_window(&app)?;
    if window.is_visible().map_err(|error| error.to_string())? {
        return window.hide().map_err(|error| error.to_string());
    }
    show(&app, &window)
}

fn show(app: &AppHandle, window: &WebviewWindow) -> Result<(), String> {
    let under_pointer = win32::window::cursor_position().and_then(|point| {
        window
            .monitor_from_point(f64::from(point.x), f64::from(point.y))
            .ok()
            .flatten()
    });
    let monitor = under_pointer
        .or(window
            .primary_monitor()
            .map_err(|error| error.to_string())?)
        .ok_or_else(|| "No monitor is available for the quick launcher.".to_string())?;
    let area = monitor.work_area();
    let work_area = (
        area.position.x,
        area.position.y,
        area.size.width,
        area.size.height,
    );
    // Position first so the destination monitor's DPI is active when applying the size.
    let placement = centered_panel(work_area, monitor.scale_factor());
    window
        .set_position(PhysicalPosition::new(placement.x, placement.y))
        .map_err(|error| error.to_string())?;
    window
        .set_size(PhysicalSize::new(placement.width, placement.height))
        .map_err(|error| error.to_string())?;
    // Appears in place, without the system's zoom-in, as the translation window does.
    if let Ok(hwnd) = window.hwnd() {
        let _ = win32::window::disable_show_animation(hwnd);
    }
    window.show().map_err(|error| error.to_string())?;
    let _ = window.set_focus();
    let _ = app.emit_to(WINDOW_LABEL, OPENED_EVENT, ());
    Ok(())
}

/// A monitor's work area in physical pixels: left, top, width, height.
type WorkArea = (i32, i32, u32, u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Placement {
    x: i32,
    y: i32,
    width: u32,
    height: u32,
}

/// Centered and scaled in the destination monitor's work area.
fn centered_panel(area: WorkArea, scale: f64) -> Placement {
    let scale = scale.clamp(0.5, 4.0);
    let margin = (PANEL_MARGIN * scale).round() as u32;
    let width = ((PANEL_WIDTH * scale).round() as u32)
        .min(area.2.saturating_sub(margin.saturating_mul(2)))
        .max(1);
    let height = ((PANEL_HEIGHT * scale).round() as u32)
        .min(area.3.saturating_sub(margin.saturating_mul(2)))
        .max(1);
    let x = area.0 + (i32::try_from(area.2.saturating_sub(width) / 2).unwrap_or(0));
    let y = area.1 + (i32::try_from(area.3.saturating_sub(height) / 2).unwrap_or(0));
    Placement {
        x,
        y,
        width,
        height,
    }
}

#[tauri::command]
pub async fn set_launcher_shortcut(shortcut: String) -> Result<String, String> {
    let binding = parse_shortcut_binding(&shortcut)
        .ok_or_else(|| "快捷键无效，请使用至少一个修饰键。".to_string())?;
    if !ENABLED.load(Ordering::SeqCst) {
        // Remembered by the frontend and registered when the launcher is switched on.
        return Ok(binding.label);
    }
    change_hotkey(Some(binding))
}

/// Switching off releases the shortcut and hides the panel; switching on registers
/// nothing by itself, the frontend then calls set_launcher_shortcut.
#[tauri::command]
pub async fn set_launcher_enabled(app: AppHandle, enabled: bool) -> Result<(), String> {
    ENABLED.store(enabled, Ordering::SeqCst);
    if !enabled {
        change_hotkey(None)?;
        if let Ok(window) = launcher_window(&app) {
            let _ = window.hide();
        }
    }
    Ok(())
}

fn change_hotkey(binding: Option<ShortcutBinding>) -> Result<String, String> {
    post_binding(&PENDING_BINDING, CHANGE_HOTKEY_MESSAGE, binding)
}

fn post_binding(
    slot: &Mutex<Option<PendingBinding>>,
    message: u32,
    binding: Option<ShortcutBinding>,
) -> Result<String, String> {
    let started = Instant::now();
    let hwnd = loop {
        if let Some(hwnd) = *HOTKEY_WINDOW.lock().map_err(|error| error.to_string())? {
            break hwnd;
        }
        if started.elapsed() >= Duration::from_secs(2) {
            return Err("快捷键服务尚未准备好。".to_string());
        }
        thread::sleep(Duration::from_millis(25));
    };
    let (sender, receiver) = mpsc::channel();
    {
        let mut pending = slot.lock().map_err(|error| error.to_string())?;
        if pending.is_some() {
            return Err("快捷键正在更新。".to_string());
        }
        *pending = Some((binding, sender));
    }
    if let Err(error) = message_window::post_message(HWND(hwnd as *mut _), message) {
        *slot.lock().map_err(|error| error.to_string())? = None;
        return Err(format!("无法更新快捷键：{error}"));
    }
    receiver
        .recv_timeout(Duration::from_secs(2))
        .map_err(|error| format!("快捷键更新超时：{error}"))?
}

fn run_hotkey_loop() -> Result<(), String> {
    let class_name = "FocuSDLauncherHotkey";
    let instance = message_window::module_instance().map_err(|error| error.to_string())?;
    if message_window::register_class::<LauncherHotkeyWindow>(instance, class_name).is_err() {
        return Err("无法注册快捷启动快捷键窗口。".to_string());
    }
    let hwnd = message_window::create_message_only_window(
        instance,
        class_name,
        "FocuSD Quick Launcher Hotkey",
    )
    .map_err(|error| error.to_string())?;
    if let Some(binding) =
        parse_shortcut_binding(DEFAULT_SHORTCUT).filter(|_| ENABLED.load(Ordering::SeqCst))
    {
        if register_binding(hwnd, HOTKEY_ID, &binding).is_ok() {
            *ACTIVE_BINDING.lock().map_err(|error| error.to_string())? = Some(binding);
        }
    }
    *HOTKEY_WINDOW.lock().map_err(|error| error.to_string())? = Some(hwnd.0 as isize);
    message_window::run_message_loop();
    let _ = win32::hotkey::unregister(hwnd, HOTKEY_ID);
    Ok(())
}

fn register_binding(hwnd: HWND, id: i32, binding: &ShortcutBinding) -> Result<(), String> {
    win32::hotkey::register(hwnd, id, binding.modifiers, binding.key_code)
        .map_err(|error| format!("快捷键 {} 已被占用或无法注册：{error}", binding.label))
}

fn apply_binding_change(
    hwnd: HWND,
    id: i32,
    pending_slot: &Mutex<Option<PendingBinding>>,
    active_slot: &Mutex<Option<ShortcutBinding>>,
) {
    let pending = pending_slot.lock().ok().and_then(|mut value| value.take());
    let Some((binding, sender)) = pending else {
        return;
    };
    let previous = active_slot.lock().ok().and_then(|mut value| value.take());
    if previous.is_some() {
        let _ = win32::hotkey::unregister(hwnd, id);
    }
    let Some(binding) = binding else {
        let _ = sender.send(Ok(String::new()));
        return;
    };
    let result = register_binding(hwnd, id, &binding).map(|()| binding.label.clone());
    if let Ok(mut active) = active_slot.lock() {
        *active = if result.is_ok() {
            Some(binding)
        } else {
            previous.and_then(|old| register_binding(hwnd, id, &old).ok().map(|()| old))
        };
    }
    let _ = sender.send(result);
}

struct LauncherHotkeyWindow;

impl WindowMessages for LauncherHotkeyWindow {
    fn handle(hwnd: HWND, message: u32, wparam: WPARAM, _: LPARAM) -> Option<LRESULT> {
        if message == WM_HOTKEY && wparam.0 == HOTKEY_ID as usize {
            if let Some(app) = APP.get().filter(|_| ENABLED.load(Ordering::SeqCst)) {
                let _ = toggle_quick_launcher(app.clone());
            }
            return Some(LRESULT(0));
        }
        if message == CHANGE_HOTKEY_MESSAGE {
            apply_binding_change(hwnd, HOTKEY_ID, &PENDING_BINDING, &ACTIVE_BINDING);
            return Some(LRESULT(0));
        }
        None
    }
}

/// This module's Tauri commands. `lib.rs` chains every module's list into one
/// `generate_handler!`, so adding a command only touches this file.
macro_rules! tauri_commands {
    ($next:ident [$($done:tt)*] $($rest:tt)*) => {
        $next!([$($done)*
            crate::quick_launcher::get_launcher_index,
            crate::quick_launcher::get_launcher_icon,
            crate::quick_launcher::get_launcher_default_folders,
            crate::quick_launcher::open_launcher_item,
            crate::quick_launcher::open_launcher_url,
            crate::quick_launcher::hide_quick_launcher,
            crate::quick_launcher::set_launcher_shortcut,
            crate::quick_launcher::set_launcher_enabled,
        ] $($rest)*)
    };
}
pub(crate) use tauri_commands;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_panel_is_centered_in_each_work_area() {
        let screens = [((0, 0, 1920, 1040), 1.0), ((1920, 0, 2560, 1400), 1.0)];
        let left = centered_panel(screens[0].0, screens[0].1);
        assert_eq!((left.width, left.height), (680, 460));
        assert_eq!(left.x, 620);
        assert_eq!(left.y, 290);

        let right = centered_panel(screens[1].0, screens[1].1);
        assert_eq!(right.x, 1920 + (2560 - 680) / 2);
        assert_eq!(right.y, (1400 - 460) / 2);
    }

    #[test]
    fn a_negative_origin_and_mixed_dpi_are_supported() {
        let screens = [((-1280, -200, 1280, 720), 1.5)];
        let placement = centered_panel(screens[0].0, screens[0].1);
        // 680 x 1.5 = 1020 wide; the height shrinks to the 720-high work area less margins.
        assert_eq!((placement.width, placement.height), (1020, 648));
        assert_eq!(placement.x, -1280 + (1280 - 1020) / 2);
        assert_eq!(placement.y, -200 + (720 - 648) / 2);
    }

    #[test]
    fn a_small_screen_keeps_a_margin_around_the_panel() {
        let screens = [((0, 0, 640, 400), 1.0)];
        let placement = centered_panel(screens[0].0, screens[0].1);
        assert_eq!((placement.width, placement.height), (592, 352));
        assert_eq!((placement.x, placement.y), (24, 24));
    }

    #[test]
    #[ignore = "reads the real Start Menu and the user's folders; run it to report local timings"]
    fn reports_local_index_timings() {
        let started = Instant::now();
        let apps = apps::enumerate();
        println!("applications: {} in {:?}", apps.len(), started.elapsed());
        assert!(
            !apps.is_empty(),
            "the Start Menu lists at least one application"
        );

        let folders: Vec<String> = [
            known_folder::desktop(),
            known_folder::documents(),
            known_folder::downloads(),
        ]
        .into_iter()
        .flatten()
        .filter(|folder| folder.is_dir())
        .map(|folder| folder.to_string_lossy().into_owned())
        .collect();
        let started = Instant::now();
        let files = files::indexed(&folders);
        println!("files: {} in {:?}", files.len(), started.elapsed());
        let started = Instant::now();
        let cached = files::indexed(&folders);
        println!(
            "files (cached): {} in {:?}",
            cached.len(),
            started.elapsed()
        );
    }

    #[test]
    fn the_default_shortcut_parses_to_its_own_label() {
        let binding = parse_shortcut_binding(DEFAULT_SHORTCUT).expect("the default is valid");
        assert_eq!(binding.label, DEFAULT_SHORTCUT);
    }
}
