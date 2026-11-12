// SPDX-License-Identifier: GPL-3.0-only
//! 抓屏（CAP-1）：一次性抓取全部虚拟屏幕（含负坐标/混合 DPI 布局，CAP-5）。
//!
//! HDR 桌面下走 DXGI Desktop Duplication 取 FP16 帧再色调映射（`hdr`，issue #75）；
//! 纯 SDR 环境仍用 `BitBlt`（`CAPTUREBLT`），更快且行为已验证。
//! 两条路径输出同为 BGRA→RGBA 后的像素数据，供 tiny-skia 使用。

mod hdr;
mod hdr_display;
mod region;
mod tonemap;

pub use region::RegionCapture;

use std::mem::size_of;
use std::ptr::null_mut;

use windows::Win32::Graphics::Gdi::{
    BI_RGB, BITMAPINFO, BITMAPINFOHEADER, BitBlt, CAPTUREBLT, CreateCompatibleDC, CreateDIBSection, DIB_RGB_COLORS,
    DeleteDC, DeleteObject, GetDC, ReleaseDC, SRCCOPY, SelectObject,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GetSystemMetrics, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN, SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN,
};

/// 一次抓屏结果：整个虚拟屏幕的像素与原点。
pub struct CapturedScreen {
    /// 虚拟屏幕宽度（像素）。
    pub width: u32,
    /// 虚拟屏幕高度（像素）。
    pub height: u32,
    /// 虚拟屏幕左上角物理坐标 X（可为负，多显示器负坐标布局）。
    pub origin_x: i32,
    /// 虚拟屏幕左上角物理坐标 Y。
    pub origin_y: i32,
    /// RGBA 像素（供 tiny-skia），长度 width * height * 4。
    pub rgba: Vec<u8>,
}

/// 按 u32 批量交换 R/B 通道（BGRA ↔ RGBA），比逐字节快数倍。
///
/// 同时把 alpha 强制为 255：`BitBlt` 到 32bpp DIB 时不保证写 alpha 通道，
/// 而下游 `Pixmap::from_vec`（预乘语义）与 PNG 导出都按「不透明」处理。
/// 保留宿主写进来的 0 会让整张截图变透明。
fn swap_rb(buf: &mut [u8]) {
    let n = buf.len() / 4;
    unsafe {
        let p = buf.as_mut_ptr() as *mut u32;
        for i in 0..n {
            let v = *p.add(i);
            *p.add(i) = (v & 0xFF00_FF00) | ((v & 0x00FF_0000) >> 16) | ((v & 0x0000_00FF) << 16) | 0xFF00_0000;
        }
    }
}

/// 大缓冲区多线程交换 R/B（目标地址用 usize 传递，裸指针不满足 Send）。
fn swap_rb_parallel(buf: &mut [u8]) {
    let n = buf.len() / 4;
    let workers = if buf.len() >= 1 << 20 {
        std::thread::available_parallelism()
            .map(|v| v.get())
            .unwrap_or(4)
            .clamp(1, 8)
    } else {
        1
    };
    if workers <= 1 {
        swap_rb(buf);
        return;
    }
    let addr = buf.as_mut_ptr() as usize;
    let chunk = n.div_ceil(workers);
    std::thread::scope(|s| {
        for start in (0..n).step_by(chunk.max(1)) {
            let count = (n - start).min(chunk);
            s.spawn(move || {
                let p = addr as *mut u32;
                for i in start..start + count {
                    let v = unsafe { *p.add(i) };
                    unsafe {
                        *p.add(i) = (v & 0xFF00_FF00) | ((v & 0x00FF_0000) >> 16) | ((v & 0x0000_00FF) << 16) | 0xFF00_0000;
                    }
                }
            });
        }
    });
}

/// 抓取屏幕指定区域（长截图逐帧采集视口用，LNG-2/3）。
/// 屏幕坐标为物理像素；区域越界部分自动裁切（BitBlt 源在屏幕外返回黑像素）。
///
/// HDR 模式下走 DDA + 色调映射（否则长截图同样会过曝），纯 SDR 仍用 BitBlt。
pub fn capture_region(x: i32, y: i32, w: u32, h: u32) -> Result<CapturedScreen, String> {
    if w == 0 || h == 0 {
        return Err("区域尺寸为 0，无法抓屏".into());
    }
    if let Some(result) = hdr::capture_region(x, y, w, h) {
        return result;
    }
    capture_region_bitblt(x, y, w, h)
}

/// GDI `BitBlt` 抓取指定区域。
fn capture_region_bitblt(x: i32, y: i32, w: u32, h: u32) -> Result<CapturedScreen, String> {
    unsafe {
        let hdc_screen = GetDC(None);
        if hdc_screen.0.is_null() {
            return Err("GetDC 失败".into());
        }
        let hdc_mem = CreateCompatibleDC(None);
        if hdc_mem.0.is_null() {
            let _ = ReleaseDC(None, hdc_screen);
            return Err("CreateCompatibleDC 失败".into());
        }

        let bmi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: w as i32,
                biHeight: -(h as i32),
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            bmiColors: [Default::default()],
        };
        let mut bits: *mut core::ffi::c_void = null_mut();
        let hbmp = match CreateDIBSection(None, &bmi, DIB_RGB_COLORS, &mut bits, None, 0) {
            Ok(hbmp) => hbmp,
            Err(e) => {
                let _ = DeleteDC(hdc_mem);
                let _ = ReleaseDC(None, hdc_screen);
                return Err(format!("CreateDIBSection 失败：{e}"));
            }
        };

        let _ = SelectObject(hdc_mem, hbmp.into());
        // 长截图不需要 CAPTUREBLT：抓取普通窗口内容即可（分层窗口内容由目标页面自绘）。
        if BitBlt(hdc_mem, 0, 0, w as i32, h as i32, Some(hdc_screen), x, y, SRCCOPY).is_err() {
            let _ = DeleteObject(hbmp.into());
            let _ = DeleteDC(hdc_mem);
            let _ = ReleaseDC(None, hdc_screen);
            return Err("BitBlt 失败".into());
        }

        let len = (w * h * 4) as usize;
        let mut rgba = vec![0u8; len];
        std::ptr::copy_nonoverlapping(bits as *const u8, rgba.as_mut_ptr(), len);
        swap_rb_parallel(&mut rgba);

        let _ = DeleteObject(hbmp.into());
        let _ = DeleteDC(hdc_mem);
        let _ = ReleaseDC(None, hdc_screen);

        Ok(CapturedScreen {
            width: w,
            height: h,
            origin_x: x,
            origin_y: y,
            rgba,
        })
    }
}

/// 抓取整个虚拟屏幕（一次，冻结画面）。
///
/// 有显示器处于 HDR 模式时走 DDA + 色调映射，否则走 GDI `BitBlt`。
pub fn capture_virtual_screen() -> Result<CapturedScreen, String> {
    // HDR 路径返回 None 表示无需介入（纯 SDR 环境），继续走下面的 BitBlt。
    if let Some(result) = hdr::capture_virtual_screen() {
        let mut capture = result?;
        // 混合 HDR/SDR 多显示器：DDA 只能抓 HDR 输出，同一块画布上的 SDR 显示器
        // 仍是全 0（截图上就是整屏黑）。这里用 BitBlt 把没覆盖到的显示器补上。
        fill_uncovered_monitors(&mut capture.screen, &capture.covered);
        return Ok(capture.screen);
    }
    capture_virtual_screen_bitblt()
}

/// 枚举当前所有显示器的矩形（虚拟屏物理坐标）。
fn monitor_rects() -> Vec<(i32, i32, i32, i32)> {
    use std::mem::size_of;
    use windows::Win32::Foundation::{LPARAM, RECT};
    use windows::core::BOOL;
    use windows::Win32::Graphics::Gdi::{
        EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR, MONITORINFO,
    };

    unsafe extern "system" fn collect(
        monitor: HMONITOR,
        _dc: HDC,
        _rect: *mut RECT,
        data: LPARAM,
    ) -> BOOL {
        // SAFETY: `data` 由下面的调用点传入，指向仍然存活的 Vec。
        let out = unsafe { &mut *(data.0 as *mut Vec<(i32, i32, i32, i32)>) };
        let mut info = MONITORINFO {
            cbSize: size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        if unsafe { GetMonitorInfoW(monitor, &mut info) }.as_bool() {
            let rect = info.rcMonitor;
            out.push((
                rect.left,
                rect.top,
                rect.right - rect.left,
                rect.bottom - rect.top,
            ));
        }
        BOOL(1)
    }

    let mut rects: Vec<(i32, i32, i32, i32)> = Vec::new();
    // SAFETY: 回调在调用期间同步执行，`rects` 的地址在整次枚举中有效。
    unsafe {
        let _ = EnumDisplayMonitors(
            None,
            None,
            Some(collect),
            LPARAM(&mut rects as *mut _ as isize),
        );
    }
    rects
}

/// 把 `covered` 之外的显示器用 GDI 补进画布。HDR 内容在这些屏上不存在，
/// 它们本来就是 SDR，BitBlt 拿到的是正确颜色。
fn fill_uncovered_monitors(screen: &mut CapturedScreen, covered: &[(i32, i32, i32, i32)]) {
    for (x, y, width, height) in monitor_rects() {
        if width <= 0 || height <= 0 {
            continue;
        }
        let overlaps = covered.iter().any(|&(cx, cy, cw, ch)| {
            x < cx + cw && cx < x + width && y < cy + ch && cy < y + height
        });
        if overlaps {
            continue;
        }
        let Ok(part) = capture_region_bitblt(x, y, width as u32, height as u32) else {
            continue;
        };
        blit_region_into(screen, &part);
    }
}

/// 把一小块抓屏结果拷进整屏画布（两者共用虚拟屏坐标系）。
fn blit_region_into(screen: &mut CapturedScreen, part: &CapturedScreen) {
    let dx = part.origin_x - screen.origin_x;
    let dy = part.origin_y - screen.origin_y;
    let (sw, sh) = (screen.width as i32, screen.height as i32);
    let (pw, ph) = (part.width as i32, part.height as i32);
    let x0 = dx.max(0);
    let y0 = dy.max(0);
    let x1 = (dx + pw).min(sw);
    let y1 = (dy + ph).min(sh);
    if x1 <= x0 || y1 <= y0 {
        return; // 完全在画布外
    }
    let len = ((x1 - x0) as usize) * 4;
    for y in y0..y1 {
        let src = (((y - dy) * pw + (x0 - dx)) as usize) * 4;
        let dst = ((y * sw + x0) as usize) * 4;
        screen.rgba[dst..dst + len].copy_from_slice(&part.rgba[src..src + len]);
    }
}

/// GDI `BitBlt` 抓取整个虚拟屏幕（纯 SDR 环境的快路径）。
fn capture_virtual_screen_bitblt() -> Result<CapturedScreen, String> {
    let t0 = std::time::Instant::now();
    unsafe {
        let vx = GetSystemMetrics(SM_XVIRTUALSCREEN);
        let vy = GetSystemMetrics(SM_YVIRTUALSCREEN);
        let vw = GetSystemMetrics(SM_CXVIRTUALSCREEN) as u32;
        let vh = GetSystemMetrics(SM_CYVIRTUALSCREEN) as u32;
        if vw == 0 || vh == 0 {
            return Err("虚拟屏幕尺寸为 0，无法抓屏".into());
        }

        let hdc_screen = GetDC(None);
        if hdc_screen.0.is_null() {
            return Err("GetDC 失败".into());
        }
        let hdc_mem = CreateCompatibleDC(None);
        if hdc_mem.0.is_null() {
            let _ = ReleaseDC(None, hdc_screen);
            return Err("CreateCompatibleDC 失败".into());
        }

        let bmi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: vw as i32,
                biHeight: -(vh as i32), // 自上而下
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            bmiColors: [Default::default()],
        };
        let mut bits: *mut core::ffi::c_void = null_mut();
        let hbmp = match CreateDIBSection(None, &bmi, DIB_RGB_COLORS, &mut bits, None, 0) {
            Ok(hbmp) => hbmp,
            Err(e) => {
                let _ = DeleteDC(hdc_mem);
                let _ = ReleaseDC(None, hdc_screen);
                return Err(format!("CreateDIBSection 失败：{e}"));
            }
        };

        let _ = SelectObject(hdc_mem, hbmp.into());
        let timing = std::env::var_os("AIRSNIP_TIMING").is_some();
        let t_blt = std::time::Instant::now();
        if BitBlt(
            hdc_mem,
            0,
            0,
            vw as i32,
            vh as i32,
            Some(hdc_screen),
            vx,
            vy,
            SRCCOPY | CAPTUREBLT,
        )
        .is_err()
        {
            let _ = DeleteObject(hbmp.into());
            let _ = DeleteDC(hdc_mem);
            let _ = ReleaseDC(None, hdc_screen);
            return Err("BitBlt 失败".into());
        }
        let blt = t_blt.elapsed();

        let t_copy = std::time::Instant::now();
        let len = (vw * vh * 4) as usize;
        let mut rgba = vec![0u8; len];
        std::ptr::copy_nonoverlapping(bits as *const u8, rgba.as_mut_ptr(), len);
        // BGRA（DIB）→ RGBA：批量交换 R/B（截图 alpha 不透明，预乘与直通相同）
        swap_rb_parallel(&mut rgba);
        let copy = t_copy.elapsed();
        if timing {
            eprintln!("[airsnip] BitBlt {blt:?}，拷贝+交换 {copy:?}");
        }

        let _ = DeleteObject(hbmp.into());
        let _ = DeleteDC(hdc_mem);
        let _ = ReleaseDC(None, hdc_screen);

        let result = Ok(CapturedScreen {
            width: vw,
            height: vh,
            origin_x: vx,
            origin_y: vy,
            rgba,
        });
        if std::env::var_os("AIRSNIP_TIMING").is_some() {
            eprintln!("[airsnip] 抓屏耗时 {:?}", t0.elapsed());
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 测试二进制没有 manifest，默认不是 Per-Monitor V2 DPI aware；
    /// HDR 桌面的 `DuplicateOutput1` 会因此返回 `DXGI_ERROR_UNSUPPORTED`。
    /// 宿主程序（Tauri）由 tao 在启动时设置同样的上下文，这里补上以便测试真实路径。
    fn make_process_dpi_aware() {
        use windows::Win32::UI::HiDpi::{
            DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, SetProcessDpiAwarenessContext,
        };
        // SAFETY: 进程级设置；重复调用只会失败并返回错误，不影响后续逻辑。
        unsafe {
            let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        }
    }

    #[test]
    fn capture_screen_works() {
        make_process_dpi_aware();
        let screen = capture_virtual_screen().expect("抓屏失败");
        assert!(screen.width > 0 && screen.height > 0);
        assert_eq!(screen.rgba.len(), (screen.width * screen.height * 4) as usize);
    }

    #[test]
    fn swap_rb_swaps_channels() {
        let mut buf = vec![1u8, 2, 3, 4, 5, 6, 7, 8];
        swap_rb(&mut buf);
        // R/B 互换，alpha 统一抬到 255（BitBlt 不保证写 alpha）。
        assert_eq!(buf, vec![3u8, 2, 1, 255, 7, 6, 5, 255]);
    }

    /// 对比 SRCCOPY 与 SRCCOPY|CAPTUREBLT 的全屏抓取耗时，供权衡。
    #[test]
    fn bitblt_rop_timing() {
        use std::time::Instant;
        unsafe {
            let vx = GetSystemMetrics(SM_XVIRTUALSCREEN);
            let vy = GetSystemMetrics(SM_YVIRTUALSCREEN);
            let vw = GetSystemMetrics(SM_CXVIRTUALSCREEN);
            let vh = GetSystemMetrics(SM_CYVIRTUALSCREEN);
            let hdc_screen = GetDC(None);
            let hdc_mem = CreateCompatibleDC(None);
            let bmi = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: vw,
                    biHeight: -vh,
                    biPlanes: 1,
                    biBitCount: 32,
                    biCompression: BI_RGB.0,
                    ..Default::default()
                },
                bmiColors: [Default::default()],
            };
            let mut bits: *mut core::ffi::c_void = null_mut();
            let hbmp = CreateDIBSection(None, &bmi, DIB_RGB_COLORS, &mut bits, None, 0).unwrap();
            let _ = SelectObject(hdc_mem, hbmp.into());

            for (name, rop) in [("SRCCOPY", SRCCOPY), ("SRCCOPY|CAPTUREBLT", SRCCOPY | CAPTUREBLT)] {
                let t = Instant::now();
                for _ in 0..5 {
                    let _ = BitBlt(hdc_mem, 0, 0, vw, vh, Some(hdc_screen), vx, vy, rop);
                }
                println!("BitBlt {name}: {:?}/次", t.elapsed() / 5);
            }

            let _ = DeleteObject(hbmp.into());
            let _ = DeleteDC(hdc_mem);
            let _ = ReleaseDC(None, hdc_screen);
        }
    }
}
