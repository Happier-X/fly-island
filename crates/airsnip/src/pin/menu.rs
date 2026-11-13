// SPDX-License-Identifier: GPL-3.0-only
//! 贴图右键菜单（PIN-4 / PIN-OCR）：编辑、识别文字、复制、保存、关闭、关闭全部。
//! 导出像素：去除阴影扩展、premultiplied → straight 还原（经 `pixmap_of`）。
//!
//! 菜单文案跟着 Windows 显示语言走（中文系统中文、其余英文）：贴图窗口是原生 Win32，
//! 拿不到宿主前端的 i18n 词表。

use windows::Win32::Foundation::{LPARAM, WPARAM};
use windows::Win32::Globalization::GetUserDefaultUILanguage;
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::core::PCWSTR;

use super::window::PinWindow;

const IDM_PIN_EDIT: usize = 41005;
const IDM_PIN_OCR: usize = 41006;
const IDM_PIN_COPY: usize = 41001;
const IDM_PIN_SAVE: usize = 41002;
const IDM_PIN_CLOSE: usize = 41003;
const IDM_PIN_CLOSE_ALL: usize = 41004;

/// 显示右键菜单并分发动作（TrackPopupMenu 模态，内部自带消息循环）。
pub(super) fn on_context_menu(win: &mut PinWindow) {
    let hwnd = win.hwnd();
    // 文案缓冲要先于菜单创建，并活到 TrackPopupMenu 返回（菜单项直接引用这段内存）
    let labels = [
        label("编辑", "Edit"),
        label("识别文字…", "Recognize text..."),
        label("复制", "Copy"),
        label("保存", "Save"),
        label("关闭", "Close"),
        label("关闭全部", "Close all"),
    ];
    let selected = unsafe {
        let mut pt = std::mem::zeroed();
        let _ = GetCursorPos(&mut pt);
        let menu = CreatePopupMenu().ok();
        let Some(menu) = menu else { return };
        let _ = AppendMenuW(menu, MF_STRING, IDM_PIN_EDIT, PCWSTR(labels[0].as_ptr()));
        let _ = AppendMenuW(menu, MF_STRING, IDM_PIN_OCR, PCWSTR(labels[1].as_ptr()));
        let _ = AppendMenuW(menu, MF_STRING, IDM_PIN_COPY, PCWSTR(labels[2].as_ptr()));
        let _ = AppendMenuW(menu, MF_STRING, IDM_PIN_SAVE, PCWSTR(labels[3].as_ptr()));
        let _ = AppendMenuW(menu, MF_SEPARATOR, 0, None);
        let _ = AppendMenuW(menu, MF_STRING, IDM_PIN_CLOSE, PCWSTR(labels[4].as_ptr()));
        let _ = AppendMenuW(menu, MF_STRING, IDM_PIN_CLOSE_ALL, PCWSTR(labels[5].as_ptr()));
        let cmd = TrackPopupMenu(
            menu,
            TPM_LEFTALIGN | TPM_RIGHTBUTTON | TPM_RETURNCMD,
            pt.x,
            pt.y,
            None,
            hwnd,
            None,
        );
        let _ = DestroyMenu(menu);
        cmd.0 as usize
    };
    match selected {
        // 进入标注编辑（ADR 0028，issue #56）：与贴图内按空格等价
        IDM_PIN_EDIT => super::request_edit(hwnd),
        // 框选取字（PIN-OCR）：进入框选态，拖出矩形后松手识别
        IDM_PIN_OCR => win.begin_select(),
        IDM_PIN_COPY => {
            if let Some(pm) = win.export_content() {
                std::thread::spawn(move || {
                    if let Err(e) = crate::output::copy_to_clipboard(&pm) {
                        crate::app::message_box(None, &format!("复制失败：{e}"), MB_OK | MB_ICONERROR);
                    }
                });
            }
        }
        IDM_PIN_SAVE => {
            // 另存为（OUT-2）：交主窗口弹原生对话框（App 持有配置，可记住上次格式与目录）
            let _ = unsafe {
                let Ok(main) = FindWindowW(crate::app::WINDOW_CLASS, None) else {
                    return;
                };
                PostMessageW(
                    Some(main),
                    crate::app::WM_PIN_SAVE_REQUEST,
                    WPARAM(hwnd.0 as usize),
                    LPARAM(0),
                )
            };
        }
        IDM_PIN_CLOSE => {
            let _ = unsafe { PostMessageW(Some(hwnd), WM_CLOSE, WPARAM(0), LPARAM(0)) };
        }
        IDM_PIN_CLOSE_ALL => super::close_all(),
        _ => {}
    }
}

/// 菜单文案：Windows 显示语言是中文时用中文，其余用英文。
fn label(zh: &str, en: &str) -> Vec<u16> {
    pick(zh, en).encode_utf16().chain([0]).collect()
}

fn pick<'a>(zh: &'a str, en: &'a str) -> &'a str {
    if chinese_ui() { zh } else { en }
}

fn chinese_ui() -> bool {
    // SAFETY: 无参只读系统查询，不涉及调用方内存。
    let lang = unsafe { GetUserDefaultUILanguage() };
    lang & 0x3FF == 0x04 // LANG_CHINESE：主语言 ID 在低 10 位
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn label_is_null_terminated() {
        let text = label("识别文字…", "Recognize text...");
        assert_eq!(text.last(), Some(&0));
        let shown = if chinese_ui() { "识别文字…" } else { "Recognize text..." };
        let decoded: Vec<u16> = shown.encode_utf16().collect();
        assert_eq!(&text[..text.len() - 1], decoded.as_slice());
    }

    #[test]
    fn picks_language_by_ui_setting() {
        assert_eq!(pick("编辑", "Edit"), if chinese_ui() { "编辑" } else { "Edit" });
    }
}
