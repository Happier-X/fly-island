//! FocuSD Island on NSH: Phase 2 shell and clipboard pilot.
//!
//! The Win32 host (nsh/host) owns the top-level window and embeds this root
//! through AppView. The island keeps the capsule, adds a searchable
//! text/image clipboard with favorites, and a small settings page.

mod clipboard;
pub mod focus;
#[cfg(feature = "embedded")]
pub mod embedded;
mod preferences;
mod window;

use std::rc::Rc;

use nsh_core::{element::input, prelude::*, Color};

use clipboard::{ClipItem, ClipKind, ClipboardHistory};

const COLLAPSED_WIDTH: f32 = 320.0;
const COLLAPSED_HEIGHT: f32 = 40.0;
const EXPANDED_WIDTH: f32 = 620.0;
const EXPANDED_HEIGHT: f32 = 340.0;

#[derive(Clone, Copy)]
struct Palette {
    bg: Color,
    fg: Color,
    muted: Color,
    faint: Color,
    border: Color,
    row: Color,
    input: Color,
    accent: Color,
}

fn palette(dark: bool) -> Palette {
    if dark {
        Palette {
            bg: rgba(0x1B1D21F2).into(),
            fg: white().into(),
            muted: rgba(0xFFFFFF99).into(),
            faint: rgba(0xFFFFFF59).into(),
            border: rgba(0xFFFFFF1F).into(),
            row: rgba(0xFFFFFF14).into(),
            input: rgba(0xFFFFFF1A).into(),
            accent: rgb(0x6ABF9B).into(),
        }
    } else {
        Palette {
            bg: rgba(0xF7F7F5F2).into(),
            fg: rgb(0x1B1D21).into(),
            muted: rgba(0x1B1D2199).into(),
            faint: rgba(0x1B1D2159).into(),
            border: rgba(0x00000024).into(),
            row: rgba(0x0000000F).into(),
            input: rgba(0x0000000F).into(),
            accent: rgb(0x3A8F72).into(),
        }
    }
}

#[derive(Clone, Copy)]
struct SettingsState {
    dark: Signal<bool>,
    clipboard_enabled: Signal<bool>,
    capture_images: Signal<bool>,
    max_items: Signal<usize>,
}

#[native_module("focusd-island-nsh/native")]
fn native(registry: &mut Registry) {
    registry.assets(assets!());
    registry.register::<App>();
}

/// The island root: capsule, clipboard history and settings.
#[component]
pub fn App() -> impl IntoElement {
    let expanded = signal(false);
    let settings_open = signal(false);
    let query = signal(String::new());
    let saved = preferences::load();
    let dark = signal(saved.dark);
    let clipboard_enabled = signal(saved.clipboard_enabled);
    let capture_images = signal(saved.capture_images);
    let max_items = signal(saved.max_items.clamp(5, 200));
    let settings_state = SettingsState {
        dark,
        clipboard_enabled,
        capture_images,
        max_items,
    };
    if !saved.locale.is_empty() {
        if let Some(locale) = nsh_core::platform::Locale::parse(&saved.locale) {
            nsh_core::i18n::set_locale(Some(locale));
        }
    }
    let history = ClipboardHistory::start(clipboard_enabled, capture_images, max_items);
    let items = history.items;
    let visible = signal(Vec::<ClipItem>::new());
    effect(move || {
        let needle = query.get().to_lowercase();
        let list = items.get();
        let filtered: Vec<ClipItem> = list
            .into_iter()
            .filter(|item| {
                needle.is_empty()
                    || item.text.to_lowercase().contains(&needle)
                    || (item.kind == ClipKind::Image && "image 图片".contains(&needle))
            })
            .collect();
        visible.set(filtered);
    });


    window::configure(expanded);
    window::install_tray(expanded);
    window::install_shortcut(expanded);
    #[cfg(feature = "embedded")]
    embedded::start(expanded, settings_open);

    view()
        .flex_1()
        .items_center()
        .child(when(expanded, move || {
            expanded_island(expanded, settings_open, query, items, visible, dark)
        }))
        .child(when(move || !expanded.get(), move || {
            collapsed_island(expanded, items, dark)
        }))
        .child(settings_window(settings_open, settings_state, items))
}

fn collapsed_island(
    expanded: Signal<bool>,
    items: Signal<Vec<ClipItem>>,
    dark: Signal<bool>,
) -> impl IntoElement {
    view()
        .bg(move || palette(dark.get()).bg)
        .border(px(1.0))
        .border_color(move || palette(dark.get()).border)
        .rounded(px(18.0))
        .overflow_hidden()
        .flex_row()
        .items_center()
        .gap(px(10.0))
        .px(px(16.0))
        .h(px(COLLAPSED_HEIGHT))
        .w(px(COLLAPSED_WIDTH))
        .child(view().w(px(8.0)).h(px(40.0)).window_drag(true))
        .child(dot(dark))
        .child(
            text(move || t!("island.title"))
                .text_size(px(13.0))
                .font_weight(FontWeight::MEDIUM)
                .text_color(move || palette(dark.get()).fg),
        )
        .child(view().flex_1())
        .child(
            text(move || t!("island.count", count = items.get().len()))
                .text_size(px(12.0))
                .text_color(move || palette(dark.get()).muted),
        )
        .child(
            text("⌄")
                .text_size(px(16.0))
                .text_color(move || palette(dark.get()).muted),
        )
        .on_press(move || window::set_expanded(expanded, true))
}

fn expanded_island(
    expanded: Signal<bool>,
    settings_open: Signal<bool>,
    query: Signal<String>,
    items: Signal<Vec<ClipItem>>,
    visible: Signal<Vec<ClipItem>>,
    dark: Signal<bool>,
) -> impl IntoElement {
    view()
        .bg(move || palette(dark.get()).bg)
        .border(px(1.0))
        .border_color(move || palette(dark.get()).border)
        .rounded(px(18.0))
        .overflow_hidden()
        .w(px(EXPANDED_WIDTH))
        .h(px(EXPANDED_HEIGHT))
        .flex_col()
        .child(expanded_header(expanded, settings_open, items, dark))
        .child(divider(dark))
        .child(search_and_list(query, items, visible, dark))
        .child(
            view().px(px(18.0)).py(px(8.0)).child(
                text(move || t!("island.hint"))
                    .text_size(px(11.0))
                    .text_color(move || palette(dark.get()).faint),
            ),
        )
}

fn expanded_header(
    expanded: Signal<bool>,
    settings_open: Signal<bool>,
    items: Signal<Vec<ClipItem>>,
    dark: Signal<bool>,
) -> impl IntoElement {
    view()
        .flex_row()
        .items_center()
        .gap(px(10.0))
        .px(px(18.0))
        .h(px(44.0))
        .child(view().w(px(8.0)).h(px(40.0)).window_drag(true))
        .child(dot(dark))
        .child(
            text(move || t!("island.clipboard"))
            .text_size(px(14.0))
            .font_weight(FontWeight::MEDIUM)
            .text_color(move || palette(dark.get()).fg),
        )
        .child(
            text(move || t!("island.count", count = items.get().len()))
                .text_size(px(12.0))
                .text_color(move || palette(dark.get()).faint),
        )
        .child(view().flex_1())

        .child(
            text(move || if settings_open.get() { "✕" } else { "⚙" })
                .text_size(px(14.0))
                .text_color(move || palette(dark.get()).muted)
                .px(px(6.0))
                .on_press(move || settings_open.set(!settings_open.get_untracked())),
        )
        .child(
            text("⌃")
                .text_size(px(15.0))
                .text_color(move || palette(dark.get()).muted)
                .px(px(6.0))
                .on_press(move || window::set_expanded(expanded, false)),
        )
}

fn search_and_list(
    query: Signal<String>,
    items: Signal<Vec<ClipItem>>,
    visible: Signal<Vec<ClipItem>>,
    dark: Signal<bool>,
) -> impl IntoElement {
    view()
        .flex_1()
        .flex_col()
        .child(
            view().px(px(12.0)).pt(px(8.0)).pb(px(4.0)).child(
                input()
                    .value(move || query.get())
                    .placeholder(move || t!("island.search"))
                    .placeholder_color(move || palette(dark.get()).faint)
                    .text_color(move || palette(dark.get()).fg)
                    .bg(move || palette(dark.get()).input)
                    .h(px(30.0))
                    .px(px(10.0))
                    .rounded(px(8.0))
                    .on_focus(move || focus::set_input_focused(true))
                    .on_blur(move || focus::set_input_focused(false))
                    .on_change(move |text: &str| {
                        query.set(text.to_string());
                    }),
            ),
        )
        .child(
            scroll()
                .flex_1()
                .scrollbar(false)
                .px(px(10.0))
                .py(px(4.0))
                .child(when(
                    move || visible.get().is_empty() && !query.get().is_empty(),
                    move || no_results(dark),
                ))
                .child(when(
                    move || items.get().is_empty(),
                    move || empty_state(dark),
                ))
                .child(each(visible, |item| item.id, move |item, _| {
                    history_row(item, items, dark)
                })),
        )
}

fn history_row(
    item: ReadSignal<ClipItem>,
    items: Signal<Vec<ClipItem>>,
    dark: Signal<bool>,
) -> impl IntoElement {
    let snapshot = item.get();
    let copy = snapshot.clone();
    let id = snapshot.id;
    let content: AnyElement = if snapshot.kind == ClipKind::Image {
        let thumb: AnyElement = match snapshot.image_url.clone() {
            Some(url) => image(url)
                .w(px(120.0))
                .h(px(48.0))
                .rounded(px(6.0))
                .into_element(),
            None => text(move || t!("island.image_missing"))
                .text_color(move || palette(dark.get()).faint)
                .into_element(),
        };
        view()
            .flex_col()
            .gap(px(2.0))
            .child(thumb)
            .child(
                text(format!("{} × {}", snapshot.width, snapshot.height))
                    .text_size(px(10.0))
                    .text_color(move || palette(dark.get()).faint),
            )
            .into_element()
    } else {
        text(snapshot.text.chars().take(160).collect::<String>())
            .truncate()
            .text_size(px(12.5))
            .text_color(move || palette(dark.get()).fg)
            .into_element()
    };
    view()
        .flex_row()
        .items_center()
        .gap(px(10.0))
        .child(
            view()
                .flex_1()
                .flex_row()
                .items_center()
                .px(px(10.0))
                .py(px(8.0))
                .rounded(px(8.0))
                .active(|s| s.bg(palette(dark.get()).row))
                .on_press(move || match copy.kind {
                    ClipKind::Text => set_clipboard(copy.text.clone()),
                    ClipKind::Image => {
                        if let Some(url) = copy.image_url.clone() {
                            if !clipboard::write_image(&url) {
                                set_clipboard_image(url);
                            }
                        }
                    }
                })
                .child(content)
                .child(view().flex_1())
                .child(
                    text(move || t!("island.copy"))
                        .text_size(px(10.0))
                        .text_color(move || palette(dark.get()).faint),
                ),
        )
        .child(
            text(move || if item.get().favorite { "★" } else { "☆" })
                .text_size(px(15.0))
                .text_color(move || {
                    if item.get().favorite {
                        palette(dark.get()).accent
                    } else {
                        palette(dark.get()).faint
                    }
                })
                .px(px(8.0))
                .on_press(move || clipboard::toggle_favorite(items, id)),
        )
}
fn settings_window(
    open: Signal<bool>,
    state: SettingsState,
    items: Signal<Vec<ClipItem>>,
) -> impl IntoElement {
    nsh_core::element::desktop::window()
        .id("settings")
        .kind(nsh_core::desktop::WindowKind::Normal)
        .title(t!("island.settings"))
        .default_size(size(px(520.0), px(600.0)))
        .open(open)
        .on_open_change(move |event| open.set(event.open))
        .content(move || settings_content(open, state, items))
}

fn settings_content(
    open: Signal<bool>,
    state: SettingsState,
    items: Signal<Vec<ClipItem>>,
) -> impl IntoElement {
    view()
        .flex_1()
        .bg(move || palette(state.dark.get()).bg)
        .text_color(move || palette(state.dark.get()).fg)
        .p(px(20.0))
        .flex_col()
        .gap(px(14.0))
        .child(
            view()
                .flex_row()
                .items_center()
                .child(
                    text(move || t!("island.settings"))
                        .flex_1()
                        .text_size(px(18.0))
                        .font_weight(FontWeight::SEMIBOLD),
                )
                .child(
                    text("✕")
                        .text_size(px(16.0))
                        .text_color(move || palette(state.dark.get()).muted)
                        .on_press(move || open.set(false)),
                ),
        )
        .child(section_label(move || t!("island.theme"), state))
        .child(
            view()
                .flex_row()
                .gap(px(8.0))
                .child(pill(
                    move || t!("island.dark"),
                    Rc::new(move || state.dark.get()),
                    move || {
                        state.dark.set(true);
                        save_settings(state);
                    },
                    state.dark,
                ))
                .child(pill(
                    move || t!("island.light"),
                    Rc::new(move || !state.dark.get()),
                    move || {
                        state.dark.set(false);
                        save_settings(state);
                    },
                    state.dark,
                )),
        )
        .child(section_label(move || t!("island.language"), state))
        .child(
            view()
                .flex_row()
                .gap(px(8.0))
                .child(pill(
                    move || "中文".to_string(),
                    Rc::new(move || locale().as_str().starts_with("zh")),
                    move || {
                        if let Some(tag) = nsh_core::platform::Locale::parse("zh-CN") {
                            nsh_core::i18n::set_locale(Some(tag));
                        }
                        save_settings(state);
                    },
                    state.dark,
                ))
                .child(pill(
                    move || "English".to_string(),
                    Rc::new(move || !locale().as_str().starts_with("zh")),
                    move || {
                        if let Some(tag) = nsh_core::platform::Locale::parse("en") {
                            nsh_core::i18n::set_locale(Some(tag));
                        }
                        save_settings(state);
                    },
                    state.dark,
                )),
        )
        .child(section_label(move || t!("island.clipboard"), state))
        .child(
            view()
                .flex_row()
                .items_center()
                .gap(px(8.0))
                .child(
                    text(move || t!("island.clipboard_enabled"))
                        .flex_1()
                        .text_size(px(13.0))
                        .text_color(move || palette(state.dark.get()).muted),
                )
                .child(pill(
                    move || t!("island.on"),
                    Rc::new(move || state.clipboard_enabled.get()),
                    move || {
                        state.clipboard_enabled.set(true);
                        save_settings(state);
                    },
                    state.dark,
                ))
                .child(pill(
                    move || t!("island.off"),
                    Rc::new(move || !state.clipboard_enabled.get()),
                    move || {
                        state.clipboard_enabled.set(false);
                        save_settings(state);
                    },
                    state.dark,
                )),
        )
        .child(
            view()
                .flex_row()
                .items_center()
                .gap(px(8.0))
                .child(
                    text(move || t!("island.capture_images"))
                        .flex_1()
                        .text_size(px(13.0))
                        .text_color(move || palette(state.dark.get()).muted),
                )
                .child(pill(
                    move || t!("island.on"),
                    Rc::new(move || state.capture_images.get()),
                    move || {
                        state.capture_images.set(true);
                        save_settings(state);
                    },
                    state.dark,
                ))
                .child(pill(
                    move || t!("island.off"),
                    Rc::new(move || !state.capture_images.get()),
                    move || {
                        state.capture_images.set(false);
                        save_settings(state);
                    },
                    state.dark,
                )),
        )
        .child(
            view()
                .flex_row()
                .items_center()
                .gap(px(8.0))
                .child(
                    text(move || t!("island.max_items"))
                        .flex_1()
                        .text_size(px(13.0))
                        .text_color(move || palette(state.dark.get()).muted),
                )
                .child(max_pill(10, state, state.dark))
                .child(max_pill(20, state, state.dark))
                .child(max_pill(30, state, state.dark))
                .child(max_pill(50, state, state.dark)),
        )
        .child(
            view()
                .flex_row()
                .gap(px(8.0))
                .child(pill(
                    move || t!("island.clear"),
                    Rc::new(|| false),
                    move || clipboard::clear(items),
                    state.dark,
                )),
        )
        .child(view().flex_1())
        .child(
            text(move || t!("island.about", version = "0.1.0"))
                .text_size(px(11.0))
                .text_color(move || palette(state.dark.get()).faint),
        )
}

fn section_label(label: impl Fn() -> String + 'static, state: SettingsState) -> impl IntoElement {
    text(label)
        .text_size(px(12.0))
        .text_color(move || palette(state.dark.get()).muted)
        .mt(px(6.0))
}

fn max_pill(value: usize, state: SettingsState, dark: Signal<bool>) -> impl IntoElement {
    pill(
        move || value.to_string(),
        Rc::new(move || state.max_items.get() == value),
        move || {
            state.max_items.set(value);
            save_settings(state);
        },
        dark,
    )
}
fn pill(
    label: impl Fn() -> String + 'static,
    active: Rc<dyn Fn() -> bool + 'static>,
    press: impl Fn() + 'static,
    dark: Signal<bool>,
) -> impl IntoElement {
    let bg_active = active.clone();
    let fg_active = active;
    view()
        .px(px(12.0))
        .py(px(6.0))
        .rounded(px(8.0))
        .bg(move || {
            if bg_active() {
                palette(dark.get()).accent
            } else {
                palette(dark.get()).input
            }
        })
        .on_press(press)
        .child(
            text(label)
                .text_size(px(12.0))
                .text_color(move || {
                    if fg_active() {
                        palette(dark.get()).bg
                    } else {
                        palette(dark.get()).fg
                    }
                }),
        )
}

fn empty_state(dark: Signal<bool>) -> impl IntoElement {
    view().items_center().justify_center().py(px(36.0)).child(
        text(move || t!("island.empty"))
            .text_size(px(12.0))
            .text_color(move || palette(dark.get()).faint),
    )
}

fn no_results(dark: Signal<bool>) -> impl IntoElement {
    view().items_center().justify_center().py(px(36.0)).child(
        text(move || t!("island.no_results"))
            .text_size(px(12.0))
            .text_color(move || palette(dark.get()).faint),
    )
}

fn dot(dark: Signal<bool>) -> impl IntoElement {
    view()
        .w(px(7.0))
        .h(px(7.0))
        .rounded(px(4.0))
        .bg(move || palette(dark.get()).accent)
}

fn divider(dark: Signal<bool>) -> impl IntoElement {
    view()
        .h(px(1.0))
        .w_full()
        .bg(move || palette(dark.get()).border)
}

fn save_settings(state: SettingsState) {
    preferences::save(&preferences::Preferences {
        dark: state.dark.get_untracked(),
        locale: locale().as_str().to_string(),
        clipboard_enabled: state.clipboard_enabled.get_untracked(),
        capture_images: state.capture_images.get_untracked(),
        max_items: state.max_items.get_untracked(),
    });
}
