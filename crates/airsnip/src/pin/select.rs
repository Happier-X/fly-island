// SPDX-License-Identifier: GPL-3.0-only
//! 贴图框选取字（PIN-OCR）：右键「识别文字…」或按住 Alt 在贴图上拖动，框一块区域，
//! 松手后把该区域交给主线程的 OCR 气泡（复用 `app::bubble`，不另起窗口）。
//!
//! 坐标分三层：窗口客户区 → 内容显示区（去掉边框扩展）→ 原图像素。
//! 选区状态只存内容显示坐标；所有缩放 / 边框换算集中在 [`display_rect_to_source`]，
//! 纯函数便于单测（缩放、非 1.0 缩放、越界、零面积）。

use tiny_skia::Pixmap;
use windows::Win32::UI::Input::KeyboardAndMouse::{ReleaseCapture, SetCapture};
use windows::Win32::UI::WindowsAndMessaging::{
    IDC_ARROW, IDC_CROSS, LoadCursorW, SetCursor,
};
use windows::core::PCWSTR;

use super::window::PinWindow;

/// 选区带颜色（窗口像素；窗口是 premultiplied BGRA，叠加见 `PinWindow::paint_band`）。
pub(super) const BAND_COLOR: [u8; 3] = [0, 120, 215];
/// 选区内部半透明填充的 alpha（0-255）。
pub(super) const BAND_FILL_ALPHA: u16 = 56;
/// 选区边框的 alpha：边框要压得住填充，否则在亮底上发虚。
pub(super) const BAND_EDGE_ALPHA: u16 = 220;

/// 框选态（内容显示坐标，内容左上角为原点）。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct Select {
    pub anchor: (i32, i32),
    pub current: (i32, i32),
    pub dragging: bool,
}

impl Select {
    /// 归一化矩形 `(x, y, w, h)`；还没拖动过时宽高为 0。
    pub(super) fn rect(&self) -> (i32, i32, i32, i32) {
        let x = self.anchor.0.min(self.current.0);
        let y = self.anchor.1.min(self.current.1);
        (
            x,
            y,
            (self.anchor.0 - self.current.0).abs(),
            (self.anchor.1 - self.current.1).abs(),
        )
    }
}

/// 贴图内容在窗口客户区里的位置与尺寸（边框扩展会往里缩）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct ContentView {
    /// 边框在窗口内的显示像素偏移（四边相同）。
    pub offset: i32,
    pub width: i32,
    pub height: i32,
}

impl ContentView {
    /// `scaled` 为窗口显示像素尺寸，`pad` 为原始图里的边框扩展像素，`scale` 为当前缩放。
    pub(super) fn new(scaled: (u32, u32), pad: u32, scale: f32) -> Self {
        let offset = (pad as f32 * scale).round() as i32;
        Self {
            offset,
            width: (scaled.0 as i32 - offset * 2).max(1),
            height: (scaled.1 as i32 - offset * 2).max(1),
        }
    }
}

/// 窗口客户区坐标 → 内容显示坐标，并钳进内容范围（右 / 下边界可取到 `width` / `height`，
/// 表示“拖到边缘”，这样框住整张图时宽高正好等于内容尺寸）。
pub(super) fn client_to_content(view: ContentView, p: (i32, i32)) -> (i32, i32) {
    (
        (p.0 - view.offset).clamp(0, view.width),
        (p.1 - view.offset).clamp(0, view.height),
    )
}

/// 内容显示坐标 → 窗口客户区坐标（选区带绘制用）。
fn band_in_client(view: ContentView, rect: (i32, i32, i32, i32)) -> (i32, i32, i32, i32) {
    (rect.0 + view.offset, rect.1 + view.offset, rect.2, rect.3)
}

/// 两个矩形的并集（任一为空则返回另一个）。
fn union(a: (i32, i32, i32, i32), b: (i32, i32, i32, i32)) -> (i32, i32, i32, i32) {
    if a.2 <= 0 || a.3 <= 0 {
        return b;
    }
    if b.2 <= 0 || b.3 <= 0 {
        return a;
    }
    let x0 = a.0.min(b.0);
    let y0 = a.1.min(b.1);
    let x1 = a.0.saturating_add(a.2).max(b.0.saturating_add(b.2));
    let y1 = a.1.saturating_add(a.3).max(b.1.saturating_add(b.3));
    (x0, y0, x1 - x0, y1 - y0)
}

/// 选区（内容显示坐标）→ 原图像素矩形 `(x, y, w, h)`。
///
/// 显示尺寸与原图尺寸可能不等（滚轮缩放），并且显示区的宽高经过取整，
/// 所以按“内容宽高之比”换算而不是直接乘 `scale`，避免边框附近一轮一轮的累积误差。
/// 越界与负值先归一化、再钳进原图；零面积返回 `None`。
pub(super) fn display_rect_to_source(
    rect: (i32, i32, i32, i32),
    display: (u32, u32),
    source: (u32, u32),
) -> Option<(u32, u32, u32, u32)> {
    if display.0 == 0 || display.1 == 0 || source.0 == 0 || source.1 == 0 {
        return None;
    }
    let (dw, dh) = (display.0 as i32, display.1 as i32);
    let x0 = rect.0.clamp(0, dw);
    let x1 = rect.0.saturating_add(rect.2).clamp(0, dw);
    let y0 = rect.1.clamp(0, dh);
    let y1 = rect.1.saturating_add(rect.3).clamp(0, dh);
    let (x0, x1) = (x0.min(x1), x0.max(x1));
    let (y0, y1) = (y0.min(y1), y0.max(y1));
    if x1 <= x0 || y1 <= y0 {
        return None;
    }
    let map = |v: i32, disp: u32, src: u32| -> u32 {
        // 四舍五入到最近像素：拖到右边界时正好落在 `src`，不会少一列
        ((v as u64 * src as u64 + disp as u64 / 2) / disp as u64) as u32
    };
    let (sx0, sx1) = (map(x0, display.0, source.0), map(x1, display.0, source.0));
    let (sy0, sy1) = (map(y0, display.1, source.1), map(y1, display.1, source.1));
    if sx1 <= sx0 || sy1 <= sy0 {
        return None;
    }
    Some((sx0, sy0, sx1.min(source.0) - sx0, sy1.min(source.1) - sy0))
}

/// 从原图内容（premultiplied）裁出指定矩形，并还原成 straight（OCR 引擎按不透明处理）。
pub(super) fn crop_source(src: &Pixmap, r: (u32, u32, u32, u32)) -> Option<Pixmap> {
    let (x, y, w, h) = r;
    if w == 0 || h == 0 || x.saturating_add(w) > src.width() || y.saturating_add(h) > src.height() {
        return None;
    }
    let mut out = Pixmap::new(w, h)?;
    for row in 0..h {
        let so = ((y + row) as usize * src.width() as usize + x as usize) * 4;
        let dst = &mut out.data_mut()[row as usize * w as usize * 4..(row as usize + 1) * w as usize * 4];
        dst.copy_from_slice(&src.data()[so..so + w as usize * 4]);
    }
    // premultiplied → straight：不还原的话半透明像素会发暗（截图通常不透明，但贴图可以有透明区域）
    for px in out.data_mut().chunks_exact_mut(4) {
        let a = px[3] as u16;
        if a == 0 {
            px[0] = 0;
            px[1] = 0;
            px[2] = 0;
        } else {
            px[0] = (px[0] as u16 * 255 / a) as u8;
            px[1] = (px[1] as u16 * 255 / a) as u8;
            px[2] = (px[2] as u16 * 255 / a) as u8;
        }
    }
    Some(out)
}

impl PinWindow {
    /// 进入框选态（右键菜单 / Alt+按下）。
    pub(super) fn begin_select(&mut self) {
        // 滚轮缩放的最近邻预览帧先精修：框选时看到的就是要识别的像素
        self.finalize_preview();
        self.select = Some(Select::default());
        set_cursor(IDC_CROSS);
    }

    /// 按下开始拉框（窗口客户区坐标）。
    pub(super) fn select_down(&mut self, client: (i32, i32)) {
        if self.select.is_none() {
            self.begin_select();
        }
        let view = self.content_view();
        let p = client_to_content(view, client);
        if let Some(s) = self.select.as_mut() {
            s.anchor = p;
            s.current = p;
            s.dragging = true;
        }
        // SAFETY: 在贴图窗口的鼠标按下处理里调用，hwnd 有效；捕获在松手 / 取消时释放。
        let _ = unsafe { SetCapture(self.hwnd()) };
        set_cursor(IDC_CROSS);
    }

    /// 拖动中移动选区：只重绘新旧选区的并集。
    pub(super) fn select_move(&mut self, client: (i32, i32)) {
        let Some(sel) = self.select else { return };
        if !sel.dragging {
            return;
        }
        let view = self.content_view();
        let old = band_in_client(view, sel.rect());
        let p = client_to_content(view, client);
        if let Some(s) = self.select.as_mut() {
            s.current = p;
        }
        let Some(sel) = self.select else { return };
        let new = band_in_client(view, sel.rect());
        if old == new {
            return;
        }
        self.paint_band(union(old, new), Some(new));
    }

    /// Esc / 右键 / 丢失鼠标捕获：撤掉选区带并退出框选态（不弹气泡、不留痕）。
    pub(super) fn cancel_select(&mut self) {
        let Some(sel) = self.select.take() else { return };
        let view = self.content_view();
        let band = band_in_client(view, sel.rect());
        if sel.dragging {
            // SAFETY: 只释放本窗口自己取得的鼠标捕获；未捕获时调用无害。
            let _ = unsafe { ReleaseCapture() };
        }
        if band.2 > 0 && band.3 > 0 {
            self.paint_band(band, None);
        }
        set_cursor(IDC_ARROW);
    }

    /// 松手：退出框选态，返回（选区原图像素, 选区屏幕位置）；零面积返回 `None`。
    pub(super) fn finish_select(&mut self) -> Option<(Pixmap, (i32, i32))> {
        let sel = self.select.take()?;
        // SAFETY: 同 `cancel_select`。
        let _ = unsafe { ReleaseCapture() };
        let view = self.content_view();
        let band = band_in_client(view, sel.rect());
        if band.2 > 0 && band.3 > 0 {
            self.paint_band(band, None);
        }
        set_cursor(IDC_ARROW);
        if !sel.dragging {
            return None;
        }
        let display = sel.rect();
        let source = display_rect_to_source(
            display,
            (view.width as u32, view.height as u32),
            (self.raw.width(), self.raw.height()),
        )?;
        let pixmap = crop_source(&self.raw, source)?;
        let (cx, cy, _, _) = self.content_screen_rect();
        // 气泡按载荷的「选区矩形」定位（下方优先），而载荷宽高是原图像素：
        // 贴图被放大时原图比屏幕上小，把锚点下移，让矩形底边仍落在选区底边。
        let at = (
            cx + display.0,
            cy + display.1 + display.3 - pixmap.height() as i32,
        );
        Some((pixmap, at))
    }

    fn content_view(&self) -> ContentView {
        ContentView::new(
            (self.scaled.width(), self.scaled.height()),
            self.pad,
            self.scale(),
        )
    }
}

/// 把识别请求交给主线程（与覆盖层 OCR 同一个消息与载荷格式）：
/// `[u32 w][u32 h][i32 x][i32 y][RGBA]`，主线程弹气泡并后台识别。
pub(super) fn post_request(pixmap: &Pixmap, at: (i32, i32)) {
    crate::app::payload::post(crate::app::WM_OCR_REQUEST, pixmap, at, "识别文字");
}

fn set_cursor(id: PCWSTR) {
    // SAFETY: 只加载系统光标资源并设为当前光标，不触碰调用方内存。
    let cursor = unsafe { LoadCursorW(None, id) }.unwrap_or_default();
    let _ = unsafe { SetCursor(Some(cursor)) };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn content_view_removes_border_at_scale() {
        // 10×10 原图含 1px 边框 → 显示 10×10；0.9 缩放下边框四舍五入为 1
        let view = ContentView::new((9, 9), 1, 0.9);
        assert_eq!(view.offset, 1);
        assert_eq!((view.width, view.height), (7, 7));
        // 无边框：偏移为 0，内容就是整张显示图
        assert_eq!(
            ContentView::new((40, 30), 0, 2.0),
            ContentView {
                offset: 0,
                width: 40,
                height: 30
            }
        );
    }

    #[test]
    fn client_to_content_clamps_to_content() {
        let view = ContentView {
            offset: 2,
            width: 100,
            height: 60,
        };
        assert_eq!(client_to_content(view, (0, 0)), (0, 0));
        assert_eq!(client_to_content(view, (-40, -3)), (0, 0));
        assert_eq!(client_to_content(view, (2 + 30, 2 + 20)), (30, 20));
        // 拖到 / 越过右下边缘时取内容边界（整张图刚好等于内容尺寸）
        assert_eq!(client_to_content(view, (500, 500)), (100, 60));
    }

    #[test]
    fn source_rect_is_one_to_one_without_scaling() {
        assert_eq!(
            display_rect_to_source((10, 5, 20, 10), (100, 50), (100, 50)),
            Some((10, 5, 20, 10))
        );
    }

    #[test]
    fn source_rect_maps_non_unit_scales() {
        // 缩小 0.5：显示 50×25 对应原图 100×50
        assert_eq!(
            display_rect_to_source((10, 5, 20, 10), (50, 25), (100, 50)),
            Some((20, 10, 40, 20))
        );
        // 放大 2：显示 200×100 对应原图 100×50
        assert_eq!(
            display_rect_to_source((20, 10, 40, 20), (200, 100), (100, 50)),
            Some((10, 5, 20, 10))
        );
        // 拖到显示边界：映射到原图整宽 / 整高，不少一列
        assert_eq!(
            display_rect_to_source((0, 0, 50, 25), (50, 25), (100, 50)),
            Some((0, 0, 100, 50))
        );
    }

    #[test]
    fn source_rect_clips_out_of_bounds_and_negative() {
        // 负起点 + 越过右边界：钳进原图
        assert_eq!(
            display_rect_to_source((-10, -10, 130, 130), (100, 100), (100, 100)),
            Some((0, 0, 100, 100))
        );
        // 只有右下角一小块在范围内
        assert_eq!(
            display_rect_to_source((90, 90, 50, 50), (100, 100), (200, 200)),
            Some((180, 180, 20, 20))
        );
        // 负宽高按归一化矩形的相反方向处理（拖动中端点互换）
        assert_eq!(
            display_rect_to_source((40, 30, -20, -10), (100, 100), (100, 100)),
            Some((20, 20, 20, 10))
        );
    }

    #[test]
    fn source_rect_rejects_zero_area_and_empty_inputs() {
        assert_eq!(display_rect_to_source((10, 10, 0, 0), (100, 100), (100, 100)), None);
        assert_eq!(display_rect_to_source((10, 10, 0, 20), (100, 100), (100, 100)), None);
        // 只剩边界线：宽度取整后为 0，也不能当成 1px 去识别
        assert_eq!(display_rect_to_source((100, 10, 5, 20), (100, 100), (100, 100)), None);
        assert_eq!(display_rect_to_source((0, 0, 10, 10), (0, 0), (100, 100)), None);
    }

    #[test]
    fn select_rect_normalizes_drag_direction() {
        let sel = Select {
            anchor: (30, 40),
            current: (10, 15),
            dragging: true,
        };
        assert_eq!(sel.rect(), (10, 15, 20, 25));
    }

    #[test]
    fn crop_source_returns_straight_pixels() {
        let mut src = Pixmap::new(4, 4).unwrap();
        // premultiplied：(100,50,0,128) 的半透明橙
        src.fill(tiny_skia::Color::TRANSPARENT);
        for px in src.data_mut().chunks_exact_mut(4) {
            px.copy_from_slice(&[100, 50, 0, 128]);
        }
        let out = crop_source(&src, (1, 1, 2, 2)).unwrap();
        assert_eq!((out.width(), out.height()), (2, 2));
        let px = &out.data()[0..4];
        // 100*255/128 ≈ 199（straight），alpha 不变
        assert!(px[0] >= 199 && px[0] <= 200, "实际 {}", px[0]);
        assert_eq!(px[3], 128);
        // 越界直接拒绝，不读越界内存
        assert!(crop_source(&src, (3, 3, 2, 2)).is_none());
    }
}
