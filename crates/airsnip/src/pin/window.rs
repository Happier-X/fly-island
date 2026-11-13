// SPDX-License-Identifier: GPL-3.0-only
//! 贴图窗口（PIN-3/4）：分层置顶窗口、像素上屏、缩放/透明度状态。
//! 消息分发与交互在 `wndproc` 子模块。

use tiny_skia::Pixmap;
use windows::Win32::Foundation::{COLORREF, HINSTANCE, HWND, POINT, RECT, SIZE};
use windows::Win32::Graphics::Gdi::{AC_SRC_ALPHA, AC_SRC_OVER, BLENDFUNCTION, DeleteDC, DeleteObject, HBITMAP, HDC};
use windows::Win32::UI::Input::Ime::{HIMC, ImmAssociateContext};
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::core::{PCWSTR, w};

use super::border::{self, BORDER_PAD};
use super::dib::create_dib;
use super::scale;
use super::select::{BAND_COLOR, BAND_EDGE_ALPHA, BAND_FILL_ALPHA, Select};
use super::zoom::MAX_SIDE;

const WINDOW_CLASS: PCWSTR = w!("airsnip.pin");
const ALPHA_MIN: u8 = 12;
/// 滚轮/透明度单步（wndproc 与测试共用）。
pub(super) const ALPHA_STEP: i32 = 24;

/// 单个贴图：原始图 + 当前缩放/透明度 + 上屏 DIB。
pub(super) struct PinWindow {
    /// 原始像素（premultiplied，含边框扩展；缩放源）。
    pub(super) original: Pixmap,
    /// 当前缩放结果（premultiplied，透明度叠加时复用）。
    pub(super) scaled: Pixmap,
    /// 干净内容图（premultiplied，无边框扩展）：边框开关时重新烘焙用。
    pub(super) raw: Pixmap,
    /// 窗口透明度 0-255（先缩放后叠加，独立于像素通道）。
    alpha: u8,
    /// 内容区在 original 中的四周偏移（边框扩展；0 = 无边框）。
    pub(super) pad: u32,
    hwnd: HWND,
    mem_dc: HDC,
    dib_bmp: HBITMAP,
    dib_bits: *mut u8,
    dib_w: u32,
    dib_h: u32,
    /// 拖动状态（wndproc 读写）。
    pub(super) dragging: bool,
    /// 按下时光标位置 / 窗口左上角（拖动增量）。
    pub(super) drag_cursor: (i32, i32),
    pub(super) win_origin: (i32, i32),
    /// 框选取字（PIN-OCR）：None = 未进入框选态。
    pub(super) select: Option<Select>,
}

impl Drop for PinWindow {
    fn drop(&mut self) {
        unsafe {
            if !self.dib_bmp.0.is_null() {
                let _ = DeleteObject(self.dib_bmp.into());
            }
            if !self.mem_dc.0.is_null() {
                let _ = DeleteDC(self.mem_dc);
            }
        }
    }
}

/// 注册窗口类并创建贴图窗口（必须在主线程）。
pub(super) fn spawn(
    mut pixmap: Pixmap,
    hinstance: HINSTANCE,
    x: i32,
    y: i32,
    show_border: bool,
) -> Result<HWND, String> {
    unsafe {
        // 入库：straight → premultiplied；随后按设置烘焙边框（扩大原图）
        let data = scale::premultiply_rgba(pixmap.data());
        let Some(in_size) = tiny_skia::IntSize::from_wh(pixmap.width(), pixmap.height()) else {
            return Err("贴图像素图尺寸非法".into());
        };
        let premul = Pixmap::from_vec(data, in_size).ok_or("贴图像素图尺寸非法")?;
        let pad = if show_border { BORDER_PAD } else { 0 };
        let raw = premul;
        let framed = border::with_border(&raw, show_border).ok_or("贴图边框合成失败")?;
        pixmap = framed;

        let wc = WNDCLASSW {
            // CS_DBLCLKS：双击关闭（PIN-4）
            style: CS_DBLCLKS,
            lpfnWndProc: Some(super::wndproc::wndproc),
            hInstance: hinstance,
            hCursor: LoadCursorW(None, IDC_ARROW).unwrap_or_default(),
            lpszClassName: WINDOW_CLASS,
            ..Default::default()
        };
        if RegisterClassW(&wc) == 0 && windows::Win32::Foundation::GetLastError().0 != 1410 {
            return Err("贴图窗口类注册失败".into());
        }
        let hwnd = CreateWindowExW(
            // WS_EX_LAYERED：UpdateLayeredWindow 提交带 alpha 的像素（PNG 透明区域透出桌面）
            WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TOOLWINDOW,
            WINDOW_CLASS,
            w!("airsnip.pin"),
            WS_POPUP,
            x,
            y,
            pixmap.width() as i32,
            pixmap.height() as i32,
            None,
            None,
            Some(hinstance),
            None,
        )
        .map_err(|e| format!("创建贴图窗口失败：{e}"))?;

        // 禁用输入法：中文 IME 会截走 Y 等字母键（出候选字），贴图窗口内的
        // 边框快捷键将失效；贴图不输入文本，直接永久屏蔽（M2b 实测缺陷）。
        let _ = ImmAssociateContext(hwnd, HIMC(std::ptr::null_mut()));

        let mut win = PinWindow {
            raw,
            original: pixmap.clone(),
            scaled: pixmap,
            alpha: 255,
            pad,
            hwnd,
            mem_dc: HDC::default(),
            dib_bmp: HBITMAP::default(),
            dib_bits: std::ptr::null_mut(),
            dib_w: 0,
            dib_h: 0,
            dragging: false,
            drag_cursor: (0, 0),
            win_origin: (x, y),
            select: None,
        };
        win.rebind()?;
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, Box::into_raw(Box::new(win)) as isize);
        // 分层窗口必须显式显示：UpdateLayeredWindow 不负责首次 ShowWindow，
        // 否则窗口创建成功但永不可见（M2b 实测缺陷）。顺带确保置顶。
        let _ = SetWindowPos(hwnd, Some(HWND_TOPMOST), x, y, 0, 0, SWP_NOSIZE | SWP_SHOWWINDOW);
        Ok(hwnd)
    }
}

impl PinWindow {
    pub(super) fn hwnd(&self) -> HWND {
        self.hwnd
    }

    /// 重建 DIB 并提交像素。缩放/透明度变化都走这里：重采样 → 合成 → 上屏。
    pub(super) fn rebind(&mut self) -> Result<(), String> {
        let need_dib =
            self.dib_bmp.0.is_null() || self.dib_w != self.scaled.width() || self.dib_h != self.scaled.height();
        unsafe {
            if need_dib {
                let (dc, bmp, bits) = create_dib(self.scaled.width(), self.scaled.height())?;
                if !self.dib_bmp.0.is_null() {
                    let _ = DeleteObject(self.dib_bmp.into());
                }
                if !self.mem_dc.0.is_null() {
                    let _ = DeleteDC(self.mem_dc);
                }
                self.mem_dc = dc;
                self.dib_bmp = bmp;
                self.dib_bits = bits;
                self.dib_w = self.scaled.width();
                self.dib_h = self.scaled.height();
            }
            // straight→已按 premultiplied 解析，这里仅合成透明度并转 BGRA
            let bgra = scale::to_bgra_premul(&self.scaled, self.alpha);
            std::ptr::copy_nonoverlapping(bgra.as_ptr(), self.dib_bits, bgra.len());
        }
        self.present();
        Ok(())
    }

    /// 切换边框（M2b）：重新烘焙原图并重采样，保持内容左上角屏幕位置不动。
    pub(super) fn toggle_border(&mut self, on: bool) {
        let new_pad = if on { BORDER_PAD } else { 0 };
        if new_pad == self.pad {
            return;
        }
        let scale = self.scale();
        // 当前内容左上角（屏幕坐标）
        let rect = unsafe {
            let mut r = RECT::default();
            let _ = GetWindowRect(self.hwnd, &mut r);
            r
        };
        let (cx, cy) = (
            rect.left + (self.pad as f32 * scale) as i32,
            rect.top + (self.pad as f32 * scale) as i32,
        );
        // 用干净内容图重新烘焙 + 按当前缩放重采样
        let Some(new_orig) = border::with_border(&self.raw, on) else {
            return;
        };
        self.pad = new_pad;
        self.original = new_orig;
        let w = ((self.original.width() as f32 * scale).round() as u32).clamp(1, MAX_SIDE);
        let h = ((self.original.height() as f32 * scale).round() as u32).clamp(1, MAX_SIDE);
        let Some(nscaled) = scale::resample(&self.original, w, h) else {
            return;
        };
        self.scaled = nscaled;
        let noff = (new_pad as f32 * scale) as i32;
        unsafe {
            let _ = SetWindowPos(
                self.hwnd,
                Some(HWND_TOPMOST),
                cx - noff,
                cy - noff,
                w as i32,
                h as i32,
                SWP_NOACTIVATE | SWP_SHOWWINDOW,
            );
        }
        let _ = self.rebind();
    }

    pub(super) fn scale(&self) -> f32 {
        self.scaled.width() as f32 / self.original.width() as f32
    }

    /// 调整透明度并重绘（无需重新缩放）。
    pub(super) fn adjust_alpha(&mut self, delta: i32) {
        self.alpha = (self.alpha as i32 + delta).clamp(ALPHA_MIN as i32, 255) as u8;
        let _ = self.rebind();
    }

    /// 导出内容像素（右键复制/保存用）：去除边框扩展、premultiplied → straight。
    pub(super) fn export_content(&self) -> Option<Pixmap> {
        scale::extract_content(&self.scaled, self.pad, self.scale())
    }

    /// 框选取字：重绘 `dirty` 区域（窗口客户区坐标），先还原成贴图底色，再叠加选区带。
    /// 只做这一块的像素，避免每次鼠标移动都全图重合成（滚轮精修才是全图）。
    pub(super) fn paint_band(&mut self, dirty: (i32, i32, i32, i32), band: Option<(i32, i32, i32, i32)>) {
        if self.dib_bits.is_null() {
            return;
        }
        let (dw, dh) = (self.dib_w as i32, self.dib_h as i32);
        let x0 = dirty.0.max(0);
        let y0 = dirty.1.max(0);
        let x1 = dirty.0.saturating_add(dirty.2).min(dw).min(self.scaled.width() as i32);
        let y1 = dirty.1.saturating_add(dirty.3).min(dh).min(self.scaled.height() as i32);
        if x1 <= x0 || y1 <= y0 {
            return;
        }
        let sw = self.scaled.width() as usize;
        {
            let data = self.scaled.data();
            for y in y0..y1 {
                for x in x0..x1 {
                    let si = (y as usize * sw + x as usize) * 4;
                    let px = scale::bgra_premul_px(&data[si..si + 4], self.alpha);
                    let di = (y as usize * dw as usize + x as usize) * 4;
                    // SAFETY: DIB 由 rebind 按 dw×dh 分配，di 落在缓冲内（x/y 已按尺寸钳过）。
                    unsafe {
                        std::ptr::copy_nonoverlapping(px.as_ptr(), self.dib_bits.add(di), 4);
                    }
                }
            }
        }
        if let Some(b) = band {
            let (bx, by, bw, bh) = b;
            if bw > 0 && bh > 0 {
                for y in by.max(y0)..(by.saturating_add(bh)).min(y1) {
                    for x in bx.max(x0)..(bx.saturating_add(bw)).min(x1) {
                        let edge = x == bx || y == by || x == bx + bw - 1 || y == by + bh - 1;
                        let (color, alpha) = if edge {
                            (BAND_COLOR, BAND_EDGE_ALPHA)
                        } else {
                            (BAND_COLOR, BAND_FILL_ALPHA)
                        };
                        let di = (y as usize * dw as usize + x as usize) * 4;
                        // SAFETY: 同上，di 落在 DIB 范围内；只改本窗口自己的位图。
                        unsafe {
                            blend_over(std::slice::from_raw_parts_mut(self.dib_bits.add(di), 4), color, alpha);
                        }
                    }
                }
            }
        }
        self.present();
    }

    /// UpdateLayeredWindow 提交当前 DIB 到窗口位置。
    fn present(&self) {
        unsafe {
            let rect = {
                let mut r = RECT::default();
                let _ = GetWindowRect(self.hwnd, &mut r);
                r
            };
            let ppt_dst = POINT {
                x: rect.left,
                y: rect.top,
            };
            let psize = SIZE {
                cx: rect.right - rect.left,
                cy: rect.bottom - rect.top,
            };
            let ppt_src = POINT { x: 0, y: 0 };
            let blend = BLENDFUNCTION {
                BlendOp: AC_SRC_OVER as u8,
                BlendFlags: 0,
                SourceConstantAlpha: 255,
                AlphaFormat: AC_SRC_ALPHA as u8,
            };
            let _ = UpdateLayeredWindow(
                self.hwnd,
                Some(self.mem_dc),
                Some(&ppt_dst),
                Some(&psize),
                Some(self.mem_dc),
                Some(&ppt_src),
                COLORREF(0),
                Some(&blend),
                ULW_ALPHA,
            );
        }
    }
}

/// 把颜色按 alpha（0-255）叠到 premultiplied BGRA 像素上（src-over）。
fn blend_over(px: &mut [u8], color: [u8; 3], alpha: u16) {
    let inv = 255 - alpha;
    px[0] = ((px[0] as u16 * inv + color[2] as u16 * alpha) / 255) as u8;
    px[1] = ((px[1] as u16 * inv + color[1] as u16 * alpha) / 255) as u8;
    px[2] = ((px[2] as u16 * inv + color[0] as u16 * alpha) / 255) as u8;
    px[3] = ((px[3] as u16 * inv + 255 * alpha) / 255) as u8;
}
