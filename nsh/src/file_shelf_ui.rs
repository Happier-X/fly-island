//! The island's file shelf page and its in-memory actions.


use nsh_core::{element::input, prelude::*};

use crate::{file_shelf, file_shelf_media, palette};

#[derive(Clone, Copy)]
pub struct ShelfState {
    pub items: Signal<Vec<file_shelf::Item>>,
    pub visible: Signal<Vec<file_shelf::Item>>,
    pub query: Signal<String>,
    pub error: Signal<String>,
    retry_path: Signal<Option<String>>,
    pub selected: Signal<Vec<String>>,
    anchor: Signal<Option<String>>,
    drag_start: Signal<Option<(String, f32, f32)>>,
    pub max_items: Signal<usize>,
    pub double_click: Signal<String>,
    pub dropping: Signal<bool>,
    confirm_clear: Signal<bool>,
}

pub fn start(max_items: usize, double_click: String) -> ShelfState {
    let max_items = max_items.clamp(file_shelf::MIN_MAX_ITEMS, file_shelf::MAX_MAX_ITEMS);
    let max_items_signal = signal(max_items);
    let double_click_signal = signal(double_click);
    let items = file_shelf::load(max_items, 0, true);
    let visible = items.clone();
    ShelfState {
        items: signal(items),
        visible: signal(visible),
        query: signal(String::new()),
        error: signal(String::new()),
        retry_path: signal(None),
        selected: signal(Vec::new()),
        anchor: signal(None),
        drag_start: signal(None),
        max_items: max_items_signal,
        double_click: double_click_signal,
        dropping: signal(false),
        confirm_clear: signal(false),
    }
}

pub fn refresh(state: ShelfState) {
    let items = file_shelf::load(state.max_items.get_untracked(), 0, true);
    sync_visible(state, items);
}

/// Adds paths from the host's WM_DROPFILES; only existing absolute paths survive.
#[cfg(feature = "embedded")]
pub fn add_dropped(state: ShelfState, paths: Vec<String>) {
    let next = file_shelf::add_paths(state.items.get_untracked(), &paths, state.max_items.get_untracked());
    store(state, next);
}

fn filter_items(items: &[file_shelf::Item], query: &str) -> Vec<file_shelf::Item> {
    let needle = file_shelf::path_key(query.trim());
    if needle.is_empty() {
        return items.to_vec();
    }
    items
        .iter()
        .filter(|item| {
            file_shelf::path_key(&item.name).contains(&needle)
                || file_shelf::path_key(&item.path).contains(&needle)
        })
        .cloned()
        .collect()
}

fn sync_visible(state: ShelfState, items: Vec<file_shelf::Item>) {
    state.visible.set(filter_items(&items, &state.query.get_untracked()));
    state.items.set(items);
}

fn store(state: ShelfState, mut items: Vec<file_shelf::Item>) {
    file_shelf::refresh_presence(&mut items);
    match file_shelf::save(&items) {
        Ok(()) => {
            sync_visible(state, items);
            state.error.set(String::new());
        }
        Err(error) => state.error.set(error),
    }
}

pub fn set_query(state: ShelfState, query: String) {
    state.query.set(query);
    state
        .visible
        .set(filter_items(&state.items.get_untracked(), &state.query.get_untracked()));
}

fn copy_path(state: ShelfState, item: &file_shelf::Item) {
    match file_shelf::copy_text(&item.path) {
        Ok(()) => {
            state.error.set(String::new());
            state.retry_path.set(None);
        }
        Err(error) => {
            state.error.set(error);
            state.retry_path.set(Some(item.path.clone()));
        }
    }
}

fn retry_copy(state: ShelfState) {
    let Some(path) = state.retry_path.get_untracked() else {
        return;
    };
    match file_shelf::copy_text(&path) {
        Ok(()) => {
            state.error.set(String::new());
            state.retry_path.set(None);
        }
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

#[cfg(feature = "embedded")]
pub fn drag_finished(state: ShelfState, result: Result<(), String>) {
    match result {
        Ok(()) => state.error.set(String::new()),
        Err(error) => state.error.set(error),
    }
}

fn start_drag(state: ShelfState, id: &str) {
    let selected = state.selected.get_untracked();
    let ids = if selected.iter().any(|selected| selected == id) {
        selected
    } else {
        vec![id.to_string()]
    };
    let paths: Vec<String> = state
        .items
        .get_untracked()
        .into_iter()
        .filter(|item| ids.contains(&item.id))
        .map(|item| item.path)
        .filter(|path| std::path::Path::new(path).exists())
        .collect();
    if paths.is_empty() {
        state.error.set(nsh_core::t!("island.shelf_unavailable").to_string());
        return;
    }
    if let Err(error) = crate::request_file_drag(paths) {
        state.error.set(error);
    }
}

fn remove_item(state: ShelfState, id: &str) {
    let mut items = state.items.get_untracked();
    file_shelf::remove(&mut items, id);
    store(state, items);
    state.selected.set(state.selected.get_untracked().into_iter().filter(|selected| selected != id).collect());
}

fn remove_selected(state: ShelfState) {
    let selected = state.selected.get_untracked();
    let mut items = state.items.get_untracked();
    items.retain(|item| !selected.contains(&item.id));
    store(state, items);
    state.selected.set(Vec::new());
    state.anchor.set(None);
}

fn toggle_pin(state: ShelfState, id: &str) {
    let mut items = state.items.get_untracked();
    file_shelf::toggle_pin(&mut items, id);
    store(state, items);
}

pub(crate) fn clear(state: ShelfState) {
    let mut items = state.items.get_untracked();
    file_shelf::clear(&mut items);
    store(state, items);
    state.selected.set(Vec::new());
    state.anchor.set(None);
    state.confirm_clear.set(false);
}

fn is_selected(state: ShelfState, id: &str) -> bool {
    state.selected.get().iter().any(|selected| selected == id)
}

fn next_selection(
    visible: &[String],
    current: Vec<String>,
    anchor: Option<&str>,
    id: &str,
    ctrl: bool,
    shift: bool,
) -> Vec<String> {
    if shift
        && let Some(anchor) = anchor
        && let (Some(from), Some(to)) = (visible.iter().position(|value| value == anchor), visible.iter().position(|value| value == id))
    {
        let (start, end) = if from <= to { (from, to) } else { (to, from) };
        return visible[start..=end].to_vec();
    }
    if ctrl {
        let mut next = current;
        if next.iter().any(|selected| selected == id) {
            next.retain(|selected| selected != id);
        } else {
            next.push(id.to_string());
            next.sort_by_key(|selected| visible.iter().position(|value| value == selected).unwrap_or(usize::MAX));
        }
        return next;
    }
    vec![id.to_string()]
}

fn selection_for(state: ShelfState, id: &str, event: &nsh_core::PressEvent) -> Vec<String> {
    let visible: Vec<String> = state.visible.get_untracked().into_iter().map(|item| item.id).collect();
    next_selection(
        &visible,
        state.selected.get_untracked(),
        state.anchor.get_untracked().as_deref(),
        id,
        event.modifiers.ctrl,
        event.modifiers.shift,
    )
}
fn press(state: ShelfState, item: file_shelf::Item, event: &nsh_core::PressEvent) {
    if event.clicks >= 2 {
        if state.double_click.get_untracked() == "reveal" {
            reveal_item(state, &item);
        } else {
            open_item(state, &item);
        }
        return;
    }
    let next = selection_for(state, &item.id, event);
    state.selected.set(next);
    if !event.modifiers.shift {
        state.anchor.set(Some(item.id));
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
        .child(search_bar(state, dark))
        .child(when(move || state.items.get().is_empty(), move || empty(state, dark)))
        .child(when(move || !state.items.get().is_empty() && state.visible.get().is_empty(), move || {
            text(move || nsh_core::t!("island.shelf_no_match"))
                .text_size(px(11.0))
                .text_color(move || palette(dark.get()).faint)
        }))
        .child(when(move || !state.visible.get().is_empty(), move || {
            scroll()
                .flex_1()
                .scrollbar(false)
                .child(
                    view()
                        .flex_row()
                        .flex_wrap()
                        .gap(px(6.0))
                        .child(each(state.visible, |item| item.id.clone(), move |item, _| {
                            item_card(state, item.get(), dark, tiles())
                        })),
                )
        }))
        .child(when(move || !state.error.get().is_empty(), move || {
            view()
                .flex_row()
                .items_center()
                .gap(px(6.0))
                .child(text(move || state.error.get()).flex_1().text_size(px(10.0)).text_color(move || palette(dark.get()).warning))
                .child(when(move || state.retry_path.get().is_some(), move || {
                    text(move || nsh_core::t!("island.shelf_retry"))
                        .text_size(px(10.0))
                        .text_color(move || palette(dark.get()).accent)
                        .on_press(move || retry_copy(state))
                }))
        }))
}

fn search_bar(state: ShelfState, dark: Signal<bool>) -> impl IntoElement {
    view()
        .flex_row()
        .items_center()
        .gap(px(4.0))
        .child(
            input()
                .flex_1()
                .h(px(26.0))
                .px(px(8.0))
                .rounded(px(7.0))
                .bg(move || palette(dark.get()).input)
                .text_size(px(11.0))
                .text_color(move || palette(dark.get()).fg)
                .placeholder(move || nsh_core::t!("island.shelf_search"))
                .placeholder_color(move || palette(dark.get()).faint)
                .on_change(move |text: &str| set_query(state, text.to_string())),
        )
        .child(when(move || !state.query.get().is_empty(), move || {
            text("✕")
                .text_size(px(11.0))
                .text_color(move || palette(dark.get()).muted)
                .px(px(4.0))
                .on_press(move || set_query(state, String::new()))
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
        .child(
            text(move || nsh_core::t!("island.shelf_add"))
                .text_size(px(10.0))
                .text_color(move || palette(dark.get()).muted)
                .px(px(4.0))
                .on_press(crate::request_file_pick),
        )
        .child(when(move || !state.selected.get().is_empty(), move || {
            text(move || nsh_core::t!("island.shelf_remove_selected", count = state.selected.get().len()))
                .text_size(px(10.0))
                .text_color(move || palette(dark.get()).warning)
                .px(px(4.0))
                .on_press(move || remove_selected(state))
        }))
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
    let preview = file_shelf_media::thumbnail(&item.path).or_else(|| file_shelf_media::icon(&item.path, item.is_dir, !bad));
    let drag_id = item.id.clone();
    let drag_id2 = drag_id.clone();
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
            .on_press(move |event: &nsh_core::PressEvent| press(state, press_item.clone(), event))
            .on_press_down(move |event: &nsh_core::PressEvent| {
                state.drag_start.set(Some((drag_id.clone(), event.position.x.0, event.position.y.0)));
            })
            .on_mouse_move(move |event: &nsh_core::MouseEvent| {
                if event.pressed_button.is_none() {
                    return;
                }
                let Some((id, x, y)) = state.drag_start.get_untracked() else {
                    return;
                };
                if id != drag_id2 {
                    return;
                }
                if (event.position.x.0 - x).hypot(event.position.y.0 - y) < 6.0 {
                    return;
                }
                state.drag_start.set(None);
                start_drag(state, &drag_id2);
            })
            .on_press_up(move |_event: &nsh_core::PressEvent| {
                state.drag_start.set(None);
            })
            .child(
                if let Some(url) = preview {
                    image(url).w(px(34.0)).h(px(34.0)).rounded(px(7.0)).into_element()
                } else {
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
                        )
                        .into_element()
                }
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
#[cfg(test)]
mod tests {
    use super::next_selection;

    #[test]
    fn ctrl_toggles_and_shift_selects_a_visible_range() {
        let visible = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        assert_eq!(next_selection(&visible, Vec::new(), None, "a", true, false), vec!["a"]);
        assert_eq!(next_selection(&visible, vec!["a".to_string()], Some("a"), "a", true, false), Vec::<String>::new());
        assert_eq!(next_selection(&visible, Vec::new(), Some("a"), "c", false, true), vec!["a", "b", "c"]);
    }
}
