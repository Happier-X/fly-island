// SPDX-License-Identifier: GPL-3.0-only
//! 屏幕选区（CAP-2）：Ctrl+A 当前屏，再按选整个虚拟桌面。

use windows::Win32::Foundation::POINT;
use windows::Win32::Graphics::Gdi::{GetMonitorInfoW, MONITOR_DEFAULTTONEAREST, MONITORINFO, MonitorFromPoint};
use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;
use super::geometry::{SelRect, intersect_rect};

impl super::Overlay {
    pub(super) fn select_screen(&mut self) {
        // 拖动期间不能变换坐标系；贴图编辑的选区必须保持锁定。
        if self.locked_selection || self.drag.is_some() || self.obj_drag.is_some()
            || self.ctrl_drag.is_some() || self.draft.is_some() || self.pending_click.is_some()
            || self.phase == super::interaction::Phase::Selecting {
            return;
        }
        let mut point = POINT::default();
        let mut info = MONITORINFO { cbSize: std::mem::size_of::<MONITORINFO>() as u32, ..Default::default() };
        // SAFETY: 输出结构已初始化，MonitorFromPoint 返回的句柄只用于本次查询。
        unsafe {
            if GetCursorPos(&mut point).is_err() { return; }
            let monitor = MonitorFromPoint(point, MONITOR_DEFAULTTONEAREST);
            if !GetMonitorInfoW(monitor, &mut info).as_bool() { return; }
        }
        let screen = info.rcMonitor;
        let Some(current) = monitor_selection(
            (screen.left, screen.top, screen.right - screen.left, screen.bottom - screen.top),
            (self.capture.origin_x, self.capture.origin_y),
            (self.original.width() as i32, self.original.height() as i32),
        ) else { return };
        let full = SelRect { x: 0, y: 0, w: self.original.width() as i32, h: self.original.height() as i32 };
        self.set_selection(Some(if self.selection == Some(current) { full } else { current }));
    }
}

fn monitor_selection(screen: (i32, i32, i32, i32), origin: (i32, i32), size: (i32, i32)) -> Option<SelRect> {
    intersect_rect(
        SelRect { x: screen.0 - origin.0, y: screen.1 - origin.1, w: screen.2, h: screen.3 },
        SelRect { x: 0, y: 0, w: size.0, h: size.1 },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn negative_monitor_coordinates_translate_to_capture_pixels() {
        assert_eq!(monitor_selection((-1920, -200, 1920, 1080), (-1920, -200), (4480, 1640)),
            Some(SelRect { x: 0, y: 0, w: 1920, h: 1080 }));
        assert_eq!(monitor_selection((0, 0, 2560, 1440), (-1920, -200), (4480, 1640)),
            Some(SelRect { x: 1920, y: 200, w: 2560, h: 1440 }));
    }
    #[test]
    fn changing_monitor_is_clipped_to_frozen_capture() {
        assert_eq!(monitor_selection((100, 0, 300, 200), (0, 0), (200, 200)),
            Some(SelRect { x: 100, y: 0, w: 100, h: 200 }));
        assert_eq!(monitor_selection((500, 0, 300, 200), (0, 0), (200, 200)), None);
    }
}
