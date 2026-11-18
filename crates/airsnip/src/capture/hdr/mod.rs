// SPDX-License-Identifier: GPL-3.0-only
//! HDR 抓屏入口（CAP-1，issue #75）。
//!
//! 两个公开函数与 `capture` 模块的同名 GDI 实现一一对应：返回 `None` 表示
//! 「当前不是 HDR 环境、无需介入」，调用方继续走 `BitBlt` 快路径。
//!
//! 之所以用 `Option<Result<..>>` 而不是直接返回 `Result`，是为了让分流逻辑留在
//! `capture` 顶层：HDR 模块只负责它擅长的部分，SDR 行为完全不受影响。

mod frame;
mod output;
mod session;

pub use frame::GrabError;
pub use session::HdrRegionSession;

use super::CapturedScreen;
use super::hdr_display::active_display_states;
use frame::grab_frame;
use output::{all_hdr_outputs, blit_into, hdr_output_intersecting};

/// 一次 HDR 抓屏的结果：整块虚拟屏画布，以及被 HDR 路径真正填过的输出矩形。
///
/// 混合 HDR/SDR 多显示器时画布上非 HDR 的显示器仍是全 0，由调用方补抓。
pub struct HdrScreenCapture {
    pub screen: CapturedScreen,
    /// 已被填过的显示器矩形（虚拟屏物理坐标）。
    pub covered: Vec<(i32, i32, i32, i32)>,
}

/// 抓取整个虚拟屏幕。非 HDR 环境返回 `None`。
pub fn capture_virtual_screen() -> Option<Result<HdrScreenCapture, String>> {
    let states = active_display_states();
    let outputs = match all_hdr_outputs(&states) {
        Ok(v) => v,
        Err(e) => return Some(Err(e)),
    };
    if outputs.is_empty() {
        return None; // 纯 SDR，交给 BitBlt
    }

    // 与 GDI 版本保持一致：画布覆盖整个虚拟屏幕，负坐标布局平移到原点。
    let (vx, vy, vw, vh) = unsafe {
        use windows::Win32::UI::WindowsAndMessaging::{
            GetSystemMetrics, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN, SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN,
        };
        (
            GetSystemMetrics(SM_XVIRTUALSCREEN),
            GetSystemMetrics(SM_YVIRTUALSCREEN),
            GetSystemMetrics(SM_CXVIRTUALSCREEN) as u32,
            GetSystemMetrics(SM_CYVIRTUALSCREEN) as u32,
        )
    };
    if vw == 0 || vh == 0 {
        return Some(Err("虚拟屏幕尺寸为 0，无法抓屏".into()));
    }

    let mut canvas = vec![0u8; (vw as usize) * (vh as usize) * 4];
    let debug = super::monitors::debug_enabled();
    // 只有真正贴进画布的矩形才算「已覆盖」：没抓到的输出必须留给调用方用 BitBlt 补，
    // 否则那块屏在截图上就是整块黑。
    let mut covered: Vec<(i32, i32, i32, i32)> = Vec::with_capacity(outputs.len());
    for out in &outputs {
        let rect = (
            out.rect.left,
            out.rect.top,
            out.rect.right - out.rect.left,
            out.rect.bottom - out.rect.top,
        );
        if debug {
            eprintln!(
                "[airsnip] HDR 输出在 {},{} {}×{}，白点 {:.0}nits",
                rect.0, rect.1, rect.2, rect.3, out.color.sdr_white_nits
            );
        }
        let frame = match grab_frame(out) {
            Ok(f) => f,
            Err(e) => {
                // 单块屏抓不到不该让整次截图失败：跳过它，后面由 BitBlt 补上。
                eprintln!(
                    "[airsnip] HDR 抓帧失败（{},{} {}×{}）：{e}；改用 BitBlt 补抓",
                    rect.0, rect.1, rect.2, rect.3
                );
                continue;
            }
        };
        // 驱动在某些输出上（实测：开启自动颜色管理的广色域屏）会返回全黑帧，
        // 直接贴上去就是整屏黑。这时同样交给 BitBlt：SDR 内容上它才是对的。
        if is_blank_frame(&frame.rgba) {
            eprintln!(
                "[airsnip] HDR 帧全黑（{},{} {}×{}）；改用 BitBlt 补抓",
                rect.0, rect.1, rect.2, rect.3
            );
            continue;
        }
        // 把该输出的帧贴进虚拟屏画布（裁掉超出画布的部分）
        if let Err(e) = blit_into(&mut canvas, vw, vh, out.rect.left - vx, out.rect.top - vy, &frame) {
            return Some(Err(e));
        }
        covered.push(rect);
    }

    Some(Ok(HdrScreenCapture {
        covered,
        screen: CapturedScreen {
            width: vw,
            height: vh,
            origin_x: vx,
            origin_y: vy,
            rgba: canvas,
        },
    }))
}

/// 帧是否整帧全黑（RGB 全 0）。
///
/// 真正全黑的桌面不会被误判：这种事上 BitBlt 抓回来也是黑的，结果不变。
fn is_blank_frame(rgba: &[u8]) -> bool {
    rgba
        .as_chunks::<4>()
        .0
        .iter()
        .all(|px| px[0] == 0 && px[1] == 0 && px[2] == 0)
}

/// 抓取屏幕指定区域（长截图逐帧采集视口用，LNG-2/3）。非 HDR 环境返回 `None`。
///
/// DDA 只能整屏抓，这里抓完再按请求矩形裁剪；越界部分留黑（与 BitBlt 行为一致）。
pub fn capture_region(x: i32, y: i32, w: u32, h: u32) -> Option<Result<CapturedScreen, String>> {
    let states = active_display_states();
    let out = hdr_output_intersecting(&states, x, y, w, h)?;

    let result = grab_frame(&out);
    let frame = match result {
        Ok(f) => f,
        // 抓不到或拿到全黑帧时退回 `None`：调用方会走 BitBlt，而不是交付一块黑图。
        Err(e) => {
            eprintln!("[airsnip] HDR 区域抓帧失败（{x},{y} {w}×{h}）：{e}；改用 BitBlt");
            return None;
        }
    };
    if is_blank_frame(&frame.rgba) {
        eprintln!("[airsnip] HDR 区域帧全黑（{x},{y} {w}×{h}）；改用 BitBlt");
        return None;
    }
    // 帧的左上是 out.rect.left/top，换算成相对请求原点的偏移（可为负）
    let dx = out.rect.left - x;
    let dy = out.rect.top - y;
    let mut rgba = vec![0u8; (w as usize) * (h as usize) * 4];
    if let Err(e) = blit_into(&mut rgba, w, h, dx, dy, &frame) {
        return Some(Err(e));
    }
    Some(Ok(CapturedScreen {
        width: w,
        height: h,
        origin_x: x,
        origin_y: y,
        rgba,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 全黑帧判定：只有 RGB 全 0 才算，单通道非零就不算（避免把正常暗色画面退回 BitBlt）。
    #[test]
    fn blank_frame_needs_every_pixel_zero() {
        assert!(is_blank_frame(&[0u8; 16]));
        assert!(is_blank_frame(&[]), "空帧也算全黑");
        let mut one_pixel = vec![0u8; 16];
        one_pixel[5] = 1;
        assert!(!is_blank_frame(&one_pixel));
        // alpha 不参与判定：不透明黑仍是黑
        assert!(is_blank_frame(&[0, 0, 0, 255, 0, 0, 0, 255]));
    }

    /// 同一个 output 同时只能有一个 duplication 会话，并行跑会互相抢到 E_INVALIDARG / E_ACCESSDENIED。
    static CAPTURE_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    /// 设置 Per-Monitor V2（`DuplicateOutput1` 的硬性前提；测试进程没有 manifest）。
    fn make_per_monitor_v2() {
        unsafe {
            use windows::Win32::UI::HiDpi::{
                DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, SetProcessDpiAwarenessContext,
            };
            let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        }
    }

    /// 本机若处于 HDR 模式，抓屏应成功且与虚拟屏幕尺寸一致。
    ///
    /// 默认 `--ignored`：需要真实桌面会话，无头环境会失败。
    #[test]
    #[ignore]
    fn hdr_capture_works() {
        let _guard = CAPTURE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        make_per_monitor_v2();
        match capture_virtual_screen() {
            None => println!("当前非 HDR 环境，走 BitBlt 路径"),
            Some(Ok(capture)) => {
                let s = capture.screen;
                assert!(s.width > 0 && s.height > 0);
                assert_eq!(s.rgba.len(), (s.width * s.height * 4) as usize);
                assert!(s.rgba.chunks_exact(4).all(|p| p[3] == 255), "输出必须不透明");
            }
            Some(Err(e)) => panic!("HDR 抓屏失败：{e}"),
        }
    }

    /// 区域抓取应返回请求尺寸，且越界不 panic。
    #[test]
    #[ignore]
    fn hdr_region_capture_works() {
        let _guard = CAPTURE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        make_per_monitor_v2();
        let (x, y) = unsafe {
            use windows::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN};
            (GetSystemMetrics(SM_XVIRTUALSCREEN), GetSystemMetrics(SM_YVIRTUALSCREEN))
        };
        let (w, h) = unsafe {
            use windows::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN};
            (
                GetSystemMetrics(SM_CXVIRTUALSCREEN) as u32,
                GetSystemMetrics(SM_CYVIRTUALSCREEN) as u32,
            )
        };
        match capture_region(x, y, w, h) {
            None => println!("当前非 HDR 环境，长截图走 BitBlt"),
            Some(Ok(s)) => assert_eq!(s.rgba.len(), (w * h * 4) as usize),
            Some(Err(e)) => panic!("HDR 区域抓取失败：{e}"),
        }
    }

    /// 长截图的区域抓帧会话（issue #128）：建会话后应先拿到一帧真实画面。
    /// 复用会话在后续帧桌面静止时返回 None，由长截图流程覆盖。
    #[test]
    #[ignore]
    fn hdr_region_session_grabs_first_frame() {
        let _guard = CAPTURE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        make_per_monitor_v2();
        let (x, y) = unsafe {
            use windows::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN};
            (GetSystemMetrics(SM_XVIRTUALSCREEN), GetSystemMetrics(SM_YVIRTUALSCREEN))
        };
        let (w, h) = (400u32, 300u32);
        match HdrRegionSession::open(x, y, w, h) {
            None => println!("当前非 HDR 环境，长截图走 BitBlt"),
            Some(Ok(c)) => {
                let f = c.grab(true).expect("抓帧失败").expect("应拿到首帧");
                assert_eq!(f.len(), (w * h * 4) as usize);
            }
            Some(Err(e)) => panic!("建立 HDR 区域会话失败：{e}"),
        }
    }

    /// 临时耗时测量（红线：热键→可见 150ms / 4K 250ms）。
    #[test]
    #[ignore]
    fn hdr_capture_timing() {
        let _guard = CAPTURE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        make_per_monitor_v2();
        let _ = capture_virtual_screen(); // 预热（首次含驱动初始化）
        for i in 1..=5 {
            let t = std::time::Instant::now();
            let r = capture_virtual_screen();
            let ms = t.elapsed().as_secs_f64() * 1000.0;
            match r {
                Some(Ok(capture)) => println!(
                    "第 {i} 次：{ms:>7.1} ms  {}x{}",
                    capture.screen.width, capture.screen.height
                ),
                Some(Err(e)) => {
                    println!("失败：{e}");
                    return;
                }
                None => println!("非 HDR 环境（走 BitBlt），耗时 {ms:.1} ms"),
            }
        }
    }
}
