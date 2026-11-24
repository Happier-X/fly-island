//! The translation shortcut, owned by one Win32 message thread.
//!
//! It works in the standalone app and inside the host: RegisterHotKey posts WM_HOTKEY to
//! this thread, which only sets a flag; the app polls it and opens its own NSH window.

use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Mutex, OnceLock,
    },
    thread,
    time::{Duration, Instant},
};

use windows::{
    Win32::{
        Foundation::{LPARAM, WPARAM},
        System::Threading::GetCurrentThreadId,
        UI::{
            Input::KeyboardAndMouse::{
                RegisterHotKey, UnregisterHotKey, HOT_KEY_MODIFIERS, MOD_ALT, MOD_CONTROL,
                MOD_SHIFT, MOD_WIN,
            },
            WindowsAndMessaging::{
                GetMessageW, PostThreadMessageW, MSG, WM_APP, WM_HOTKEY,
            },
        },
    },
};

const HOTKEY_ID: i32 = 0x4654;
const REREGISTER_MESSAGE: u32 = WM_APP + 0x61;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Binding {
    pub label: String,
    pub modifiers: u32,
    pub key_code: u32,
}

static THREAD_ID: OnceLock<u32> = OnceLock::new();
static ENABLED: AtomicBool = AtomicBool::new(true);
static REQUEST: AtomicBool = AtomicBool::new(false);
static ACTIVE: Mutex<Option<Binding>> = Mutex::new(None);
type Pending = (Option<Binding>, mpsc::Sender<Result<String, String>>);
static PENDING: Mutex<Option<Pending>> = Mutex::new(None);

/// Registers the startup shortcut and starts the message thread. A taken shortcut is left
/// unregistered; the settings page reports it when the user applies it again.
pub fn start(shortcut: &str, enabled: bool) {
    ENABLED.store(enabled, Ordering::SeqCst);
    let initial = parse(shortcut);
    let (ready_tx, ready_rx) = mpsc::channel();
    thread::spawn(move || {
        let id = unsafe { GetCurrentThreadId() };
        let _ = THREAD_ID.set(id);
        if enabled {
            if let Some(binding) = initial {
                let _ = register(&binding);
            }
        }
        let _ = ready_tx.send(());
        run_loop();
    });
    let _ = ready_rx.recv_timeout(Duration::from_secs(2));
}

/// True once after the shortcut was pressed. The app polls this and toggles its window.
pub fn take_request() -> bool {
    REQUEST.swap(false, Ordering::SeqCst)
}

pub fn set_shortcut(shortcut: &str, enabled: bool) -> Result<String, String> {
    ENABLED.store(enabled, Ordering::SeqCst);
    let binding = parse(shortcut).ok_or_else(|| "快捷键无效，请使用至少一个修饰键。".to_string())?;
    if !enabled {
        let _ = change_hotkey(None);
        return Ok(binding.label);
    }
    change_hotkey(Some(binding))
}

fn change_hotkey(binding: Option<Binding>) -> Result<String, String> {
    let started = Instant::now();
    let thread_id = loop {
        if let Some(id) = THREAD_ID.get() {
            break *id;
        }
        if started.elapsed() >= Duration::from_secs(2) {
            return Err("快捷键服务尚未准备好。".to_string());
        }
        thread::sleep(Duration::from_millis(25));
    };
    let (sender, receiver) = mpsc::channel();
    {
        let mut pending = PENDING.lock().map_err(|error| error.to_string())?;
        if pending.is_some() {
            return Err("快捷键正在更新。".to_string());
        }
        *pending = Some((binding, sender));
    }
    if let Err(error) = unsafe { PostThreadMessageW(thread_id, REREGISTER_MESSAGE, WPARAM(0), LPARAM(0)) } {
        if let Ok(mut pending) = PENDING.lock() {
            *pending = None;
        }
        return Err(format!("无法更新快捷键：{error}"));
    }
    receiver
        .recv_timeout(Duration::from_secs(2))
        .map_err(|error| format!("快捷键更新超时：{error}"))?
}

fn run_loop() {
    loop {
        let mut message = MSG::default();
        // SAFETY: this thread owns its message queue; message points at a writable MSG.
        if unsafe { GetMessageW(&mut message, None, 0, 0) }.0 <= 0 {
            break;
        }
        if message.message == WM_HOTKEY && message.wParam.0 as i32 == HOTKEY_ID {
            if ENABLED.load(Ordering::SeqCst) {
                REQUEST.store(true, Ordering::SeqCst);
            }
        } else if message.message == REREGISTER_MESSAGE {
            apply_pending();
        }
    }
}

fn apply_pending() {
    let Some((binding, sender)) = PENDING.lock().ok().and_then(|mut pending| pending.take()) else {
        return;
    };
    let previous = ACTIVE.lock().ok().and_then(|mut active| active.take());
    if previous.is_some() {
        let _ = unsafe { UnregisterHotKey(None, HOTKEY_ID) };
    }
    let Some(binding) = binding else {
        let _ = sender.send(Ok(String::new()));
        return;
    };
    let result = register(&binding);
    if result.is_ok() {
        if let Ok(mut active) = ACTIVE.lock() {
            *active = Some(binding.clone());
        }
    } else if let Some(previous) = previous {
        // Keep the old shortcut usable when the new one is taken.
        let _ = register(&previous);
    }
    let _ = sender.send(result.map(|()| binding.label));
}

fn register(binding: &Binding) -> Result<(), String> {
    // SAFETY: a null owner window is allowed; WM_HOTKEY is posted to this thread.
    unsafe { RegisterHotKey(None, HOTKEY_ID, HOT_KEY_MODIFIERS(binding.modifiers), binding.key_code) }
        .map_err(|error| format!("快捷键 {} 已被占用或无法注册：{error}", binding.label))
}

pub fn parse(shortcut: &str) -> Option<Binding> {
    let mut ctrl = false;
    let mut alt = false;
    let mut shift = false;
    let mut win = false;
    let mut key: Option<(String, u32)> = None;
    for part in shortcut.split('+').map(str::trim).filter(|part| !part.is_empty()) {
        match part.to_ascii_lowercase().as_str() {
            "ctrl" | "control" => ctrl = true,
            "alt" | "option" => alt = true,
            "shift" => shift = true,
            "win" | "windows" | "meta" | "cmd" | "super" => win = true,
            _ => {
                if key.is_some() {
                    return None;
                }
                key = key_code(part);
            }
        }
    }
    if !(ctrl || alt || shift || win) {
        return None;
    }
    let (key_label, key_code) = key?;
    let mut label = Vec::new();
    let mut modifiers = 0;
    if ctrl {
        label.push("Ctrl".to_string());
        modifiers |= MOD_CONTROL.0;
    }
    if alt {
        label.push("Alt".to_string());
        modifiers |= MOD_ALT.0;
    }
    if shift {
        label.push("Shift".to_string());
        modifiers |= MOD_SHIFT.0;
    }
    if win {
        label.push("Win".to_string());
        modifiers |= MOD_WIN.0;
    }
    label.push(key_label);
    Some(Binding { label: label.join("+"), modifiers, key_code })
}

fn key_code(key: &str) -> Option<(String, u32)> {
    let upper = key.trim().to_ascii_uppercase();
    if upper.len() == 1 {
        let byte = upper.as_bytes()[0];
        if byte.is_ascii_alphanumeric() {
            return Some((upper, u32::from(byte)));
        }
    }
    if let Some(number) = upper.strip_prefix('F').and_then(|value| value.parse::<u32>().ok()) {
        if (1..=24).contains(&number) {
            return Some((format!("F{number}"), 0x70 + number - 1));
        }
    }
    match upper.as_str() {
        "ESC" | "ESCAPE" => Some(("Esc".to_string(), 0x1B)),
        "TAB" => Some(("Tab".to_string(), 0x09)),
        "ENTER" | "RETURN" => Some(("Enter".to_string(), 0x0D)),
        "SPACE" => Some(("Space".to_string(), 0x20)),
        "BACKSPACE" => Some(("Backspace".to_string(), 0x08)),
        "DELETE" | "DEL" => Some(("Delete".to_string(), 0x2E)),
        "INSERT" | "INS" => Some(("Insert".to_string(), 0x2D)),
        "HOME" => Some(("Home".to_string(), 0x24)),
        "END" => Some(("End".to_string(), 0x23)),
        "PAGEUP" | "PAGE UP" => Some(("PageUp".to_string(), 0x21)),
        "PAGEDOWN" | "PAGE DOWN" => Some(("PageDown".to_string(), 0x22)),
        "ARROWUP" | "UP" => Some(("Up".to_string(), 0x26)),
        "ARROWDOWN" | "DOWN" => Some(("Down".to_string(), 0x28)),
        "ARROWLEFT" | "LEFT" => Some(("Left".to_string(), 0x25)),
        "ARROWRIGHT" | "RIGHT" => Some(("Right".to_string(), 0x27)),
        ";" => Some((upper, 0xBA)),
        "=" => Some((upper, 0xBB)),
        "," => Some((upper, 0xBC)),
        "-" => Some((upper, 0xBD)),
        "." => Some((upper, 0xBE)),
        "/" => Some((upper, 0xBF)),
        "\u{0060}" => Some((upper, 0xC0)),
        "[" => Some((upper, 0xDB)),
        "\\" => Some((upper, 0xDC)),
        "]" => Some((upper, 0xDD)),
        "'" => Some((upper, 0xDE)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_shortcut_parses() {
        let binding = parse("Ctrl+Shift+F").unwrap();
        assert_eq!(binding.label, "Ctrl+Shift+F");
        assert_eq!(binding.key_code, 0x46);
        assert_ne!(binding.modifiers, 0);
        assert!(parse("F").is_none());
        assert!(parse("Ctrl+ ").is_none());
    }
}