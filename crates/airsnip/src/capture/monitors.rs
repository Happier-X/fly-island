// SPDX-License-Identifier: GPL-3.0-only
//! 多显示器布局与画布拼装（CAP-1 / CAP-5）：枚举显示器矩形与设备名、按显示器补抓、
//! 把单块显示器的抓图拼进虚拟屏画布。
//!
//! 混合 DPI、互不对齐、负坐标（副屏在左 / 在上）的坐标运算都集中在这里：
//! 一块屏该不该补抓，只看「它和已经填过的矩形有没有面积重叠」，共享边界不算。

use windows::Win32::Foundation::{LPARAM, RECT};
use windows::Win32::Graphics::Gdi::{
    EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR, MONITORINFO, MONITORINFOEXW,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GetSystemMetrics, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN, SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN,
};
use windows::core::BOOL;

use super::hdr_display::{active_display_states, state_for_device};
use super::{CapturedScreen, capture_region_bitblt};

/// 一块显示器：虚拟屏物理像素矩形与 GDI 设备名（如 `\\.\DISPLAY1`）。
#[derive(Clone, Debug)]
pub(super) struct MonitorRect {
    pub(super) device: String,
    pub(super) x: i32,
    pub(super) y: i32,
    pub(super) w: i32,
    pub(super) h: i32,
}

impl MonitorRect {
    /// 与另一块显示器是否有面积重叠（仅共享边界不算）。
    fn overlaps(&self, other: (i32, i32, i32, i32)) -> bool {
        let (ox, oy, ow, oh) = other;
        self.x < ox + ow && ox < self.x + self.w && self.y < oy + oh && oy < self.y + self.h
    }

    /// 诊断日志用的 `x,y w×h`。
    fn describe(&self) -> String {
        format!("{},{} {}×{}", self.x, self.y, self.w, self.h)
    }
}

/// 抓屏诊断日志开关（`AIRSNIP_DEBUG_CAPTURE=1`）：逐显示器打印 HDR 状态、抓屏路径与补抓结果。
pub(super) fn debug_enabled() -> bool {
    std::env::var_os("AIRSNIP_DEBUG_CAPTURE").is_some()
}

/// 枚举当前所有显示器的矩形与设备名（虚拟屏物理坐标）。
pub(super) fn monitor_rects() -> Vec<MonitorRect> {
    use std::mem::size_of;

    unsafe extern "system" fn collect(
        monitor: HMONITOR,
        _dc: HDC,
        _rect: *mut RECT,
        data: LPARAM,
    ) -> BOOL {
        // SAFETY: `data` 由下面的调用点传入，指向仍然存活的 Vec。
        let out = unsafe { &mut *(data.0 as *mut Vec<MonitorRect>) };
        let mut info = MONITORINFOEXW {
            monitorInfo: MONITORINFO {
                cbSize: size_of::<MONITORINFOEXW>() as u32,
                ..Default::default()
            },
            ..Default::default()
        };
        if unsafe { GetMonitorInfoW(monitor, &mut info.monitorInfo) }.as_bool() {
            let rect = info.monitorInfo.rcMonitor;
            let name = info.szDevice;
            let len = name.iter().position(|c| *c == 0).unwrap_or(name.len());
            out.push(MonitorRect {
                device: String::from_utf16_lossy(&name[..len]),
                x: rect.left,
                y: rect.top,
                w: rect.right - rect.left,
                h: rect.bottom - rect.top,
            });
        }
        BOOL(1)
    }

    let mut rects: Vec<MonitorRect> = Vec::new();
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
pub(super) fn fill_uncovered_monitors(screen: &mut CapturedScreen, covered: &[(i32, i32, i32, i32)]) {
    let debug = debug_enabled();
    for m in monitor_rects() {
        let (x, y, width, height) = (m.x, m.y, m.w, m.h);
        if width <= 0 || height <= 0 {
            continue;
        }
        if covered.iter().any(|&c| m.overlaps(c)) {
            if debug {
                eprintln!(
                    "[airsnip] 补抓跳过 {} {}：已由 HDR 路径填入",
                    m.device,
                    m.describe()
                );
            }
            continue;
        }
        let Ok(part) = capture_region_bitblt(x, y, width as u32, height as u32) else {
            if debug {
                eprintln!("[airsnip] 补抓失败 {} {}", m.device, m.describe());
            }
            continue;
        };
        if debug {
            eprintln!("[airsnip] 补抓 {} {}：BitBlt 完成", m.device, m.describe());
        }
        blit_region_into(screen, &part);
    }
}

/// 诊断日志：每块显示器的矩形、设备名与 HDR 状态，以及虚拟屏边界。
pub(super) fn log_monitor_layout() {
    let vx = unsafe { GetSystemMetrics(SM_XVIRTUALSCREEN) };
    let vy = unsafe { GetSystemMetrics(SM_YVIRTUALSCREEN) };
    let vw = unsafe { GetSystemMetrics(SM_CXVIRTUALSCREEN) };
    let vh = unsafe { GetSystemMetrics(SM_CYVIRTUALSCREEN) };
    eprintln!("[airsnip] 虚拟屏 {vx},{vy} {vw}×{vh}");
    let states = active_display_states();
    for m in monitor_rects() {
        let state = state_for_device(&states, &m.device);
        eprintln!(
            "[airsnip] 显示器 {} {}：DisplayConfig 报 HDR={}",
            m.device,
            m.describe(),
            state.hdr_enabled
        );
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

#[cfg(test)]
mod tests {
    use super::*;

    fn monitor(x: i32, y: i32, w: i32, h: i32) -> MonitorRect {
        MonitorRect {
            device: "\\\\.\\DISPLAY1".into(),
            x,
            y,
            w,
            h,
        }
    }

    fn screen(x: i32, y: i32, w: u32, h: u32, value: u8) -> CapturedScreen {
        CapturedScreen {
            width: w,
            height: h,
            origin_x: x,
            origin_y: y,
            rgba: vec![value; (w * h * 4) as usize],
        }
    }

    /// 主屏 125%、副屏在右且顶部错开 41px（本机实测的双屏布局）。
    /// 副屏不能被当成「已被 HDR 路径覆盖」，否则它不会走 BitBlt 补抓，截图上就是整块黑。
    #[test]
    fn neighbours_sharing_an_edge_do_not_overlap() {
        let laptop = monitor(2560, 41, 2240, 1400);
        assert!(
            !laptop.overlaps((0, 0, 2560, 1440)),
            "只共享 x=2560 边界，不算重叠"
        );
        assert!(!monitor(0, 0, 2560, 1440).overlaps((2560, 41, 2240, 1400)));
        // 错开 1px 就是真重叠
        assert!(monitor(2559, 41, 2240, 1400).overlaps((0, 0, 2560, 1440)));
    }

    /// 副屏在左 / 在上时坐标为负，重叠判定不能因为负值而反掉。
    #[test]
    fn negative_layout_overlap_is_symmetric() {
        let left = monitor(-1920, 0, 1920, 1080);
        assert!(!left.overlaps((0, 0, 2560, 1440)));
        assert!(monitor(-100, 0, 1920, 1080).overlaps((0, 0, 2560, 1440)));

        let above = monitor(0, -1080, 1920, 1080);
        assert!(!above.overlaps((0, 0, 2560, 1440)));
        assert!(monitor(0, -100, 1920, 1080).overlaps((0, 0, 2560, 1440)));
    }

    /// 覆盖矩形比显示器大 / 小都算命中（HDR 输出矩形与 GDI 显示器矩形不一定完全相等）。
    #[test]
    fn covering_rect_of_any_size_counts() {
        let m = monitor(0, 0, 2560, 1440);
        assert!(m.overlaps((0, 0, 2560, 1440)));
        assert!(m.overlaps((-10, -10, 2600, 1460)), "更大的覆盖矩形");
        assert!(m.overlaps((100, 100, 10, 10)), "完全包含在内");
        assert!(!m.overlaps((2560, 0, 10, 10)), "隔着边界往右");
    }

    /// 画布原点为负（副屏在左 / 在上）时，单块显示器的抓图要落在正确的偏移上。
    #[test]
    fn blit_region_into_handles_negative_canvas_origin() {
        let mut canvas = screen(-1920, -200, 3840, 1280, 0);
        let at = |c: &CapturedScreen, x: usize, y: usize| c.rgba[(y * 3840 + x) * 4];

        // 副屏在左：显示器原点与画布原点重合
        blit_region_into(&mut canvas, &screen(-1920, -200, 1920, 1080, 7));
        assert_eq!(at(&canvas, 0, 0), 7);
        assert_eq!(at(&canvas, 1919, 1079), 7);
        assert_eq!(at(&canvas, 1920, 0), 0, "主屏区域不动");
        assert_eq!(at(&canvas, 0, 1080), 0, "副屏下方空白（顶部错开）保持原样");

        // 主屏：原点在画布 (1920, 200)
        blit_region_into(&mut canvas, &screen(0, 0, 1920, 1080, 9));
        assert_eq!(at(&canvas, 1920, 200), 9);
        assert_eq!(at(&canvas, 3839, 1279), 9);
        assert_eq!(at(&canvas, 1919, 200), 7, "左边缘仍是副屏");
        assert_eq!(at(&canvas, 1920, 199), 0, "主屏上方错开的 200px 不动");
    }
}
