//! The launcher's global shortcut, on its own Win32 message thread: a key
//! combination, or two clean Ctrl taps.

use std::{
    cell::RefCell,
    sync::{
        atomic::{AtomicBool, AtomicU8, Ordering},
        mpsc, Mutex, OnceLock,
    },
    thread,
    time::{Duration, Instant},
};

use windows::Win32::{
    Foundation::{LPARAM, LRESULT, WPARAM},
    System::Threading::GetCurrentThreadId,
    UI::{
        Input::KeyboardAndMouse::{
            GetAsyncKeyState, RegisterHotKey, UnregisterHotKey, HOT_KEY_MODIFIERS, VK_DOWN,
            VK_ESCAPE, VK_LCONTROL, VK_LWIN, VK_MENU, VK_RCONTROL, VK_RETURN, VK_RWIN,
            VK_SHIFT, VK_UP,
        },
        WindowsAndMessaging::{
            CallNextHookEx, GetMessageW, PostThreadMessageW, SetWindowsHookExW,
            UnhookWindowsHookEx, HC_ACTION, HHOOK, KBDLLHOOKSTRUCT, LLKHF_ALTDOWN, MSG,
            WH_KEYBOARD_LL, WM_APP, WM_HOTKEY, WM_KEYDOWN, WM_KEYUP, WM_SYSKEYDOWN, WM_SYSKEYUP,
        },
    },
};

use crate::translation::hotkey::{parse, Binding};

const HOTKEY_ID: i32 = 0x464C;
const REREGISTER_MESSAGE: u32 = WM_APP + 0x71;
const DOUBLE_CTRL: &str = "DoubleCtrl";

static THREAD_ID: OnceLock<u32> = OnceLock::new();
static ENABLED: AtomicBool = AtomicBool::new(true);
static REQUEST: AtomicBool = AtomicBool::new(false);
static NAV_ENABLED: AtomicBool = AtomicBool::new(false);
static NAV_ACTION: AtomicU8 = AtomicU8::new(0);
static CTRL_TAPS_ENABLED: AtomicBool = AtomicBool::new(false);
static HOOK_READY: AtomicBool = AtomicBool::new(false);
static ACTIVE: Mutex<Option<Trigger>> = Mutex::new(None);
type Pending = (Option<Trigger>, mpsc::Sender<Result<String, String>>);
static PENDING: Mutex<Option<Pending>> = Mutex::new(None);

/// A key the open panel takes before the window does, so the native text
/// field cannot swallow the arrow keys.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NavAction {
    Up,
    Down,
    Enter,
    Escape,
}

pub fn set_nav(enabled: bool) {
    NAV_ENABLED.store(enabled, Ordering::SeqCst);
    if !enabled {
        NAV_ACTION.store(0, Ordering::SeqCst);
    }
}

pub fn take_nav() -> Option<NavAction> {
    match NAV_ACTION.swap(0, Ordering::SeqCst) {
        1 => Some(NavAction::Up),
        2 => Some(NavAction::Down),
        3 => Some(NavAction::Enter),
        4 => Some(NavAction::Escape),
        _ => None,
    }
}

/// What opens the panel: a registered key combination, or two clean Ctrl taps.
#[derive(Clone)]
enum Trigger {
    Keys(Binding),
    CtrlTaps,
}

impl Trigger {
    fn parse(value: &str) -> Result<Self, String> {
        if value.eq_ignore_ascii_case(DOUBLE_CTRL)
            || value.eq_ignore_ascii_case("double ctrl")
            || value == "双击 Ctrl"
        {
            return Ok(Self::CtrlTaps);
        }
        parse(value)
            .map(Self::Keys)
            .ok_or_else(|| "快捷键无效，请使用至少一个修饰键。".to_string())
    }

    fn label(&self) -> String {
        match self {
            Self::Keys(binding) => binding.label.clone(),
            Self::CtrlTaps => DOUBLE_CTRL.to_string(),
        }
    }
}

pub fn start(shortcut: &str, enabled: bool) {
    ENABLED.store(enabled, Ordering::SeqCst);
    let initial = Trigger::parse(shortcut).ok();
    let (ready_tx, ready_rx) = mpsc::channel();
    thread::spawn(move || {
        // SAFETY: a plain query for the current thread's id.
        let id = unsafe { GetCurrentThreadId() };
        let _ = THREAD_ID.set(id);
        HOOK_READY.store(install_hook().is_ok(), Ordering::SeqCst);
        if enabled {
            if let Some(trigger) = initial {
                let _ = register(&trigger);
                if let Ok(mut active) = ACTIVE.lock() {
                    *active = Some(trigger);
                }
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
    let trigger = Trigger::parse(shortcut)?;
    let label = trigger.label();
    if !enabled {
        let _ = change_trigger(None);
        return Ok(label);
    }
    change_trigger(Some(trigger))
}

fn change_trigger(trigger: Option<Trigger>) -> Result<String, String> {
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
        *pending = Some((trigger, sender));
    }
    // SAFETY: the hotkey thread owns its message queue; a plain post to it.
    if let Err(error) = unsafe {
        PostThreadMessageW(thread_id, REREGISTER_MESSAGE, WPARAM(0), LPARAM(0))
    } {
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
    uninstall_hook();
}

fn apply_pending() {
    let Some((trigger, sender)) = PENDING.lock().ok().and_then(|mut pending| pending.take()) else {
        return;
    };
    let previous = ACTIVE.lock().ok().and_then(|mut active| active.take());
    if let Some(previous) = &previous {
        unregister(previous);
    }
    let Some(trigger) = trigger else {
        let _ = sender.send(Ok(String::new()));
        return;
    };
    match register(&trigger) {
        Ok(()) => {
            if let Ok(mut active) = ACTIVE.lock() {
                *active = Some(trigger.clone());
            }
            let _ = sender.send(Ok(trigger.label()));
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

fn register(trigger: &Trigger) -> Result<(), String> {
    match trigger {
        Trigger::Keys(binding) => {
            // SAFETY: the hotkey thread registers its own key; id and modifiers are plain values.
            unsafe {
                RegisterHotKey(
                    None,
                    HOTKEY_ID,
                    HOT_KEY_MODIFIERS(binding.modifiers),
                    binding.key_code,
                )
            }
            .map_err(|error| format!("快捷键 {} 已被占用或无法注册：{error}", binding.label))
        }
        Trigger::CtrlTaps => {
            if !HOOK_READY.load(Ordering::SeqCst) {
                return Err("无法监听双击 Ctrl：键盘钩子未安装。".to_string());
            }
            CTRL_TAPS_ENABLED.store(true, Ordering::SeqCst);
            CTRL_TAPS.with(|taps| *taps.borrow_mut() = CtrlTaps::default());
            Ok(())
        }
    }
}

fn unregister(trigger: &Trigger) {
    match trigger {
        Trigger::Keys(_) => {
            // SAFETY: the hotkey thread releases its own registration.
            let _ = unsafe { UnregisterHotKey(None, HOTKEY_ID) };
        }
        Trigger::CtrlTaps => CTRL_TAPS_ENABLED.store(false, Ordering::SeqCst),
    }
}

thread_local! {
    static CTRL_TAPS: RefCell<CtrlTaps> = RefCell::new(CtrlTaps::default());
    static KEYBOARD_HOOK: RefCell<Option<HHOOK>> = const { RefCell::new(None) };
}

fn install_hook() -> Result<(), String> {
    KEYBOARD_HOOK.with(|slot| {
        if slot.borrow().is_some() {
            return Ok(());
        }
        CTRL_TAPS.with(|taps| *taps.borrow_mut() = CtrlTaps::default());
        // SAFETY: the installing thread pumps messages until it drops the hook.
        let hook = unsafe { SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard_event), None, 0) }
            .map_err(|error| format!("无法监听双击 Ctrl：{error}"))?;
        *slot.borrow_mut() = Some(hook);
        Ok(())
    })
}

fn uninstall_hook() {
    KEYBOARD_HOOK.with(|slot| {
        if let Some(hook) = slot.borrow_mut().take() {
            // SAFETY: the hook came from SetWindowsHookExW on this thread.
            let _ = unsafe { UnhookWindowsHookEx(hook) };
        }
    });
}

unsafe extern "system" fn keyboard_event(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    let message = wparam.0 as u32;
    if code == HC_ACTION as i32
        && matches!(message, WM_KEYDOWN | WM_KEYUP | WM_SYSKEYDOWN | WM_SYSKEYUP)
    {
        // SAFETY: Windows supplies a readable KBDLLHOOKSTRUCT during an HC_ACTION callback.
        let event = unsafe { &*(lparam.0 as *const KBDLLHOOKSTRUCT) };
        let down = matches!(message, WM_KEYDOWN | WM_SYSKEYDOWN);
        if down && NAV_ENABLED.load(Ordering::SeqCst) {
            let action = match event.vkCode {
                key if key == u32::from(VK_UP.0) => 1,
                key if key == u32::from(VK_DOWN.0) => 2,
                key if key == u32::from(VK_RETURN.0) => 3,
                key if key == u32::from(VK_ESCAPE.0) => 4,
                _ => 0,
            };
            if action != 0 {
                NAV_ACTION.store(action, Ordering::SeqCst);
                // Observed only: the key still reaches the window, so a stale
                // hook can never take the keyboard away from other apps.
            }
        }
        if !CTRL_TAPS_ENABLED.load(Ordering::SeqCst) {
            // SAFETY: pass the original hook parameters through.
            return unsafe { CallNextHookEx(None, code, wparam, lparam) };
        }
        let key = match event.vkCode {
            key if key == u32::from(VK_LCONTROL.0) => Key::LeftCtrl,
            key if key == u32::from(VK_RCONTROL.0) => Key::RightCtrl,
            _ => Key::Other,
        };
        let other_modifier_held = if matches!(key, Key::LeftCtrl | Key::RightCtrl) {
            // The callback precedes the async state update for the current key: query others.
            event.flags.contains(LLKHF_ALTDOWN)
                || [VK_LCONTROL, VK_RCONTROL, VK_SHIFT, VK_MENU, VK_LWIN, VK_RWIN]
                    .into_iter()
                    .any(|modifier| {
                        // SAFETY: GetAsyncKeyState takes a virtual key and reads no caller memory.
                        u32::from(modifier.0) != event.vkCode
                            && unsafe { GetAsyncKeyState(i32::from(modifier.0)) } < 0
                    })
        } else {
            false
        };
        let fired = CTRL_TAPS.with(|taps| {
            taps.borrow_mut().event(
                key,
                down,
                event.time,
                other_modifier_held,
            )
        });
        if fired && ENABLED.load(Ordering::SeqCst) {
            REQUEST.store(true, Ordering::SeqCst);
        }
    }
    // SAFETY: pass the original hook parameters through for every event, negative codes too.
    unsafe { CallNextHookEx(None, code, wparam, lparam) }
}

const MAX_HOLD_MS: u32 = 250;
const MAX_GAP_MS: u32 = 300;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Key {
    LeftCtrl,
    RightCtrl,
    Other,
}

/// Recognizes two clean Ctrl press-and-release cycles.
#[derive(Default)]
struct CtrlTaps {
    held: u8,
    press: Option<u32>,
    first_release: Option<u32>,
}

impl CtrlTaps {
    fn event(&mut self, key: Key, down: bool, time: u32, modifier_held: bool) -> bool {
        let bit = match key {
            Key::LeftCtrl => 1,
            Key::RightCtrl => 2,
            Key::Other => {
                if down || self.held != 0 {
                    self.cancel();
                }
                return false;
            }
        };
        if down {
            self.press = if self.held == 0 && !modifier_held {
                if self
                    .first_release
                    .is_some_and(|first| time.wrapping_sub(first) > MAX_GAP_MS)
                {
                    self.first_release = None;
                }
                Some(time)
            } else {
                self.cancel();
                None
            };
            self.held |= bit;
            return false;
        }
        if self.held & bit == 0 {
            self.cancel();
            return false;
        }
        self.held &= !bit;
        let clean = self
            .press
            .take()
            .is_some_and(|start| time.wrapping_sub(start) <= MAX_HOLD_MS)
            && self.held == 0
            && !modifier_held;
        if !clean {
            self.cancel();
            return false;
        }
        if self.first_release.take().is_some() {
            return true;
        }
        self.first_release = Some(time);
        false
    }

    fn cancel(&mut self) {
        self.press = None;
        self.first_release = None;
    }
}

#[cfg(test)]
mod tests {
    use super::{CtrlTaps, Key::*};

    fn events(events: &[(super::Key, bool, u32, bool)]) -> Vec<usize> {
        let mut taps = CtrlTaps::default();
        events
            .iter()
            .enumerate()
            .filter_map(|(index, &(key, down, time, modifier))| {
                taps.event(key, down, time, modifier).then_some(index)
            })
            .collect()
    }

    #[test]
    fn fires_once_at_the_second_release_and_accepts_either_ctrl() {
        assert_eq!(
            events(&[
                (LeftCtrl, true, 0, false),
                (LeftCtrl, false, 40, false),
                (RightCtrl, true, 110, false),
                (RightCtrl, false, 140, false),
                (LeftCtrl, true, 190, false),
                (LeftCtrl, false, 220, false),
            ]),
            [3]
        );
    }

    #[test]
    fn other_keys_cancel_either_tap() {
        let tap = [(LeftCtrl, true, 0, false), (LeftCtrl, false, 60, false)];
        let copy = [
            (LeftCtrl, true, 0, false),
            (Other, true, 20, false),
            (Other, false, 40, false),
            (LeftCtrl, false, 60, false),
        ];
        for (first, second) in [
            (tap.as_slice(), copy.as_slice()),
            (copy.as_slice(), tap.as_slice()),
        ] {
            let sequence = first
                .iter()
                .copied()
                .chain(second.iter().map(|&(key, down, time, modifier)| {
                    (key, down, time + 100, modifier)
                }))
                .collect::<Vec<_>>();
            assert!(events(&sequence).is_empty());
        }
    }

    #[test]
    fn long_hold_overlap_and_existing_modifiers_cancel() {
        for bad in [
            vec![
                (LeftCtrl, true, 0, false),
                (RightCtrl, true, 10, false),
                (LeftCtrl, false, 20, false),
                (RightCtrl, false, 30, false),
            ],
            vec![
                (LeftCtrl, true, 0, false),
                (LeftCtrl, true, 20, false),
                (LeftCtrl, false, 40, false),
            ],
            vec![(LeftCtrl, true, 0, false), (LeftCtrl, false, 251, false)],
            vec![(LeftCtrl, true, 0, true), (LeftCtrl, false, 40, false)],
        ] {
            let mut sequence = bad;
            sequence.extend([(LeftCtrl, true, 270, false), (LeftCtrl, false, 310, false)]);
            assert!(events(&sequence).is_empty());
        }
    }

    #[test]
    fn a_gap_longer_than_the_limit_starts_over() {
        assert_eq!(
            events(&[
                (LeftCtrl, true, 0, false),
                (LeftCtrl, false, 30, false),
                (LeftCtrl, true, 331, false),
                (LeftCtrl, false, 370, false),
                (Other, true, 380, false),
                (Other, false, 390, false),
                (LeftCtrl, true, 440, false),
                (LeftCtrl, false, 470, false),
                (LeftCtrl, true, 510, false),
                (LeftCtrl, false, 550, false),
            ]),
            [9]
        );
    }

    #[test]
    fn message_ticks_can_wrap() {
        assert_eq!(
            events(&[
                (LeftCtrl, true, u32::MAX - 60, false),
                (LeftCtrl, false, u32::MAX - 20, false),
                (RightCtrl, true, 15, false),
                (RightCtrl, false, 40, false),
            ]),
            [3]
        );
    }
}
