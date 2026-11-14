// SPDX-License-Identifier: GPL-3.0-only
//! 图片贴图的本地操作：置顶、锁定、原尺寸与快捷键（PIN-3/4）。

use windows::Win32::Foundation::HANDLE;
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::core::w;

use super::window::PinWindow;

const LOCK_PROPERTY: windows::core::PCWSTR = w!("focusd.pin.locked");

impl PinWindow {
    pub(super) fn locked(&self) -> bool {
        // SAFETY: 只读当前窗口上由本模块写入的布尔属性，不解引用 HANDLE。
        !unsafe { GetPropW(self.hwnd(), LOCK_PROPERTY) }.0.is_null()
    }

    pub(super) fn toggle_locked(&self) {
        // SAFETY: 属性只存非空哨兵，不含分配的内存；窗口销毁时系统移除属性。
        unsafe {
            if self.locked() {
                let _ = RemovePropW(self.hwnd(), LOCK_PROPERTY);
            } else {
                let _ = SetPropW(self.hwnd(), LOCK_PROPERTY, Some(HANDLE(1usize as *mut _)));
            }
        }
    }

    pub(super) fn topmost(&self) -> bool {
        // SAFETY: 查询本窗口样式，无外部指针。
        unsafe { GetWindowLongPtrW(self.hwnd(), GWL_EXSTYLE) as u32 & WS_EX_TOPMOST.0 != 0 }
    }

    pub(super) fn toggle_topmost(&self) {
        let after = if self.topmost() { HWND_NOTOPMOST } else { HWND_TOPMOST };
        // SAFETY: 仅改变本窗口 Z 序，不移动、不激活窗口。
        let _ = unsafe {
            SetWindowPos(self.hwnd(), Some(after), 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE)
        };
    }

    /// 只处理首次按下且修饰键精确匹配的操作，避免长按切换和 Alt+Space 误触。
    /// `sys` 为真时表示消息来自 `WM_SYSKEYDOWN`（按着 Alt），方向键仍需照常微调。
    pub(super) fn operation_key(&mut self, vk: u32, mods: u32, repeated: bool, sys: bool) -> bool {
        if sys { return false; }
        let Some(action) = action_key(vk, mods) else { return false };
        if repeated { return true; }
        match action {
            KeyAction::Lock => self.toggle_locked(),
            KeyAction::Topmost => self.toggle_topmost(),
            KeyAction::Copy => super::menu::copy(self),
            KeyAction::Save => super::menu::save(self.hwnd()),
            KeyAction::Edit if !self.locked() => super::request_edit(self.hwnd()),
            _ => {}
        }
        true
    }
}

#[derive(Debug, PartialEq, Eq)]
enum KeyAction { Lock, Topmost, Copy, Save, Edit }

fn action_key(vk: u32, mods: u32) -> Option<KeyAction> {
    match (vk, mods) {
        (0x4C, 0) => Some(KeyAction::Lock),
        (0x54, 0) => Some(KeyAction::Topmost),
        (0x43, 2) => Some(KeyAction::Copy),
        (0x53, 2) => Some(KeyAction::Save),
        (0x20, 0) => Some(KeyAction::Edit),
        _ => None,
    }
}

/// 按步数调整而非乘当前倍率：Alt 每档 1%，普通每档 10%。
pub(super) fn wheel_scale(current: f32, delta: i16, fine: bool) -> f32 {
    (current + delta as f32 / 120.0 * if fine { 0.01 } else { 0.1 }).clamp(0.1, 8.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modifiers_do_not_steal_system_or_text_shortcuts() {
        assert_eq!(action_key(0x20, 1), None); // Alt+Space system menu
        assert_eq!(action_key(0x4C, 2), None); // Ctrl+L is not lock
        assert_eq!(action_key(0x54, 0), Some(KeyAction::Topmost));
        assert_eq!(action_key(0x43, 2), Some(KeyAction::Copy));
        assert_eq!(action_key(0x53, 2), Some(KeyAction::Save));
    }

    #[test]
    fn zoom_steps_are_reversible_and_support_high_resolution_wheels() {
        let up = wheel_scale(1.0, 120, false);
        assert!((up - 1.1).abs() < 0.0001);
        assert!((wheel_scale(up, -120, false) - 1.0).abs() < 0.0001);
        assert!((wheel_scale(1.0, 120, true) - 1.01).abs() < 0.0001);
        assert!((wheel_scale(1.0, 60, true) - 1.005).abs() < 0.0001);
        assert_eq!(wheel_scale(0.1, -120, false), 0.1);
        assert_eq!(wheel_scale(8.0, 120, false), 8.0);
    }
}
