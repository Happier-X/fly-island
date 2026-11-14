// SPDX-License-Identifier: GPL-3.0-only
//! 图像载荷投递：把像素图 + 屏幕锚点交给主线程处理（OCR / 贴图共用）。
//! 载荷格式与覆盖层一致：`[u32 w][u32 h][i32 x][i32 y][RGBA]`，由接收方释放。

use tiny_skia::Pixmap;
use windows::Win32::Foundation::{GlobalFree, LPARAM, WPARAM};
use windows::Win32::System::Memory::{GMEM_MOVEABLE, GlobalAlloc, GlobalLock, GlobalUnlock};
use windows::Win32::UI::WindowsAndMessaging::{FindWindowW, MB_ICONERROR, MB_OK, PostMessageW};

/// 投递到主窗口的 `msg`（`WM_OCR_REQUEST` 或 `WM_PIN_FROM_EDITOR`）。
/// 主窗口找不到或分配失败时自行释放并提示，返回是否已投递。
pub(crate) fn post(msg: u32, pixmap: &Pixmap, at: (i32, i32), failure_hint: &str) -> bool {
    let (w, h) = (pixmap.width(), pixmap.height());
    let Some(len) = (w as usize)
        .checked_mul(h as usize)
        .and_then(|n| n.checked_mul(4))
    else {
        crate::app::message_box(None, &format!("{failure_hint}：图像尺寸非法"), MB_OK | MB_ICONERROR);
        return false;
    };
    // SAFETY: 分配 / 锁定 / 拷贝 / 投递成对完成；失败路径各自释放，不泄漏也不重复释放。
    unsafe {
        let Ok(hg) = GlobalAlloc(GMEM_MOVEABLE, 16 + len) else {
            crate::app::message_box(None, &format!("{failure_hint}：内存分配失败"), MB_OK | MB_ICONERROR);
            return false;
        };
        let ptr = GlobalLock(hg);
        if ptr.is_null() {
            let _ = GlobalFree(Some(hg));
            return false;
        }
        *(ptr as *mut u32) = w;
        *((ptr as *mut u32).add(1)) = h;
        let pos = (ptr as *mut u8).add(8) as *mut i32;
        *pos = at.0;
        *pos.add(1) = at.1;
        std::ptr::copy_nonoverlapping(pixmap.data().as_ptr(), (ptr as *mut u8).add(16), len);
        let _ = GlobalUnlock(hg);
        match FindWindowW(crate::app::WINDOW_CLASS, None) {
            Ok(main) => {
                if PostMessageW(Some(main), msg, WPARAM(hg.0 as usize), LPARAM(0)).is_ok() {
                    true
                } else {
                    let _ = GlobalFree(Some(hg));
                    false
                }
            }
            Err(_) => {
                let _ = GlobalFree(Some(hg));
                false
            }
        }
    }
}
