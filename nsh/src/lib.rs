//! FocuSD Island on NSH: the Phase 1 feasibility spike.
//!
//! A top-centered, frameless, translucent capsule that expands into a
//! clipboard history panel; a tray icon and one global shortcut toggle it.
//! The text comes from `locales/` through NSH's i18n module.

mod clipboard;
mod window;

use nsh_core::prelude::*;

use clipboard::{ClipItem, ClipboardHistory};

const COLLAPSED_WIDTH: f32 = 320.0;
const COLLAPSED_HEIGHT: f32 = 40.0;
const EXPANDED_WIDTH: f32 = 620.0;
const EXPANDED_HEIGHT: f32 = 340.0;

/// The app's UI set (`nsh.json` `"rust"`): what a TSX entry may import, and
/// the assets its crate ships.
#[native_module("focusd-island-nsh/native")]
fn native(registry: &mut Registry) {
    registry.assets(assets!());
    registry.register::<App>();
}

/// The application's root component.
#[component]
pub fn App() -> impl IntoElement {
    let expanded = signal(false);
    let history = ClipboardHistory::start();
    window::configure(expanded);
    window::install_tray(expanded);
    window::install_shortcut(expanded);

    view()
        .flex_1()
        .items_center()
        .child(when(expanded, move || {
            expanded_island(expanded, history.items)
        }))
        .child(when(move || !expanded.get(), move || {
            collapsed_island(expanded, history.items)
        }))
}

fn collapsed_island(expanded: Signal<bool>, items: Signal<Vec<ClipItem>>) -> impl IntoElement {
    view()
        .bg(rgba(0x1B1D21F2))
        .border(px(1.0))
        .border_color(rgba(0xFFFFFF1F))
        .rounded(px(18.0))
        .overflow_hidden()
        .flex_row()
        .items_center()
        .gap(px(10.0))
        .px(px(16.0))
        .h(px(COLLAPSED_HEIGHT))
        .w(px(COLLAPSED_WIDTH))
        .child(dot())
        .child(
            text(move || t!("island.title"))
                .text_size(px(13.0))
                .font_weight(FontWeight::MEDIUM)
                .text_color(white()),
        )
        .child(view().flex_1())
        .child(
            text(move || t!("island.count", count = items.get().len()))
                .text_size(px(12.0))
                .text_color(rgba(0xFFFFFF99)),
        )
        .child(text("⌄").text_size(px(16.0)).text_color(rgba(0xFFFFFFB3)))
        .on_press(move || window::set_expanded(expanded, true))
}

fn expanded_island(expanded: Signal<bool>, items: Signal<Vec<ClipItem>>) -> impl IntoElement {
    view()
        .bg(rgba(0x1B1D21F2))
        .border(px(1.0))
        .border_color(rgba(0xFFFFFF1F))
        .rounded(px(18.0))
        .overflow_hidden()
        .w(px(EXPANDED_WIDTH))
        .h(px(EXPANDED_HEIGHT))
        .flex_col()
        .child(expanded_header(expanded, items))
        .child(divider())
        .child(
            scroll()
                .flex_1()
                .scrollbar(false)
                .px(px(10.0))
                .py(px(6.0))
                .child(when(move || items.get().is_empty(), empty_state))
                .child(each(items, |item| item.id, move |item, _| {
                    history_row(item.get())
                })),
        )
        .child(
            view().px(px(18.0)).py(px(8.0)).child(
                text(move || t!("island.hint"))
                    .text_size(px(11.0))
                    .text_color(rgba(0xFFFFFF66)),
            ),
        )
}

fn expanded_header(expanded: Signal<bool>, items: Signal<Vec<ClipItem>>) -> impl IntoElement {
    view()
        .flex_row()
        .items_center()
        .gap(px(10.0))
        .px(px(18.0))
        .h(px(44.0))
        .child(dot())
        .child(
            text(move || t!("island.clipboard"))
                .text_size(px(14.0))
                .font_weight(FontWeight::MEDIUM)
                .text_color(white()),
        )
        .child(
            text(move || t!("island.count", count = items.get().len()))
                .text_size(px(12.0))
                .text_color(rgba(0xFFFFFF80)),
        )
        .child(view().flex_1())
        .child(
            text(move || {
                if locale().as_str().starts_with("zh") {
                    "EN"
                } else {
                    "中文"
                }
            })
            .text_size(px(11.0))
            .text_color(rgba(0xFFFFFFB3))
            .px(px(8.0))
            .py(px(3.0))
            .rounded(px(8.0))
            .bg(rgba(0xFFFFFF1A))
            .on_press(move || {
                if let Some(tag) = nsh_core::platform::Locale::parse(next_locale()) {
                    nsh_core::i18n::set_locale(Some(tag));
                }
            }),
        )
        .child(
            text("⌃")
                .text_size(px(15.0))
                .text_color(rgba(0xFFFFFFB3))
                .px(px(6.0))
                .on_press(move || window::set_expanded(expanded, false)),
        )
}

fn history_row(item: ClipItem) -> impl IntoElement {
    let preview: String = item.text.chars().take(160).collect();
    let copy = item.text.clone();
    view()
        .flex_row()
        .items_center()
        .gap(px(10.0))
        .px(px(10.0))
        .py(px(8.0))
        .rounded(px(8.0))
        .active(|s| s.bg(rgba(0xFFFFFF14)))
        .on_press(move || set_clipboard(copy.clone()))
        .child(
            text(preview)
                .flex_1()
                .truncate()
                .text_size(px(12.5))
                .text_color(rgba(0xFFFFFFE6)),
        )
        .child(
            text(move || t!("island.copy"))
                .text_size(px(10.5))
                .text_color(rgba(0xFFFFFF59)),
        )
}

fn empty_state() -> impl IntoElement {
    view()
        .items_center()
        .justify_center()
        .py(px(36.0))
        .child(
            text(move || t!("island.empty"))
                .text_size(px(12.0))
                .text_color(rgba(0xFFFFFF59)),
        )
}

fn dot() -> impl IntoElement {
    view()
        .w(px(7.0))
        .h(px(7.0))
        .rounded(px(4.0))
        .bg(rgb(0x6ABF9B))
}

fn divider() -> impl IntoElement {
    view().h(px(1.0)).w_full().bg(rgba(0xFFFFFF12))
}

fn next_locale() -> &'static str {
    if locale().as_str().starts_with("zh") {
        "en"
    } else {
        "zh-CN"
    }
}
