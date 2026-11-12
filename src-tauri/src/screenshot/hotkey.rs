//! 全局截图热键：独立线程上的 message-only 窗口。
//!
//! 结构照 `translation/mod.rs` 的热键循环：`RegisterHotKey` 绑定到某个 HWND，
//! 消息只进该窗口所在线程的循环，所以注册与改绑都必须在那个线程上做，
//! 其他线程要改绑时用 `post_message` 投递过去。

use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc, Mutex,
};
use std::{thread, time::{Duration, Instant}};
use windows::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, WPARAM},
    UI::WindowsAndMessaging::{WM_APP, WM_HOTKEY},
};

use crate::clipboard_history::{parse_shortcut_binding, ShortcutBinding};
use crate::win32::{
    self,
    message_window::{self, WindowMessages},
};

/// 截图热键 id（已占用：0x4643 剪贴板、0x4654 翻译、0x5300+ Agent 审批）。
const HOTKEY_ID: i32 = 0x5343;
/// 改绑消息（已占用：WM_APP + 0x51、+0x52、+0x63）。
const CHANGE_HOTKEY_MESSAGE: u32 = WM_APP + 0x64;

static HOTKEY_WINDOW: Mutex<Option<isize>> = Mutex::new(None);
static ACTIVE_BINDING: Mutex<Option<ShortcutBinding>> = Mutex::new(None);
/// 关掉截图后热键要被注销，否则它会继续吞掉其他程序的按键。
static ENABLED: AtomicBool = AtomicBool::new(true);

/// 一个待应用的绑定（None 表示注销）及回传结果的通道。
type PendingBinding = (Option<ShortcutBinding>, mpsc::Sender<Result<String, String>>);

static PENDING_BINDING: Mutex<Option<PendingBinding>> = Mutex::new(None);

struct ScreenshotHotkeyWindow;

/// 启动热键线程。`enabled` 为假时只建窗口不注册，等设置页打开开关。
pub(super) fn start(enabled: bool, shortcut: &str) {
    ENABLED.store(enabled, Ordering::SeqCst);
    let initial = parse_shortcut_binding(shortcut);
    thread::spawn(move || {
        if let Err(error) = run_hotkey_loop(initial) {
            eprintln!("failed to initialize screenshot shortcut: {error}");
        }
    });
}

/// 改绑热键；`enabled` 为假时只记住绑定不注册。
pub(super) fn rebind(enabled: bool, shortcut: &str) -> Result<String, String> {
    ENABLED.store(enabled, Ordering::SeqCst);
    let binding = parse_shortcut_binding(shortcut)
        .ok_or_else(|| "快捷键无效，请使用至少一个修饰键。".to_string())?;
    if !enabled {
        return Ok(binding.label);
    }
    change_hotkey(Some(binding))
}

/// 关掉截图时注销热键。
pub(super) fn disable() -> Result<(), String> {
    ENABLED.store(false, Ordering::SeqCst);
    change_hotkey(None).map(|_| ())
}

fn run_hotkey_loop(initial: Option<ShortcutBinding>) -> Result<(), String> {
    let class_name = "FocuSDScreenshotHotkey";
    let instance = message_window::module_instance().map_err(|error| error.to_string())?;
    if message_window::register_class::<ScreenshotHotkeyWindow>(instance, class_name).is_err() {
        return Err("无法注册截图快捷键窗口。".to_string());
    }
    let hwnd = message_window::create_message_only_window(
        instance,
        class_name,
        "FocuSD Screenshot Hotkey",
    )
    .map_err(|error| error.to_string())?;

    if ENABLED.load(Ordering::SeqCst) {
        if let Some(binding) = initial {
            if register_binding(hwnd, HOTKEY_ID, &binding).is_ok() {
                if let Ok(mut active) = ACTIVE_BINDING.lock() {
                    *active = Some(binding);
                }
            }
        }
    }

    *HOTKEY_WINDOW
        .lock()
        .map_err(|error| error.to_string())? = Some(hwnd.0 as isize);
    message_window::run_message_loop();
    let _ = win32::hotkey::unregister(hwnd, HOTKEY_ID);
    Ok(())
}

fn register_binding(hwnd: HWND, id: i32, binding: &ShortcutBinding) -> Result<(), String> {
    win32::hotkey::register(hwnd, id, binding.modifiers, binding.key_code)
        .map_err(|error| format!("截图快捷键 {} 已被占用或无法注册：{error}", binding.label))
}

/// 把绑定交给持有注册的线程，并等待结果。
fn change_hotkey(binding: Option<ShortcutBinding>) -> Result<String, String> {
    let started = Instant::now();
    let hwnd = loop {
        if let Some(hwnd) = *HOTKEY_WINDOW.lock().map_err(|error| error.to_string())? {
            break hwnd;
        }
        if started.elapsed() >= Duration::from_secs(2) {
            return Err("截图快捷键服务尚未准备好。".to_string());
        }
        thread::sleep(Duration::from_millis(25));
    };

    let (sender, receiver) = mpsc::channel();
    {
        let mut pending = PENDING_BINDING.lock().map_err(|error| error.to_string())?;
        if pending.is_some() {
            return Err("截图快捷键正在更新。".to_string());
        }
        *pending = Some((binding, sender));
    }
    if let Err(error) = message_window::post_message(HWND(hwnd as *mut _), CHANGE_HOTKEY_MESSAGE) {
        if let Ok(mut pending) = PENDING_BINDING.lock() {
            *pending = None;
        }
        return Err(format!("无法更新截图快捷键：{error}"));
    }
    receiver
        .recv_timeout(Duration::from_secs(2))
        .map_err(|error| format!("截图快捷键更新超时：{error}"))?
}

fn apply_binding_change(hwnd: HWND) {
    let pending = PENDING_BINDING.lock().ok().and_then(|mut value| value.take());
    let Some((binding, sender)) = pending else {
        return;
    };
    let previous = ACTIVE_BINDING.lock().ok().and_then(|mut value| value.take());
    if previous.is_some() {
        let _ = win32::hotkey::unregister(hwnd, HOTKEY_ID);
    }
    let Some(binding) = binding else {
        let _ = sender.send(Ok(String::new()));
        return;
    };
    let result = register_binding(hwnd, HOTKEY_ID, &binding);
    if let Ok(mut active) = ACTIVE_BINDING.lock() {
        *active = if result.is_ok() {
            Some(binding.clone())
        } else {
            previous.and_then(|old| register_binding(hwnd, HOTKEY_ID, &old).ok().map(|()| old))
        };
    }
    let _ = sender.send(result.map(|()| binding.label));
}

impl WindowMessages for ScreenshotHotkeyWindow {
    fn handle(_hwnd: HWND, message: u32, wparam: WPARAM, _: LPARAM) -> Option<LRESULT> {
        if message == WM_HOTKEY && wparam.0 == HOTKEY_ID as usize {
            if ENABLED.load(Ordering::SeqCst) {
                super::capture();
            }
            return Some(LRESULT(0));
        }
        if message == CHANGE_HOTKEY_MESSAGE {
            apply_binding_change(_hwnd);
            return Some(LRESULT(0));
        }
        None
    }
}
