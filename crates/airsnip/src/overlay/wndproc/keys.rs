// SPDX-License-Identifier: GPL-3.0-only
//! 键盘 / IME 辅助（CAP-2、EDT-1·EDT-2、EDT-4、ADR 0023）：修饰键状态与 IME 组合串读取。
//! 从 `wndproc` 拆出，让窗口过程只留消息分发，单文件守在 300 行以内。
//! 调用方仍按 `super::wndproc::xxx` 引用（本模块在 `wndproc/mod.rs` 里 `pub(super) use` 转出）。

use windows::Win32::Foundation::HWND;
use windows::Win32::UI::Input::Ime::IME_COMPOSITION_STRING;
use windows::Win32::UI::Input::KeyboardAndMouse::GetKeyState;

/// 修饰键是否按下（`GetKeyState` 高位为 1 表示按下）。
fn key_down(vk: i32) -> bool {
    unsafe { GetKeyState(vk) as u16 & 0x8000 != 0 }
}

/// Ctrl 是否按下（Ctrl + 拖动对象本体 = 移动该对象，ADR 0023）。
pub(in crate::overlay) fn ctrl_down() -> bool {
    key_down(0x11)
}

/// Shift 是否按下（文本输入修饰键 / 绘制约束 EDT-1·EDT-2）。
pub(in crate::overlay) fn shift_down() -> bool {
    key_down(0x10)
}

pub(in crate::overlay) fn alt_down() -> bool {
    key_down(0x12)
}

/// 空格键是否按下（按住空格拖动可移动整个选区，CAP-2）。
pub(in crate::overlay) fn space_down() -> bool {
    key_down(0x20)
}

/// 从 IME 上下文读取组合串/结果串（UTF-16 → String，EDT-4）。
pub(in crate::overlay) fn ime_string(hwnd: HWND, what: IME_COMPOSITION_STRING) -> String {
    unsafe {
        use windows::Win32::UI::Input::Ime::{ImmGetCompositionStringW, ImmGetContext, ImmReleaseContext};
        let ctx = ImmGetContext(hwnd);
        if ctx.0.is_null() {
            return String::new();
        }
        let len = ImmGetCompositionStringW(ctx, what, None, 0);
        let out = if len <= 0 {
            String::new()
        } else {
            let mut buf = vec![0u16; (len / 2) as usize];
            ImmGetCompositionStringW(ctx, what, Some(buf.as_mut_ptr() as *mut core::ffi::c_void), len as u32);
            String::from_utf16_lossy(&buf)
        };
        let _ = ImmReleaseContext(hwnd, ctx);
        out
    }
}

/// 普通按键分发（`WM_KEYDOWN`）：文本输入优先，其余按快捷键 → 工具键的顺序处理。
/// 贴在 `wndproc` 的按键分发里，只负责搬移代码，行为不变。
pub(super) fn on_keydown(
    hwnd: HWND,
    overlay: &mut super::Overlay,
    wparam: windows::Win32::Foundation::WPARAM,
) -> windows::Win32::Foundation::LRESULT {
    use windows::Win32::Foundation::LRESULT;
    use windows::Win32::UI::Input::Ime::ImmGetVirtualKey;

    // 文本输入中：只处理输入相关按键（光标移动/退格/Enter 提交/Esc 取消），
    // 拦截全局快捷键与空格抓手切换（空格作为普通字符经 WM_CHAR 进入文本）
    if overlay.is_editing_text() {
        overlay.on_text_key(wparam.0 as u32, shift_down(), ctrl_down());
        return LRESULT(0);
    }
    // 中文输入法活动时，按键会被改写为 VK_PROCESSKEY(0xE5)，真实键另存。
    // 不禁用 IME（文本工具需要），改用 ImmGetVirtualKey 取回真实 vk，
    // 否则 IME 开启状态下按 S/F3/回车等全部失效（M4 实测缺陷）。
    let vk = if wparam.0 as u32 == 0xE5 {
        unsafe { ImmGetVirtualKey(hwnd) }
    } else {
        wparam.0 as u32
    };
    // 除 Esc 外的按键都解除取消待命（Esc 本身走下面的取消分支）
    if vk != 0x1B {
        overlay.clear_cancel_arm();
    }
    // 空格按下立即刷新光标：系统只在鼠标移动时发 WM_SETCURSOR，不能等它。
    // 贴图编辑模式（ADR 0028）下空格 = 完成（选区锁定，无需空格移动选区）。
    if vk == 0x20 {
        if overlay.locked_selection {
            overlay.finish_output();
        } else {
            overlay.apply_cursor_now(hwnd);
        }
        return LRESULT(0);
    }
    match vk {
        // Esc：与右键共用分层取消（先收面板 / 取消输入；有标注则二次确认，OUT-3/ADR 0027）
        0x1B => overlay.on_cancel(super::cancel_hint::KEY_ESCAPE),
        0x0D => overlay.finish_output(), // Enter：普通=复制关闭，编辑=写回贴图（ADR 0028）
        0x41 if ctrl_down() => overlay.select_screen(), // Ctrl+A 当前屏 / 全屏
        0x43 if ctrl_down() => overlay.on_copy(), // Ctrl+C
        // Ctrl+S 保存（弹另存为对话框）；Ctrl+Shift+S 快速保存到设置目录（EXT-2）
        0x53 if ctrl_down() && shift_down() => overlay.on_quick_save(),
        0x53 if ctrl_down() => overlay.on_save(),
        // PIN-1：贴选区（键可配，默认 F3）；编辑模式下输出组不适用
        k if k == overlay.pin_key && !overlay.locked_selection => overlay.on_pin(),
        // 长截图键（默认 S，设置面板可改）：当前选区作为滚动视口（M4）
        k if k == overlay.longshot_key && !overlay.locked_selection => overlay.on_longshot(),
        // OCR 键（默认 T，设置面板可改，OCR-3）：OCR 常开；清空为「未设置」即失效
        k if k == overlay.ocr_key && !overlay.locked_selection => overlay.on_ocr(),
        0x5A if ctrl_down() => {
            // Ctrl+Z 撤销 / Ctrl+Shift+Z 重做（EDT-7）
            if shift_down() {
                overlay.on_redo()
            } else {
                overlay.on_undo()
            }
        }
        0x59 if ctrl_down() => overlay.on_redo(), // Ctrl+Y
        // 删除选中对象（issue #71 / ADR 0032）：`Del` 与 `Backspace` 硬编码。
        // 仅在真的删掉时消费按键，否则落到下面的工具切换——
        // 这两个键可被用户绑成工具键（不在冲突检测表里，没选中对象时照旧生效）。
        0x2E | 0x08 if overlay.on_delete_selected() => {}
        _ => {
            // 方向键微调（EDT-7 / CAP-2）：1px，Shift 5px；优先于工具切换键
            if let Some((dx, dy)) = crate::app::hotkey::arrow_step(vk, shift_down()) {
                overlay.nudge(dx, dy);
                return LRESULT(0);
            }
            // 工具切换键（M3）：任意键都查映射（不限于数字键），
            // 以便设置面板可把工具键改为字母等
            if let Some(t) = overlay.tool_by_key(vk) {
                overlay.switch_tool(t);
            }
        }
    }
    LRESULT(0)
        }
