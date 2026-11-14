// SPDX-License-Identifier: GPL-3.0-only
//! 贴图菜单（PIN-4/PIN-OCR）：共享复制/保存入口与可恢复的窗口状态操作。

use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows::Win32::Globalization::GetUserDefaultUILanguage;
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::core::PCWSTR;

use super::window::PinWindow;

const EDIT: usize = 41005;
const OCR: usize = 41006;
const COPY: usize = 41001;
const SAVE: usize = 41002;
const CLOSE: usize = 41003;
const CLOSE_ALL: usize = 41004;
const LOCK: usize = 41007;
const TOPMOST: usize = 41008;
const RESET: usize = 41009;

pub(super) fn on_context_menu(win: &mut PinWindow) {
    let rows = [
        (EDIT, label("编辑\tSpace", "Edit\tSpace")),
        (OCR, label("识别文字…\tAlt+拖动", "Recognize text...\tAlt+Drag")),
        (COPY, label("复制\tCtrl+C", "Copy\tCtrl+C")),
        (SAVE, label("保存\tCtrl+S", "Save\tCtrl+S")),
        (TOPMOST, label("置顶\tT", "Always on top\tT")),
        (LOCK, label("锁定位置与大小\tL", "Lock position and size\tL")),
        (RESET, label("原始大小\t中键", "Original size\tMiddle click")),
        (CLOSE, label("关闭\tEsc", "Close\tEsc")),
        (CLOSE_ALL, label("关闭全部", "Close all")),
    ];
    let selected = unsafe {
        let mut pt = std::mem::zeroed();
        let _ = GetCursorPos(&mut pt);
        let Ok(menu) = CreatePopupMenu() else { return };
        for (id, text) in &rows {
            if *id == TOPMOST || *id == CLOSE { let _ = AppendMenuW(menu, MF_SEPARATOR, 0, None); }
            let checked = (*id == LOCK && win.locked()) || (*id == TOPMOST && win.topmost());
            let disabled = win.locked() && matches!(*id, EDIT | OCR | RESET);
            let flags = MF_STRING | if checked { MF_CHECKED } else { MF_UNCHECKED }
                | if disabled { MF_GRAYED } else { MF_ENABLED };
            let _ = AppendMenuW(menu, flags, *id, PCWSTR(text.as_ptr()));
        }
        let cmd = TrackPopupMenu(menu, TPM_LEFTALIGN | TPM_RIGHTBUTTON | TPM_RETURNCMD,
            pt.x, pt.y, None, win.hwnd(), None);
        let _ = DestroyMenu(menu);
        cmd.0 as usize
    };
    match selected {
        EDIT if !win.locked() => super::request_edit(win.hwnd()),
        OCR if !win.locked() => win.begin_select(),
        COPY => copy(win),
        SAVE => save(win.hwnd()),
        LOCK => win.toggle_locked(),
        TOPMOST => win.toggle_topmost(),
        RESET if !win.locked() => win.reset_zoom(),
        CLOSE => { let _ = unsafe { PostMessageW(Some(win.hwnd()), WM_CLOSE, WPARAM(0), LPARAM(0)) }; }
        CLOSE_ALL => super::close_all(),
        _ => {}
    }
}

pub(super) fn copy(win: &mut PinWindow) {
    win.finalize_preview();
    if let Some(pm) = win.export_content() {
        std::thread::spawn(move || {
            if let Err(e) = crate::output::copy_to_clipboard(&pm) {
                crate::app::message_box(None, &format!("{}: {e}", pick("复制失败", "Copy failed")), MB_OK | MB_ICONERROR);
            }
        });
    }
}

pub(super) fn save(hwnd: HWND) {
    let _ = unsafe {
        let Ok(main) = FindWindowW(crate::app::WINDOW_CLASS, None) else { return };
        PostMessageW(Some(main), crate::app::WM_PIN_SAVE_REQUEST, WPARAM(hwnd.0 as usize), LPARAM(0))
    };
}

fn label(zh: &str, en: &str) -> Vec<u16> {
    pick(zh, en).encode_utf16().chain([0]).collect()
}

fn pick<'a>(zh: &'a str, en: &'a str) -> &'a str {
    if chinese_ui() { zh } else { en }
}

fn chinese_ui() -> bool {
    // SAFETY: 无参只读系统查询，不涉及调用方内存。
    unsafe { GetUserDefaultUILanguage() & 0x3FF == 0x04 }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn menu_labels_are_null_terminated() {
        assert_eq!(label("编辑", "Edit").last(), Some(&0));
        assert_eq!(pick("编辑", "Edit"), if chinese_ui() { "编辑" } else { "Edit" });
    }
}
