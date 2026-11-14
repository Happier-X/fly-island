// SPDX-License-Identifier: GPL-3.0-only
//! 界面图标（EDT-7/PIN-1/M4/OUT-1/OUT-2）：把 `assets/icons/ui/` 的 SVG 解析成矢量路径，
//! 按按钮矩形归一化绘制。图标集规格、来源与视觉重量见 `assets/icons/ui/README.md`。
//!
//! 绘制时按**包围盒归一化**：同一批图标即使原始 viewBox 尺寸不同，也会被缩放到统一的
//! 活动区并居中，避免出现「有的图标看着大、有的看着小」。

mod data;
mod svg;

pub use data::Icon;

use std::sync::OnceLock;

use tiny_skia::{Paint, Pixmap, Transform};

/// 26px 原生按钮中使用 16px 的 Lucide 坐标框，与岛上 16px/1.5px 描边一致。
const LIVE: f32 = 16.0 / 26.0;

/// 已解析的图标几何，按 `Icon` 顺序缓存：首次绘制时解析一次，之后零开销。
fn glyphs() -> &'static [svg::SvgIcon] {
    static CACHE: OnceLock<Vec<svg::SvgIcon>> = OnceLock::new();
    CACHE.get_or_init(|| Icon::ALL.iter().map(|i| svg::parse(i.source())).collect())
}

/// 把图标绘制到像素图：内容居中于 `(cx, cy)` 的正方形活动区，边长 `size`（已含 DPI 缩放）。
pub fn draw(pixmap: &mut Pixmap, icon: Icon, cx: f32, cy: f32, size: f32, color: [u8; 4]) {
    let g = &glyphs()[icon as usize];
    debug_assert!(g.bounds.iter().all(|v| v.is_finite()));
    // 保留 Lucide 的原始 24×24 坐标；按墨水包围盒归一化会把短横线
    // 放大成粗杠，也会令相邻图标的绝对描边不一致。
    let k = size * LIVE / 24.0;
    let (bx, by) = (12.0, 12.0);
    // 先按 k 缩放，再平移到目标中心
    let transform = Transform::from_translate(cx - bx * k, cy - by * k).pre_scale(k, k);
    let mut paint = Paint::default();
    paint.set_color_rgba8(color[0], color[1], color[2], color[3]);
    for (path, rule) in &g.paths {
        pixmap.fill_path(path, &paint, *rule, transform, None);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 渲染到正方形画布，返回（墨水覆盖率, 是否触碰画布边缘）。
    fn render(icon: Icon, side: u32) -> (f32, bool) {
        let mut pm = Pixmap::new(side, side).expect("画布创建失败");
        let c = side as f32 / 2.0;
        draw(&mut pm, icon, c, c, side as f32, [255, 255, 255, 255]);
        let (mut ink, mut touches) = (0u32, false);
        for y in 0..side {
            for x in 0..side {
                if pm.pixel(x, y).expect("像素越界").alpha() == 0 {
                    continue;
                }
                ink += 1;
                if x == 0 || y == 0 || x == side - 1 || y == side - 1 {
                    touches = true;
                }
            }
        }
        (ink as f32 / (side * side) as f32, touches)
    }

    /// 每个图标都要能解析出至少一条路径（资源缺失或格式退化会在这里暴露）。
    #[test]
    fn every_icon_parses() {
        for &icon in Icon::ALL {
            assert!(!glyphs()[icon as usize].paths.is_empty(), "{icon:?} 没有解析出路径");
        }
    }

    /// 图标必须完整落在按钮活动区内，不能被裁掉或顶到边缘。
    #[test]
    fn icons_fit_inside_button() {
        for &icon in Icon::ALL {
            let (_, touches) = render(icon, 26);
            assert!(!touches, "{icon:?} 顶到按钮边缘（光学留白被吃掉）");
        }
    }

    /// 视觉重量守门：墨水覆盖率过低会看不清，过高会盖过邻居。
    /// 实测区间约 9%（减号）～46%（Phosphor 带背景框的 text）——Phosphor fill 系列
    /// 天然比早期手绘图标实心，故上限放到 48%。
    #[test]
    fn icon_weight_is_balanced() {
        for &icon in Icon::ALL {
            let (ink, _) = render(icon, 26);
            assert!(
                (0.015..=0.30).contains(&ink),
                "{icon:?} 墨水覆盖率 {ink:.3} 超出 Lucide 线框范围 1.5%～30%"
            );
        }
    }

    /// 同一图标在不同尺寸下都居中（活动区按边长等比缩放）。
    #[test]
    fn icons_stay_centered_at_any_size() {
        for &icon in Icon::ALL {
            let (_, touches) = render(icon, 48);
            assert!(!touches, "{icon:?} 在 48px 下顶到边缘");
        }
    }
}
