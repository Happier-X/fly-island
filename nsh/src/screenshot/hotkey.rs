//! The screenshot shortcut, owned by one Win32 message thread.
//!
//! RegisterHotKey posts WM_HOTKEY to this thread. The handler asks AirSnip to
//! capture and returns immediately, so a slow grab never blocks the key.

use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Mutex, OnceLock,
    },
    thread,
    time::{Duration, Instant},
};

use windows::Win32::{
    Foundation::{LPARAM, WPARAM},
    System::Threading::GetCurrentThreadId,
    UI::{
        Input::KeyboardAndMouse::{RegisterHotKey, UnregisterHotKey, HOT_KEY_MODIFIERS},
        WindowsAndMessaging::{GetMessageW, PostThreadMessageW, MSG, WM_APP, WM_HOTKEY},
    },
};

use crate::translation::hotkey::{parse, Binding};

const HOTKEY_ID: i32 = 0x4653;
const REREGISTER_MESSAGE: u32 = WM_APP + 0x81;

static THREAD_ID: OnceLock<u32> = OnceLock::new();
static ENABLED: AtomicBool = AtomicBool::new(true);
static ACTIVE: Mutex<Option<Binding>> = Mutex::new(None);
type Pending = (Option<Binding>, mpsc::Sender<Result<String, String>>);
static PENDING: Mutex<Option<Pending>> = Mutex::new(None);

pub fn start(shortcut: &str, enabled: bool) -> Result<(), String> {
    ENABLED.store(enabled, Ordering::SeqCst);
    let initial = parse(shortcut);
    let (ready_tx, ready_rx) = mpsc::channel();
    thread::spawn(move || {
        // SAFETY: this thread owns its message queue and reads its own id.
        let id = unsafe { GetCurrentThreadId() };
        let _ = THREAD_ID.set(id);
        let mut failure = None;
        if enabled && let Some(binding) = initial {
            match register(&binding) {
                Ok(()) => {
                    if let Ok(mut active) = ACTIVE.lock() {
                        *active = Some(binding);
                    }
                }
                Err(error) => failure = Some(error),
            }
        }
        let _ = ready_tx.send(failure.clone());
        if failure.is_none() {
            run_loop();
        }
    });
    match ready_rx.recv_timeout(Duration::from_secs(2)) {
        Ok(Some(error)) => Err(error),
        Ok(None) => Ok(()),
        Err(error) => Err(format!("截图快捷键服务启动超时：{error}")),
    }
}

pub fn set_shortcut(shortcut: &str, enabled: bool) -> Result<String, String> {
    ENABLED.store(enabled, Ordering::SeqCst);
    let binding = parse(shortcut)
        .ok_or_else(|| "截图快捷键无效，请使用至少一个修饰键。".to_string())?;
    if !enabled {
        change_hotkey(None)?;
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
            return Err("截图快捷键服务尚未准备好。".to_string());
        }
        thread::sleep(Duration::from_millis(25));
    };
    let (sender, receiver) = mpsc::channel();
    {
        let mut pending = PENDING.lock().map_err(|error| error.to_string())?;
        if pending.is_some() {
            return Err("截图快捷键正在更新。".to_string());
        }
        *pending = Some((binding, sender));
    }
    // SAFETY: the hotkey thread owns its queue; this only posts to it.
    if let Err(error) =
        unsafe { PostThreadMessageW(thread_id, REREGISTER_MESSAGE, WPARAM(0), LPARAM(0)) }
    {
        if let Ok(mut pending) = PENDING.lock() {
            *pending = None;
        }
        return Err(format!("无法更新截图快捷键：{error}"));
    }
    receiver
        .recv_timeout(Duration::from_secs(2))
        .map_err(|error| format!("截图快捷键更新超时：{error}"))?
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
                log::info!("screenshot hotkey fired");
                super::capture();
            }
        } else if message.message == REREGISTER_MESSAGE {
            apply_pending();
        }
    }
}

fn apply_pending() {
    let Some((binding, sender)) = PENDING.lock().ok().and_then(|mut pending| pending.take())
    else {
        return;
    };
    let previous = ACTIVE.lock().ok().and_then(|mut active| active.take());
    if previous.is_some() {
        // SAFETY: the hotkey thread releases its own registration.
        let _ = unsafe { UnregisterHotKey(None, HOTKEY_ID) };
    }
    let Some(binding) = binding else {
        let _ = sender.send(Ok(String::new()));
        return;
    };
    match register(&binding) {
        Ok(()) => {
            if let Ok(mut active) = ACTIVE.lock() {
                *active = Some(binding.clone());
            }
            let _ = sender.send(Ok(binding.label));
        }
        Err(error) => {
            if let Some(previous) = previous {
                let _ = register(&previous);
                if let Ok(mut active) = ACTIVE.lock() {
                    *active = Some(previous);
                }
            }
            let _ = sender.send(Err(error));
        }
    }
}

fn register(binding: &Binding) -> Result<(), String> {
    // SAFETY: a null owner window is allowed; WM_HOTKEY is posted to this thread.
    unsafe {
        RegisterHotKey(
            None,
            HOTKEY_ID,
            HOT_KEY_MODIFIERS(binding.modifiers),
            binding.key_code,
        )
    }
    .map_err(|error| {
        let message = format!("截图快捷键 {} 已被占用或无法注册：{error}。请在设置里换一个组合。", binding.label);
        log::warn!("{message}");
        message
    })
}
