//! The island's file shelf page and its in-memory actions.

use std::time::{Duration, Instant};

use nsh_core::prelude::*;

use crate::{file_shelf, palette};

#[derive(Clone, Copy)]
pub struct ShelfState {
    pub items: Signal<Vec<file_shelf::Item>>,
    pub error: Signal<String>,
    pub selected: Signal<Option<String>>,
    last_press: Signal<Option<(String, Instant)>>,
    pub dropping: Signal<bool>,
    confirm_clear: Signal<bool>,
}

pub fn start() -> ShelfState {
    ShelfState {
        items: signal(file_shelf::load(file_shelf::MAX_ITEMS, 0, true)),
        error: signal(String::new()),
        selected: signal(None),
        last_press: signal(None),
        dropping: signal(false),
        confirm_clear: signal(false),
    }
}

pub fn refresh(state: ShelfState) {
    state.items.set(file_shelf::load(file_shelf::MAX_ITEMS, 0, true));
}

/// Adds paths from the host's WM_DROPFILES; only existing absolute paths survive.
#[cfg(feature = "embedded")]
pub fn add_dropped(state: ShelfState, paths: Vec<String>) {
    let next = file_shelf::add_paths(state.items.get_untracked(), &paths, file_shelf::MAX_ITEMS);
    store(state, next);
}

fn store(state: ShelfState, mut items: Vec<file_shelf::Item>) {
    file_shelf::refresh_presence(&mut items);
    match file_shelf::save(&items) {
        Ok(()) => {
            state.items.set(items);
            state.error.set(String::new());
        }
        Err(error) => state.error.set(error),
    }
}


fn copy_path(state: ShelfState, item: &file_shelf::Item) {
    match file_shelf::copy_text(&item.path) {
        Ok(()) => state.error.set(String::new()),
        Err(error) => state.error.set(error),
    }
}

fn open_item(state: ShelfState, item: &file_shelf::Item) {
    if item.missing || item.unreachable {
        state.error.set(nsh_core::t!("island.shelf_unavailable").to_string());
        return;
    }
    if let Err(error) = file_shelf::open(item) {
        state.error.set(error);
    }
}

fn reveal_item(state: ShelfState, item: &file_shelf::Item) {
    if item.missing || item.unreachable {
        state.error.set(nsh_core::t!("island.shelf_unavailable").to_string());
        return;
    }
    if let Err(error) = file_shelf::reveal(item) {
        state.error.set(error);
    }
}

fn remove_item(state: ShelfState, id: &str) {
    let mut items = state.items.get_untracked();
    file_shelf::remove(&mut items, id);
    store(state, items);
    if state.selected.get_untracked().as_deref() == Some(id) {
        state.selected.set(None);
    }
}

fn toggle_pin(state: ShelfState, id: &str) {
    let mut items = state.items.get_untracked();
    file_shelf::toggle_pin(&mut items, id);
    store(state, items);
}

fn clear(state: ShelfState) {
    let mut items = state.items.get_untracked();
    file_shelf::clear(&mut items);
    store(state, items);
    state.selected.set(None);
    state.confirm_clear.set(false);
}

fn is_selected(state: ShelfState, id: &str) -> bool {
    state.selected.get().as_deref() == Some(id)
}

fn press(state: ShelfState, item: file_shelf::Item) {
    let now = Instant::now();
    let double = state
        .last_press
        .get_untracked()
        .is_some_and(|(id, at)| id == item.id && now.duration_since(at) < Duration::from_millis(450));
    if double {
        state.last_press.set(None);
        open_item(state, &item);
    } else {
        state.selected.set(Some(item.id.clone()));
        state.last_press.set(Some((item.id, now)));
    }
}

pub fn panel(state: ShelfState, dark: Signal<bool>) -> impl IntoElement {
    let tiles = move || state.items.get().len() <= 4;
    view()
        .flex_1()
        .min_h(px(0.0))
        .flex_col()
        .px(px(14.0))
        .py(px(10.0))
        .gap(px(8.0))
        .child(top_bar(state, dark))
        .child(when(move || state.items.get().is_empty(), move || empty(state, dark)))
        .child(when(move || !state.items.get().is_empty(), move || {
            scroll()
                .flex_1()
                .scrollbar(false)
                .child(
                    view()
                        .flex_row()
                        .flex_wrap()
                        .gap(px(6.0))
                        .child(each(state.items, |item| item.id.clone(), move |item, _| {
                            item_card(state, item.get(), dark, tiles())
                        })),
                )
        }))
        .child(when(move || !state.error.get().is_empty(), move || {
            text(move || state.error.get())
                .text_size(px(10.0))
                .text_color(move || palette(dark.get()).warning)
        }))
}

fn top_bar(state: ShelfState, dark: Signal<bool>) -> impl IntoElement {
    view()
        .flex_row()
        .items_center()
        .gap(px(8.0))
        .child(
            text(move || nsh_core::t!("island.shelf_title"))
                .text_size(px(12.0))
                .font_weight(FontWeight::MEDIUM)
                .text_color(move || palette(dark.get()).fg),
        )
        .child(
            text(move || nsh_core::t!("island.shelf_count", count = state.items.get().len()))
                .text_size(px(10.0))
                .text_color(move || palette(dark.get()).faint),
        )
        .child(view().flex_1())
        .child(when(move || !state.items.get().is_empty(), move || {
            text(move || {
                if state.confirm_clear.get() {
                    nsh_core::t!("island.shelf_confirm_clear")
                } else {
                    nsh_core::t!("island.shelf_clear")
                }
            })
            .text_size(px(10.0))
            .text_color(move || {
                if state.confirm_clear.get() {
                    palette(dark.get()).warning
                } else {
                    palette(dark.get()).muted
                }
            })
            .px(px(4.0))
            .on_press(move || {
                if state.confirm_clear.get_untracked() {
                    clear(state);
                } else {
                    state.confirm_clear.set(true);
                }
            })
        }))
}

fn empty(state: ShelfState, dark: Signal<bool>) -> impl IntoElement {
    view()
        .flex_1()
        .items_center()
        .justify_center()
        .gap(px(6.0))
        .rounded(px(10.0))
        .border(px(1.0))
        .border_color(move || {
            if state.dropping.get() {
                palette(dark.get()).accent
            } else {
                palette(dark.get()).border
            }
        })
        .child(
            text("⇣")
                .text_size(px(22.0))
                .text_color(move || palette(dark.get()).accent),
        )
        .child(
            text(move || {
                if state.dropping.get() {
                    nsh_core::t!("island.shelf_drop_hint")
                } else {
                    nsh_core::t!("island.shelf_empty")
                }
            })
            .text_size(px(12.0))
            .text_color(move || palette(dark.get()).muted),
        )
        .child(
            text(move || nsh_core::t!("island.shelf_empty_note"))
                .text_size(px(10.0))
                .text_color(move || palette(dark.get()).faint),
        )
}

fn item_card(
    state: ShelfState,
    item: file_shelf::Item,
    dark: Signal<bool>,
    tile: bool,
) -> impl IntoElement {
    let press_item = item.clone();
    let badge_text = file_shelf::badge(&item);
    let selected_id = item.id.clone();
    let selected_bg = selected_id.clone();
    let selected_border = selected_id;
    let meta = if item.missing {
        nsh_core::t!("island.shelf_missing").to_string()
    } else if item.unreachable {
        nsh_core::t!("island.shelf_unreachable").to_string()
    } else if item.is_dir {
        nsh_core::t!("island.shelf_folder").to_string()
    } else {
        file_shelf::format_size(item.size)
    };
    let is_dir = item.is_dir;
    let bad = item.missing || item.unreachable;
    let card = view()
        .w(if tile { px(0.0) } else { px(282.0) })
        .flex_grow(if tile { 1.0 } else { 0.0 })
        .min_w(if tile { px(150.0) } else { px(0.0) })
        .max_w(if tile { px(190.0) } else { px(290.0) })
        .flex_col()
        .gap(px(5.0))
        .p(px(9.0))
        .rounded(px(9.0))
        .bg(move || {
            if is_selected(state, &selected_bg) {
                palette(dark.get()).input
            } else {
                palette(dark.get()).row
            }
        })
        .border(px(1.0))
        .border_color(move || {
            if is_selected(state, &selected_border) {
                palette(dark.get()).accent
            } else {
                palette(dark.get()).border
            }
        })
        ;
    card.child(
        view()
            .flex_row()
            .items_center()
            .gap(px(7.0))
            .on_press(move || press(state, press_item.clone()))
            .child(
                view()
                    .w(px(34.0))
                    .h(px(34.0))
                    .items_center()
                    .justify_center()
                    .rounded(px(7.0))
                    .bg(move || palette(dark.get()).input)
                    .child(
                        text(badge_text)
                            .text_size(px(9.0))
                            .font_weight(FontWeight::BOLD)
                            .text_color(move || {
                                if is_dir {
                                    palette(dark.get()).warning
                                } else {
                                    palette(dark.get()).accent
                                }
                            }),
                    ),
            )
            .child(
                view()
                    .flex_1()
                    .min_w(px(0.0))
                    .flex_col()
                    .child(
                        text(item.name.clone())
                            .truncate()
                            .text_size(px(11.0))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(move || palette(dark.get()).fg),
                    )
                    .child(
                        text(item.path.clone())
                            .truncate()
                            .text_size(px(9.0))
                            .text_color(move || palette(dark.get()).faint),
                    ),
            )
    )
    .child(
        text(meta)
            .truncate()
            .text_size(px(9.0))
            .text_color(move || {
                if bad {
                    palette(dark.get()).warning
                } else {
                    palette(dark.get()).muted
                }
            }),
    )
    .child(action_row(state, item, dark))
}

fn action_row(state: ShelfState, item: file_shelf::Item, dark: Signal<bool>) -> impl IntoElement {
    let pin_id = item.id.clone();
    let pinned = item.pinned;
    let open_target = item.clone();
    let reveal_target = item.clone();
    let copy_target = item.clone();
    let remove_id = item.id.clone();
    view()
        .flex_row()
        .gap(px(8.0))
        .child(
            text(move || if pinned { "★" } else { "☆" })
                .text_size(px(11.0))
                .text_color(move || if pinned { palette(dark.get()).accent } else { palette(dark.get()).faint })
                .px(px(2.0))
                .on_press(move || toggle_pin(state, &pin_id)),
        )
        .child(
            text("↗")
                .text_size(px(11.0))
                .text_color(move || palette(dark.get()).muted)
                .px(px(2.0))
                .on_press(move || open_item(state, &open_target)),
        )
        .child(
            text("⌖")
                .text_size(px(11.0))
                .text_color(move || palette(dark.get()).muted)
                .px(px(2.0))
                .on_press(move || reveal_item(state, &reveal_target)),
        )
        .child(
            text("⧉")
                .text_size(px(11.0))
                .text_color(move || palette(dark.get()).muted)
                .px(px(2.0))
                .on_press(move || copy_path(state, &copy_target)),
        )
        .child(
            text("✕")
                .text_size(px(11.0))
                .text_color(move || palette(dark.get()).warning)
                .px(px(2.0))
                .on_press(move || remove_item(state, &remove_id)),
        )
}
