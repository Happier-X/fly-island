// SPDX-License-Identifier: GPL-3.0-only
//! 像素辅助（CAP-6）：Alt 显示冻结原图的像素网格、屏幕坐标与颜色。
//! Alt+C 复制 HEX，Alt+Shift+C 复制 RGB，Alt+Ctrl+C 复制坐标；不改变截图内容。

use tiny_skia::{Pixmap, Transform};
use windows::Win32::Foundation::POINT;
use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;
use super::geometry::{SelRect, union_rect};
use super::shape::rounded_rect;
use crate::editor::Point;
use crate::render::text::{self, FontSpec};

#[derive(Clone, Copy)]
pub(super) struct Sample {
    pub rect: SelRect,
    point: (i32, i32),
}

impl super::Overlay {
    pub(super) fn update_sample_now(&mut self, active: bool) {
        let mut p = POINT::default();
        // SAFETY: 系统只写入已初始化的 POINT。
        if unsafe { GetCursorPos(&mut p) }.is_ok() {
            self.update_sample(p.x - self.capture.origin_x, p.y - self.capture.origin_y, active);
        }
    }

    pub(super) fn update_sample(&mut self, x: i32, y: i32, active: bool) {
        let old = self.sample.map(|s| s.rect);
        let (w, h) = (self.original.width() as i32, self.original.height() as i32);
        let point = (x.clamp(0, w - 1), y.clamp(0, h - 1));
        self.sample = (active && !self.locked_selection && !self.is_editing_text()).then(|| {
            let (pw, ph, gap) = (self.ui.px(174.0).min(w), self.ui.px(132.0).min(h), self.ui.px(18.0));
            let px = if x + gap + pw <= w { x + gap } else { x - gap - pw };
            let py = if y + gap + ph <= h { y + gap } else { y - gap - ph };
            Sample { point, rect: SelRect { x: px.clamp(0, w - pw), y: py.clamp(0, h - ph), w: pw, h: ph } }
        });
        if let Some(dirty) = union_rect(old, self.sample.map(|s| s.rect)) { self.repaint(dirty); }
    }

    pub(super) fn copy_sample(&self, coordinates: bool, rgb: bool) {
        let Some(sample) = self.sample else { return };
        let Some(color) = color_at(&self.original, sample.point) else { return };
        let value = if coordinates {
            format!("{}, {}", sample.point.0 + self.capture.origin_x, sample.point.1 + self.capture.origin_y)
        } else { format_color(color, rgb) };
        if let Err(e) = crate::output::copy_text_to_clipboard(&value) {
            crate::app::message_box(None, &format!("复制失败 / Copy failed: {e}"),
                windows::Win32::UI::WindowsAndMessaging::MB_OK | windows::Win32::UI::WindowsAndMessaging::MB_ICONERROR);
        }
    }

    pub(super) fn draw_sample(&mut self) {
        let Some(sample) = self.sample else { return };
        let Some(color) = color_at(&self.original, sample.point) else { return };
        let ui = self.ui;
        let p = self.palette;
        let r = sample.rect;
        let cell = ui.px(7.0);
        rounded_rect(&mut self.display, r, ui.f(8.0), p.bg, Some(p.border));
        let (left, top) = (r.x + (r.w - cell * 9) / 2, r.y + ui.px(7.0));
        for gy in 0..9 {
            for gx in 0..9 {
                let at = ((sample.point.0 + gx - 4).clamp(0, self.original.width() as i32 - 1),
                    (sample.point.1 + gy - 4).clamp(0, self.original.height() as i32 - 1));
                if let Some(c) = color_at(&self.original, at) {
                    rounded_rect(&mut self.display, SelRect { x: left + gx * cell, y: top + gy * cell, w: cell, h: cell },
                        0.0, [c[0], c[1], c[2], 255], Some(p.border));
                }
            }
        }
        // 中心像素用双重边框，在纯黑与纯白底图上都能辨认。
        let center = SelRect { x: left + 4 * cell, y: top + 4 * cell, w: cell, h: cell };
        super::shape::rounded_rect_outline(&mut self.display, center, 0.0, p.accent);
        let lines = [format_color(color, false),
            format!("{}, {}", sample.point.0 + self.capture.origin_x, sample.point.1 + self.capture.origin_y),
            "C: HEX · Shift+C: RGB".into()];
        let font = FontSpec::new(ui.f(11.0));
        for (i, line) in lines.iter().enumerate() {
            text::draw(&mut self.display, Point::new((r.x + ui.px(8.0)) as f32,
                (top + 9 * cell + ui.px(5.0) + i as i32 * ui.px(16.0)) as f32),
                line, &font, p.text, Transform::identity());
        }
    }
}

fn color_at(pm: &Pixmap, point: (i32, i32)) -> Option<[u8; 3]> {
    // 显式边界判断：`Pixmap::pixel` 对越界坐标的行为不值得依赖。
    if point.0 < 0 || point.1 < 0 || point.0 >= pm.width() as i32 || point.1 >= pm.height() as i32 {
        return None;
    }
    let p = pm.pixel(point.0 as u32, point.1 as u32)?.demultiply();
    Some([p.red(), p.green(), p.blue()])
}

fn format_color(color: [u8; 3], rgb: bool) -> String {
    let [r, g, b] = color;
    if rgb { format!("rgb({r}, {g}, {b})") } else { format!("#{r:02X}{g:02X}{b:02X}") }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sample_reads_original_pixel_and_rejects_edges() {
        let mut pm = Pixmap::new(2, 2).unwrap();
        pm.fill(tiny_skia::Color::from_rgba8(17, 34, 51, 255));
        assert_eq!(color_at(&pm, (1, 1)), Some([17, 34, 51]));
        assert_eq!(color_at(&pm, (-1, 0)), None);
        assert_eq!(color_at(&pm, (2, 0)), None);
    }
    #[test]
    fn color_formats_are_ready_to_paste() {
        assert_eq!(format_color([0, 15, 255], false), "#000FFF");
        assert_eq!(format_color([0, 15, 255], true), "rgb(0, 15, 255)");
    }
}
