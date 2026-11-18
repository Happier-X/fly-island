//! A thread-bound observer for low-level keyboard messages. Keystrokes always pass through.

use std::{cell::RefCell, marker::PhantomData, rc::Rc};
use windows::Win32::{
    Foundation::{LPARAM, LRESULT, WPARAM},
    UI::{
        Input::KeyboardAndMouse::{
            GetAsyncKeyState, VK_LCONTROL, VK_LWIN, VK_MENU, VK_RCONTROL, VK_RWIN, VK_SHIFT,
        },
        WindowsAndMessaging::{
            CallNextHookEx, SetWindowsHookExW, UnhookWindowsHookEx, HC_ACTION, HHOOK,
            KBDLLHOOKSTRUCT, LLKHF_ALTDOWN, WH_KEYBOARD_LL, WM_KEYDOWN, WM_KEYUP, WM_SYSKEYDOWN,
            WM_SYSKEYUP,
        },
    },
};

pub(crate) struct KeyEvent {
    pub(crate) key: u32,
    pub(crate) down: bool,
    pub(crate) time: u32,
    pub(crate) other_modifier_held: bool,
}

type Observer = Box<dyn FnMut(KeyEvent)>;
thread_local! {
    static OBSERVER: RefCell<Option<Observer>> = RefCell::new(None);
}

pub(crate) struct KeyboardHook(HHOOK, PhantomData<Rc<()>>);

impl KeyboardHook {
    pub(crate) fn install(observer: impl FnMut(KeyEvent) + 'static) -> Result<Self, String> {
        OBSERVER.with(|slot| {
            if slot.borrow().is_some() {
                return Err("A keyboard observer is already installed on this thread.".into());
            }
            let instance =
                super::message_window::module_instance().map_err(|error| error.to_string())?;
            // SAFETY: the module owns `keyboard_event`; the installing thread pumps messages
            // until the returned guard unhooks on that same thread.
            let handle = unsafe {
                SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard_event), Some(instance), 0)
            }
            .map_err(|error| error.to_string())?;
            *slot.borrow_mut() = Some(Box::new(observer));
            Ok(Self(handle, PhantomData))
        })
    }
}

impl Drop for KeyboardHook {
    fn drop(&mut self) {
        OBSERVER.with(|slot| *slot.borrow_mut() = None);
        // SAFETY: this is the hook returned by SetWindowsHookExW, released once on its owner thread.
        let _ = unsafe { UnhookWindowsHookEx(self.0) };
    }
}

unsafe extern "system" fn keyboard_event(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    let message = wparam.0 as u32;
    if code == HC_ACTION as i32
        && matches!(message, WM_KEYDOWN | WM_KEYUP | WM_SYSKEYDOWN | WM_SYSKEYUP)
    {
        // SAFETY: Windows supplies a readable KBDLLHOOKSTRUCT during an HC_ACTION callback.
        let event = unsafe { &*(lparam.0 as *const KBDLLHOOKSTRUCT) };
        let key = event.vkCode;
        let other_modifier_held =
            if key == u32::from(VK_LCONTROL.0) || key == u32::from(VK_RCONTROL.0) {
                // The callback precedes the async state update for the current key. Query only other keys.
                event.flags.contains(LLKHF_ALTDOWN)
                    || [
                        VK_LCONTROL,
                        VK_RCONTROL,
                        VK_SHIFT,
                        VK_MENU,
                        VK_LWIN,
                        VK_RWIN,
                    ]
                    .into_iter()
                    .any(|modifier| {
                        // SAFETY: GetAsyncKeyState takes a virtual key and accesses no caller memory.
                        u32::from(modifier.0) != key
                            && unsafe { GetAsyncKeyState(i32::from(modifier.0)) } < 0
                    })
            } else {
                false
            };
        OBSERVER.with(|slot| {
            if let Ok(mut slot) = slot.try_borrow_mut() {
                if let Some(observer) = slot.as_mut() {
                    observer(KeyEvent {
                        key,
                        down: matches!(message, WM_KEYDOWN | WM_SYSKEYDOWN),
                        time: event.time,
                        other_modifier_held,
                    });
                }
            }
        });
    }
    // SAFETY: pass the original hook parameters through for every event, including negative codes.
    unsafe { CallNextHookEx(None, code, wparam, lparam) }
}
