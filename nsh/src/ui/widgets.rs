//! The island's building blocks, each one a class the Tauri shell has: a round
//! icon button, a page dot, a hairline, a progress ring. Screens compose these
//! instead of writing their own paddings and colors.

use nsh_core::{Color, element::ImageSrc, prelude::*};

use super::theme;

/// An icon at `size` points, stroked in the surrounding text color.
pub fn icon(markup: &str, size: f32) -> Svg {
    svg(ImageSrc::svg_markup(markup))
        .w(px(size))
        .h(px(size))
        .flex_shrink(0.0)
}

/// A round icon button (`.icon-button`): 34px, a faint disc, secondary color,
/// brighter on hover. Callers attach `.on_press` and any color override.
pub fn icon_button(markup: &str, icon_size: f32, color: Color) -> View {
    view()
        .w(px(theme::ICON_BUTTON))
        .h(px(theme::ICON_BUTTON))
        .flex_shrink(0.0)
        .items_center()
        .justify_center()
        .rounded(px(theme::ICON_BUTTON / 2.0))
        .bg(theme::WHITE_A08)
        .text_color(color)
        .cursor_pointer()
        .transition()
        .duration(140.0)
        .hover(|s| s.bg(theme::WHITE_A16).text_color(white()))
        .active(|s| s.translate_y(px(1.0)))
        .child(icon(markup, icon_size))
}

/// A page dot (`.dot-button`): a 24x34 hit box around an 18x12 capsule in the
/// page's color; hovering lifts and widens it, the active page keeps it wider.
pub fn page_dot(color: Color, active: bool) -> View {
    let glow = theme::dot_glow(color);
    view()
        .w(px(theme::DOT_WIDTH))
        .h(px(theme::HEADER_HEIGHT))
        .flex_shrink(0.0)
        .items_center()
        .justify_center()
        .cursor_pointer()
        .group()
        .child(
            view()
                .w(px(theme::DOT_CAPSULE_WIDTH))
                .h(px(theme::DOT_CAPSULE_HEIGHT))
                .flex_shrink(0.0)
                .rounded(px(theme::DOT_CAPSULE_HEIGHT / 2.0))
                .bg(color)
                .shadow(glow)
                .transition()
                .duration(200.0)
                .style_when(active, |s| s.scale(1.1))
                .group_hover(|s| s.scale(1.4).translate_y(px(-3.0))),
        )
}

/// The 1px hairline between a page's rows and its footer.
pub fn hairline() -> View {
    view().h(px(1.0)).w_full().bg(theme::WHITE_A06)
}

/// A text pill (a filter or a count): 20px tall, fully rounded.
pub fn pill(label: impl Into<String>, color: Color, bg: Color) -> View {
    view()
        .px(px(theme::SPACE_MD))
        .h(px(20.0))
        .items_center()
        .justify_center()
        .rounded(px(10.0))
        .bg(bg)
        .child(text(label.into()).text_size(px(theme::HINT_SIZE)).font_weight(FontWeight::SEMIBOLD).text_color(color))
}

/// A progress ring: a full track circle with an arc for `value` (0–1), turned
/// so it starts at 12 o'clock, as `MiniRing` draws it.
pub fn ring(size: f32, stroke: f32, value: f32, color: Color, track: Color) -> Svg {
    let center = size / 2.0;
    let radius = (size - stroke) / 2.0;
    let circumference = 2.0 * std::f32::consts::PI * radius;
    let share = value.clamp(0.0, 1.0);
    let dash = circumference * share;
    let markup = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{size}\" height=\"{size}\" \
         viewBox=\"0 0 {size} {size}\" fill=\"none\">\
         <circle cx=\"{center}\" cy=\"{center}\" r=\"{radius}\" stroke=\"{track}\" \
         stroke-width=\"{stroke}\"/>\
         <circle cx=\"{center}\" cy=\"{center}\" r=\"{radius}\" stroke=\"{color}\" \
         stroke-width=\"{stroke}\" stroke-linecap=\"round\" \
         stroke-dasharray=\"{dash} {circumference}\" transform=\"rotate(-90 {center} {center})\"/>\
         </svg>",
        track = css_color(track),
        color = css_color(color),
    );
    svg(ImageSrc::svg_markup(&markup)).w(px(size)).h(px(size)).flex_shrink(0.0)
}

/// A color as the CSS text an SVG document needs, `#rrggbbaa`.
pub fn css_color(color: Color) -> String {
    let (r, g, b, a) = color.to_f64();
    format!(
        "#{:02x}{:02x}{:02x}{:02x}",
        (r * 255.0).round() as u8,
        (g * 255.0).round() as u8,
        (b * 255.0).round() as u8,
        (a * 255.0).round() as u8
    )
}

/// A round avatar-like chip holding an icon, for lists that lead with one.
pub fn icon_chip(markup: &str, size: f32, color: Color, bg: Color) -> View {
    view()
        .w(px(size))
        .h(px(size))
        .flex_shrink(0.0)
        .items_center()
        .justify_center()
        .rounded(px(size / 2.0))
        .bg(bg)
        .text_color(color)
        .child(icon(markup, size * 0.5))
}

/// A section title above a page's rows.
pub fn section_title(label: impl Fn() -> String + 'static) -> impl IntoElement {
    text(label)
        .text_size(px(theme::HINT_SIZE))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(theme::TEXT_3)
        .letter_spacing(px(0.4))
        .h(px(18.0))
}

/// An empty page's message, centered in the space it gets.
pub fn empty_message(label: impl Fn() -> String + 'static) -> impl IntoElement {
    view().flex_1().items_center().justify_center().child(
        text(label)
            .text_size(px(theme::LABEL_SIZE))
            .text_color(theme::TEXT_3)
            .text_center(),
    )
}

/// The small number badge the collapsed bar and the header share.
pub fn count_badge(label: impl Fn() -> String + 'static, color: Color, bg: Color) -> View {
    view()
        .h(px(15.0))
        .min_w(px(15.0))
        .px(px(theme::SPACE_XS))
        .items_center()
        .justify_center()
        .rounded(px(7.5))
        .bg(bg)
        .child(text(label).text_size(px(9.5)).font_weight(FontWeight::BOLD).text_color(color))
        .flex_shrink(0.0)
}
