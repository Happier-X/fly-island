//! Advanced screenshot settings: palette, filename template, UI scale, tool
//! switch keys and toolbar visibility. Each control maps 1:1 onto a field in
//! AirSnip's Config, so the settings page mirrors the Tauri version.

use std::rc::Rc;

use airsnip::settings::{ToolbarGroup, ToolbarSlot};
use nsh_core::{element::input, prelude::*, Color};

use crate::palette;
use crate::screenshot::{self, TOOL_KEY_FIELDS, TOOL_KEY_LABELS, UI_SCALES};
use crate::screenshot_ui::{persist, pill, ScreenshotSettingsState};

pub fn advanced_section(state: ScreenshotSettingsState, dark: Signal<bool>) -> impl IntoElement {
    view()
        .flex_col()
        .gap(px(5.0))
        .child(name_style_row(state, dark))
        .child(ui_scale_row(state, dark))
        .child(
            view()
                .flex_col()
                .gap(px(4.0))
                .child(row_label(move || t!("island.screenshot_colors"), dark))
                .child(
                    view()
                        .flex_row()
                        .flex_wrap()
                        .gap(px(6.0))
                        .children(
                            (0..5usize)
                                .map(|index| color_cell(state, index, dark).into_element())
                                .collect::<Vec<_>>(),
                        ),
                )
                .child(hint(move || t!("island.screenshot_colors_hint"), dark)),
        )
        .child(
            view()
                .flex_col()
                .gap(px(4.0))
                .child(row_label(move || t!("island.screenshot_tool_keys"), dark))
                .child(
                    view()
                        .flex_row()
                        .flex_wrap()
                        .gap(px(6.0))
                        .children(
                            tool_key_fields()
                                .into_iter()
                                .map(|(field, label)| {
                                    tool_key_cell(state, field, label, dark).into_element()
                                })
                                .collect::<Vec<_>>(),
                        ),
                )
                .child(hint(move || t!("island.screenshot_tool_keys_hint"), dark)),
        )
        .child(
            view()
                .flex_col()
                .gap(px(4.0))
                .child(row_label(move || t!("island.screenshot_toolbar"), dark))
                .child(toolbar_visibility(state, dark))
                .child(hint(move || t!("island.screenshot_toolbar_hint"), dark)),
        )
}

fn tool_key_fields() -> Vec<(&'static str, &'static str)> {
    TOOL_KEY_FIELDS
        .iter()
        .copied()
        .zip(TOOL_KEY_LABELS)
        .collect()
}

fn row_label(label: impl Fn() -> String + 'static, dark: Signal<bool>) -> impl IntoElement {
    text(label)
        .text_size(px(13.0))
        .text_color(move || palette(dark.get()).muted)
}

fn hint(label: impl Fn() -> String + 'static, dark: Signal<bool>) -> impl IntoElement {
    text(label)
        .text_size(px(11.0))
        .text_color(move || palette(dark.get()).faint)
}

fn name_style_row(state: ScreenshotSettingsState, dark: Signal<bool>) -> impl IntoElement {
    view()
        .flex_row()
        .flex_wrap()
        .items_center()
        .gap(px(5.0))
        .child(row_label(move || t!("island.screenshot_name_style"), dark))
        .child(pill(
            move || t!("island.screenshot_name_datetime"),
            Rc::new(move || state.name_style.get() == "datetime"),
            move || {
                state.name_style.set("datetime".to_string());
                persist(state);
            },
            dark,
        ))
        .child(pill(
            move || t!("island.screenshot_name_timestamp"),
            Rc::new(move || state.name_style.get() == "timestamp"),
            move || {
                state.name_style.set("timestamp".to_string());
                persist(state);
            },
            dark,
        ))
        .child(pill(
            move || t!("island.screenshot_name_date"),
            Rc::new(move || state.name_style.get() == "date"),
            move || {
                state.name_style.set("date".to_string());
                persist(state);
            },
            dark,
        ))
}

fn ui_scale_row(state: ScreenshotSettingsState, dark: Signal<bool>) -> impl IntoElement {
    view()
        .flex_row()
        .flex_wrap()
        .items_center()
        .gap(px(5.0))
        .child(row_label(move || t!("island.screenshot_ui_scale"), dark))
        .children(
            UI_SCALES
                .iter()
                .copied()
                .map(|value| {
                    pill(
                        move || {
                            if value == "auto" {
                                t!("island.screenshot_ui_auto")
                            } else {
                                format!("{value}%")
                            }
                        },
                        Rc::new(move || state.ui_scale.get() == value),
                        move || {
                            state.ui_scale.set(value.to_string());
                            persist(state);
                        },
                        dark,
                    )
                    .into_element()
                })
                .collect::<Vec<_>>(),
        )
}

fn color_cell(
    state: ScreenshotSettingsState,
    index: usize,
    dark: Signal<bool>,
) -> impl IntoElement {
    view()
        .flex_col()
        .gap(px(3.0))
        .child(
            view()
                .h(px(16.0))
                .w(px(66.0))
                .rounded(px(4.0))
                .bg(move || preview_color(state, index, dark)),
        )
        .child(
            input()
                .value(move || {
                    state
                        .colors
                        .get()
                        .get(index)
                        .cloned()
                        .unwrap_or_default()
                })
                .placeholder(move || "#RRGGBB")
                .placeholder_color(move || palette(dark.get()).faint)
                .text_color(move || palette(dark.get()).fg)
                .bg(move || palette(dark.get()).input)
                .h(px(26.0))
                .w(px(66.0))
                .px(px(6.0))
                .rounded(px(6.0))
                .on_change(move |text: &str| {
                    let mut colors = state.colors.get_untracked();
                    if colors.len() < 5 {
                        colors.resize(5, String::new());
                    }
                    colors[index] = text.trim().to_string();
                    state.colors.set(colors);
                    persist(state);
                }),
        )
}

fn preview_color(state: ScreenshotSettingsState, index: usize, dark: Signal<bool>) -> Color {
    let value = state
        .colors
        .get()
        .get(index)
        .and_then(|text| screenshot::parse_hex_color(text));
    match value {
        Some([r, g, b, _]) => rgb(u32::from(r) << 16 | u32::from(g) << 8 | u32::from(b)).into(),
        None => palette(dark.get()).input,
    }
}

fn tool_key_cell(
    state: ScreenshotSettingsState,
    field: &'static str,
    label: &'static str,
    dark: Signal<bool>,
) -> impl IntoElement {
    view()
        .flex_col()
        .gap(px(3.0))
        .child(
            text(move || label.to_string())
                .text_size(px(11.0))
                .text_color(move || palette(dark.get()).muted),
        )
        .child(
            input()
                .value(move || {
                    state
                        .tool_keys
                        .get()
                        .get(field)
                        .cloned()
                        .unwrap_or_default()
                })
                .placeholder(move || "1")
                .placeholder_color(move || palette(dark.get()).faint)
                .text_color(move || palette(dark.get()).fg)
                .bg(move || palette(dark.get()).input)
                .h(px(26.0))
                .w(px(52.0))
                .px(px(6.0))
                .rounded(px(6.0))
                .on_change(move |text: &str| {
                    let mut keys = state.tool_keys.get_untracked();
                    keys.insert(field.to_string(), text.trim().to_string());
                    state.tool_keys.set(keys);
                    persist(state);
                }),
        )
}

fn toolbar_visibility(state: ScreenshotSettingsState, dark: Signal<bool>) -> impl IntoElement {
    view().flex_col().gap(px(4.0)).children(
        ToolbarGroup::ALL
            .iter()
            .copied()
            .map(|group| group_row(state, group, dark).into_element())
            .collect::<Vec<_>>(),
    )
}

fn group_row(
    state: ScreenshotSettingsState,
    group: ToolbarGroup,
    dark: Signal<bool>,
) -> impl IntoElement {
    view()
        .flex_col()
        .gap(px(3.0))
        .child(
            text(move || group.title().to_string())
                .text_size(px(11.0))
                .text_color(move || palette(dark.get()).faint),
        )
        .child(
            view()
                .flex_row()
                .flex_wrap()
                .gap(px(4.0))
                .children(
                    group
                        .slots()
                        .iter()
                        .map(|slot| slot_toggle(state, *slot, dark).into_element())
                        .collect::<Vec<_>>(),
                ),
        )
}

fn slot_toggle(
    state: ScreenshotSettingsState,
    slot: ToolbarSlot,
    dark: Signal<bool>,
) -> impl IntoElement {
    let key = slot.key();
    let label = slot.label();
    pill(
        move || label.to_string(),
        Rc::new(move || {
            !state
                .hidden_toolbar
                .get()
                .iter()
                .any(|hidden| hidden == key)
        }),
        move || {
            let mut hidden = state.hidden_toolbar.get_untracked();
            if let Some(index) = hidden.iter().position(|item| item == key) {
                hidden.remove(index);
            } else {
                hidden.push(key.to_string());
            }
            state.hidden_toolbar.set(hidden);
            persist(state);
        },
        dark,
    )
}
