//! Quick launcher window, ranking list and its settings section.

use std::{rc::Rc, time::Duration};

use nsh_core::{
    desktop::{DisplayId, WindowAttributes, WindowFrameStyle, WindowKind, WindowLevel, WindowPosition},
    element::{input, scroll},
    prelude::*,
};

use crate::launcher::{self, hotkey::NavAction, LauncherResult, ResultKind, SearchEngine};
use crate::palette;
use crate::translation_ui::{is_modifier_key, shortcut_label};

#[derive(Clone, Copy)]
pub struct LauncherSettingsState {
    pub enabled: Signal<bool>,
    pub shortcut: Signal<String>,
    pub search_apps: Signal<bool>,
    pub search_files: Signal<bool>,
    pub search_web: Signal<bool>,
    pub engine: Signal<String>,
    pub folder_mode: Signal<String>,
    pub folders: Signal<Vec<String>>,
    pub shortcut_draft: Signal<String>,
    pub recording: Signal<bool>,
    pub folder_draft: Signal<String>,
    pub error: Signal<String>,
    pub dirty: Signal<u64>,
}

#[derive(Clone, Copy)]
pub struct LauncherState {
    pub open: Signal<bool>,
    pub query: Signal<String>,
    pub selected: Signal<i64>,
    pub results: Signal<Vec<LauncherResult>>,
    pub loading: Signal<bool>,
    pub error: Signal<String>,
    pub display: Signal<Option<DisplayId>>,
}

fn save_tick(settings: LauncherSettingsState) {
    settings.dirty.set(settings.dirty.get_untracked() + 1);
}

fn folders_of(settings: LauncherSettingsState) -> Vec<String> {
    launcher::effective_folders(&settings.folder_mode.get_untracked(), &settings.folders.get_untracked())
}

pub fn start(settings: LauncherSettingsState) -> LauncherState {
    let open = signal(false);
    let query = signal(String::new());
    let selected = signal(-1i64);
    let results = signal(Vec::<LauncherResult>::new());
    let loading = signal(false);
    let error = signal(String::new());
    let display = signal(None::<DisplayId>);
    let revision = signal(0u64);
    let state = LauncherState { open, query, selected, results, loading, error, display };
    set_interval(Duration::from_millis(50), move || {
        let is_open = open.get_untracked();
        launcher::hotkey::set_nav(is_open);
        if !is_open {
            return;
        }
        match launcher::hotkey::take_nav() {
            Some(NavAction::Up) => move_selection(state, -1),
            Some(NavAction::Down) => move_selection(state, 1),
            Some(NavAction::Enter) => open_selected(state),
            Some(NavAction::Escape) => state.open.set(false),
            None => {}
        }
    });
    set_interval(Duration::from_millis(300), move || {
        if open.get_untracked() {
            let current = launcher::revision();
            if current != revision.get_untracked() {
                revision.set(current);
            }
        }
    });
    set_interval(Duration::from_millis(150), move || {
        if launcher::hotkey::take_request() {
            let next = !open.get_untracked();
            if next {
                query.set(String::new());
                selected.set(-1);
                error.set(String::new());
                display.set(launcher::display_under_cursor());
                launcher::ensure_index(settings.search_apps.get_untracked(), settings.search_files.get_untracked(), &folders_of(settings));
            }
            open.set(next);
        }
    });
    effect(move || {
        let is_open = open.get();
        let query_value = query.get();
        let _ = revision.get();
        if !is_open {
            loading.set(false);
            results.set(Vec::new());
            return;
        }
        let folders = folders_of(settings);
        launcher::ensure_index(settings.search_apps.get_untracked(), settings.search_files.get_untracked(), &folders);
        let snapshot = launcher::snapshot(settings.search_apps.get_untracked(), settings.search_files.get_untracked());
        let mut entries = snapshot.apps;
        entries.extend(snapshot.files);
        let engine = SearchEngine::parse(&settings.engine.get());
        let found = launcher::search(&entries, &query_value, 30, settings.search_web.get_untracked(), engine);
        loading.set(snapshot.apps_building || snapshot.files_building);
        if query_value.trim().is_empty() || found.is_empty() {
            selected.set(-1);
        } else {
            selected.set(0);
        }
        results.set(found);
    });
    state
}
fn open_result(state: LauncherState, result: &LauncherResult) {
    match launcher::open_target(&result.target) {
        Ok(()) => {
            state.open.set(false);
            state.error.set(String::new());
        }
        Err(error) => state.error.set(format!("无法打开「{}」：{error}", result.name)),
    }
}

fn move_selection(state: LauncherState, delta: i64) {
    let count = state.results.get_untracked().len() as i64;
    if count == 0 {
        state.selected.set(-1);
        return;
    }
    let current = state.selected.get_untracked();
    let next = if current < 0 { 0 } else { (current + delta).rem_euclid(count) };
    state.selected.set(next);
}

fn open_selected(state: LauncherState) {
    let index = state.selected.get_untracked();
    if index >= 0 {
        if let Some(result) = state.results.get_untracked().get(index as usize).cloned() {
            open_result(state, &result);
        }
    }
}
pub fn open_now(state: LauncherState, settings: LauncherSettingsState) {
    state.query.set(String::new());
    state.selected.set(-1);
    state.error.set(String::new());
    state.display.set(launcher::display_under_cursor());
    launcher::ensure_index(settings.search_apps.get_untracked(), settings.search_files.get_untracked(), &folders_of(settings));
    state.open.set(true);
}

pub fn launcher_window(
    state: LauncherState,
    settings: LauncherSettingsState,
    dark: Signal<bool>,
) -> impl IntoElement {
    let mut attributes = WindowAttributes::for_kind(WindowKind::Normal);
    attributes.level = Some(WindowLevel::AboveDock);
    attributes.transparent = Some(true);
    attributes.frame = Some(WindowFrameStyle::None);
    attributes.resizable = Some(false);
    attributes.taskbar = Some(false);
    attributes.shadow = Some(false);
    attributes.activate = Some(true);
    attributes.close_on_blur = Some(true);
    let mut window = nsh_core::element::desktop::window()
        .id("launcher")
        .kind(WindowKind::Normal)
        .title(t!("island.launcher_title"))
        .default_size(size(px(680.0), px(460.0)))
        .default_position(WindowPosition::Center)
        .attributes(attributes)
        .open(state.open)
        .on_open_change(move |event| {
            state.open.set(event.open);
            launcher::hotkey::set_nav(event.open);
        });
    if let Some(id) = state.display.get_untracked() {
        window = window.display(id);
    }
    window.content(move || content(state, settings, dark))
}
fn content(state: LauncherState, _settings: LauncherSettingsState, dark: Signal<bool>) -> impl IntoElement {
    view()
        .flex_1()
        .w_full()
        .h_full()
        .overflow_hidden()
        .rounded(px(12.0))
        .border(px(1.0))
        .border_color(move || palette(dark.get()).border)
        .bg(move || palette(dark.get()).bg)
        .text_color(move || palette(dark.get()).fg)
        .p(px(12.0))
        .flex_col()
        .gap(px(8.0))
        .child(
            view()
                .flex_row()
                .items_center()
                .gap(px(8.0))
                .h(px(38.0))
                .child(
                    text("⌕")
                        .text_size(px(20.0))
                        .text_color(move || palette(dark.get()).muted),
                )
                .child(
                    input()
                        .value(move || state.query.get())
                        .placeholder(move || t!("island.launcher_placeholder"))
                        .placeholder_color(move || palette(dark.get()).faint)
                        .text_color(move || palette(dark.get()).fg)
                        .bg(move || palette(dark.get()).input)
                        .h(px(36.0))
                        .flex_1()
                        .submit_behavior(SubmitBehavior::Submit)
                        .px(px(10.0))
                        .rounded(px(8.0))
                        .text_size(px(15.0))
                        .auto_focus(true)
                        .on_change(move |text: &str| {
                            state.query.set(text.to_string());
                            state.error.set(String::new());
                        })
                        .on_submit(move |_text: &str| open_selected(state)),
                )
                .child(
                    text("Esc")
                        .px(px(6.0))
                        .py(px(2.0))
                        .rounded(px(5.0))
                        .bg(move || palette(dark.get()).input)
                        .text_size(px(9.0))
                        .text_color(move || palette(dark.get()).faint),
                ),
        )
        .child(
            view()
                .flex_1()
                .min_h(px(0.0))
                .w_full()
                .child(
                    scroll()
                        .flex_1()
                        .min_h(px(0.0))
                        .scrollbar(false)
                        .child(list_body(state, dark)),
                ),
        )
        .child(
            view()
                .flex_row()
                .items_center()
                .gap(px(10.0))
                .pb(px(4.0))
                .child(
                    text(move || t!("island.launcher_hint"))
                        .text_size(px(10.0))
                        .text_color(move || palette(dark.get()).faint),
                )
                .child(view().flex_1())
                .child(
                    text(move || {
                        if !state.error.get().is_empty() {
                            state.error.get()
                        } else if state.loading.get() {
                            t!("island.launcher_indexing")
                        } else {
                            String::new()
                        }
                    })
                    .truncate()
                    .text_size(px(10.0))
                    .text_color(move || if !state.error.get().is_empty() { palette(dark.get()).warning } else { palette(dark.get()).faint }),
                ),
        )
}
fn list_body(state: LauncherState, dark: Signal<bool>) -> impl IntoElement {
    view()
        .flex_col()
        .gap(px(2.0))
        .child(when(move || state.query.get().trim().is_empty(), move || {
            text(move || t!("island.launcher_empty"))
                .text_size(px(12.0))
                .text_color(move || palette(dark.get()).faint)
                .py(px(24.0))
                .items_center()
        }))
        .child(when(
            move || !state.query.get().trim().is_empty() && state.results.get().is_empty(),
            move || {
                text(move || if state.loading.get() { t!("island.launcher_indexing") } else { t!("island.launcher_no_results") })
                    .text_size(px(12.0))
                    .text_color(move || palette(dark.get()).faint)
                    .py(px(24.0))
                    .items_center()
            },
        ))
        .child(each(state.results, |result| result.id.clone(), move |result, index| {
            result_row(state, result, index, dark)
        }))
}

fn result_row(
    state: LauncherState,
    result: ReadSignal<LauncherResult>,
    index: ReadSignal<usize>,
    dark: Signal<bool>,
) -> impl IntoElement {
    let snapshot = result.get();
    let result_for_press = snapshot.clone();
    let glyph = match snapshot.kind {
        ResultKind::Web => "◎",
        ResultKind::File => "▤",
        ResultKind::App => "▣",
    };
    view()
        .flex_row()
        .items_center()
        .gap(px(8.0))
        .px(px(8.0))
        .py(px(5.0))
        .rounded(px(7.0))
        .bg(move || if state.selected.get() == index.get() as i64 { palette(dark.get()).row } else { palette(dark.get()).bg })
        .on_mouse_enter(move |_event: &MouseEvent| state.selected.set(index.get() as i64))
        .on_press(move || open_result(state, &result_for_press))
        .child(
            text(glyph)
                .w(px(22.0))
                .text_size(px(15.0))
                .text_color(move || palette(dark.get()).muted),
        )
        .child(
            view()
                .flex_col()
                .flex_1()
                .child(
                    text(move || result.get().name)
                        .truncate()
                        .text_size(px(13.0))
                        .text_color(move || palette(dark.get()).fg),
                )
                .child(
                    text(move || result.get().subtitle)
                        .truncate()
                        .text_size(px(9.5))
                        .text_color(move || palette(dark.get()).faint),
                ),
        )
        .child(
            text(move || match result.get().kind {
                ResultKind::Web => t!("island.launcher_web"),
                ResultKind::File => t!("island.launcher_file"),
                ResultKind::App => t!("island.launcher_app"),
            })
            .text_size(px(9.0))
            .text_color(move || palette(dark.get()).faint),
        )
}
fn shortcut_conflict(_settings: LauncherSettingsState, translation_shortcut: Signal<String>, label: &str) -> Option<String> {
    if label == translation_shortcut.get_untracked() {
        return Some(t!("island.shortcut_owner_translation"));
    }
    match label {
        "Ctrl+Shift+I" => Some(t!("island.shortcut_owner_island")),
        "Ctrl+Shift+O" => Some(t!("island.shortcut_owner_through")),
        "Ctrl+Shift+M" => Some(t!("island.shortcut_owner_monitor")),
        "Ctrl+Shift+T" => Some(t!("island.shortcut_owner_tuck")),
        _ => None,
    }
}

fn apply_shortcut(settings: LauncherSettingsState, translation_shortcut: Signal<String>) {
    let draft = settings.shortcut_draft.get_untracked();
    if let Some(owner) = shortcut_conflict(settings, translation_shortcut, &draft) {
        settings.error.set(t!("island.launcher_shortcut_taken", shortcut = draft, owner = owner));
        return;
    }
    match launcher::hotkey::set_shortcut(&draft, settings.enabled.get_untracked()) {
        Ok(label) => {
            settings.shortcut.set(label.clone());
            settings.shortcut_draft.set(label);
            settings.error.set(String::new());
            save_tick(settings);
        }
        Err(error) => settings.error.set(error),
    }
}

fn set_enabled(settings: LauncherSettingsState, translation_shortcut: Signal<String>, enabled: bool) {
    let shortcut = settings.shortcut.get_untracked();
    if enabled {
        if let Some(owner) = shortcut_conflict(settings, translation_shortcut, &shortcut) {
            settings.error.set(t!("island.launcher_shortcut_taken", shortcut = shortcut, owner = owner));
            return;
        }
    }
    match launcher::hotkey::set_shortcut(&shortcut, enabled) {
        Ok(label) => {
            settings.enabled.set(enabled);
            settings.shortcut.set(label);
            settings.shortcut_draft.set(settings.shortcut.get_untracked());
            settings.error.set(String::new());
            save_tick(settings);
        }
        Err(error) => settings.error.set(error),
    }
}

fn double_ctrl(settings: LauncherSettingsState) -> bool {
    settings.shortcut.get() == "DoubleCtrl"
}

fn set_trigger_mode(settings: LauncherSettingsState, translation_shortcut: Signal<String>, double: bool) {
    let next = if double {
        "DoubleCtrl".to_string()
    } else if double_ctrl(settings) {
        "Alt+Space".to_string()
    } else {
        settings.shortcut.get_untracked()
    };
    if !double {
        if let Some(owner) = shortcut_conflict(settings, translation_shortcut, &next) {
            settings.error.set(t!("island.launcher_shortcut_taken", shortcut = next, owner = owner));
            return;
        }
    }
    match launcher::hotkey::set_shortcut(&next, settings.enabled.get_untracked()) {
        Ok(label) => {
            settings.shortcut.set(label.clone());
            settings.shortcut_draft.set(label);
            settings.error.set(String::new());
            save_tick(settings);
        }
        Err(error) => settings.error.set(error),
    }
}

fn record_shortcut(settings: LauncherSettingsState, translation_shortcut: Signal<String>, event: &KeyEvent) {
    if !settings.recording.get_untracked() {
        return;
    }
    if event.key == "Escape" {
        settings.recording.set(false);
        return;
    }
    if is_modifier_key(&event.key) {
        return;
    }
    if let Some(label) = shortcut_label(event) {
        settings.recording.set(false);
        settings.shortcut_draft.set(label);
        apply_shortcut(settings, translation_shortcut);
    }
}
fn toggle_row(label: impl Fn() -> String + 'static, value: Signal<bool>, settings: LauncherSettingsState, dark: Signal<bool>) -> impl IntoElement {
    view()
        .flex_row()
        .items_center()
        .gap(px(6.0))
        .child(
            text(label)
                .flex_1()
                .text_size(px(13.0))
                .text_color(move || palette(dark.get()).muted),
        )
        .child(pill(
            move || t!("island.on"),
            Rc::new(move || value.get()),
            move || {
                value.set(true);
                save_tick(settings);
            },
            dark,
        ))
        .child(pill(
            move || t!("island.off"),
            Rc::new(move || !value.get()),
            move || {
                value.set(false);
                save_tick(settings);
            },
            dark,
        ))
}

fn pill(label: impl Fn() -> String + 'static, active: Rc<dyn Fn() -> bool + 'static>, press: impl Fn() + 'static, dark: Signal<bool>) -> impl IntoElement {
    let background = active.clone();
    view()
        .px(px(9.0))
        .py(px(3.0))
        .rounded(px(6.0))
        .bg(move || if background() { palette(dark.get()).accent } else { palette(dark.get()).input })
        .on_press(press)
        .child(
            text(label)
                .text_size(px(10.0))
                .text_color(move || if active() { palette(dark.get()).bg } else { palette(dark.get()).muted }),
        )
}

fn mini(label: impl Into<String> + 'static, dark: Signal<bool>, press: impl Fn() + 'static) -> impl IntoElement {
    let label = label.into();
    text(move || label.clone())
        .px(px(7.0))
        .py(px(2.0))
        .rounded(px(5.0))
        .bg(move || palette(dark.get()).input)
        .text_size(px(10.0))
        .text_color(move || palette(dark.get()).muted)
        .on_press(press)
}
fn mini_dynamic(label: impl Fn() -> String + 'static, dark: Signal<bool>, press: impl Fn() + 'static) -> impl IntoElement {
    text(label)
        .px(px(7.0))
        .py(px(2.0))
        .rounded(px(5.0))
        .bg(move || palette(dark.get()).input)
        .text_size(px(10.0))
        .text_color(move || palette(dark.get()).muted)
        .on_press(press)
}
fn add_folder(settings: LauncherSettingsState) {
    let draft = settings.folder_draft.get_untracked().trim().to_string();
    if draft.is_empty() {
        return;
    }
    let mut folders = settings.folders.get_untracked();
    if !folders.iter().any(|folder| folder.eq_ignore_ascii_case(&draft)) {
        folders.push(draft);
        settings.folders.set(folders);
        settings.folder_mode.set("custom".to_string());
        save_tick(settings);
    }
    settings.folder_draft.set(String::new());
}

fn remove_folder(settings: LauncherSettingsState, folder: String) {
    let folders: Vec<String> = settings.folders.get_untracked().into_iter().filter(|current| current != &folder).collect();
    settings.folders.set(folders);
    save_tick(settings);
}

pub fn settings_section(
    settings: LauncherSettingsState,
    translation_shortcut: Signal<String>,
    dark: Signal<bool>,
) -> impl IntoElement {
    let defaults = launcher::default_folders();
    let default_text = defaults.join("\n");
    let custom = move || settings.folder_mode.get() == "custom";
    view()
        .flex_col()
        .gap(px(5.0))
        .child(
            text(move || t!("island.launcher_settings"))
                .text_size(px(12.0))
                .text_color(move || palette(dark.get()).muted)
                .mt(px(6.0)),
        )
        .child(
            view()
                .flex_row()
                .items_center()
                .gap(px(6.0))
                .child(
                    text(move || t!("island.launcher_enabled"))
                        .flex_1()
                        .text_size(px(13.0))
                        .text_color(move || palette(dark.get()).muted),
                )
                .child(pill(
                    move || t!("island.on"),
                    Rc::new(move || settings.enabled.get()),
                    move || set_enabled(settings, translation_shortcut, true),
                    dark,
                ))
                .child(pill(
                    move || t!("island.off"),
                    Rc::new(move || !settings.enabled.get()),
                    move || set_enabled(settings, translation_shortcut, false),
                    dark,
                )),
        )
        .child(
            view()
                .flex_row()
                .items_center()
                .gap(px(6.0))
                .child(
                    text(move || t!("island.launcher_trigger"))
                        .text_size(px(13.0))
                        .text_color(move || palette(dark.get()).muted),
                )
                .child(pill(
                    move || t!("island.launcher_combination"),
                    Rc::new(move || !double_ctrl(settings)),
                    move || set_trigger_mode(settings, translation_shortcut, false),
                    dark,
                ))
                .child(pill(
                    move || t!("island.launcher_double_ctrl"),
                    Rc::new(move || double_ctrl(settings)),
                    move || set_trigger_mode(settings, translation_shortcut, true),
                    dark,
                )),
        )
        .child(when(move || !double_ctrl(settings), move || {
            let capture = element_ref();
            view()
                .flex_row()
                .items_center()
                .gap(px(6.0))
                .child(
                    text(move || t!("island.launcher_shortcut"))
                        .text_size(px(13.0))
                        .text_color(move || palette(dark.get()).muted),
                )
                .child(
                    input()
                        .readonly(true)
                        .value(move || if settings.recording.get() { t!("island.launcher_recording") } else { settings.shortcut.get() })
                        .text_color(move || palette(dark.get()).fg)
                        .bg(move || palette(dark.get()).input)
                        .h(px(28.0))
                        .w(px(150.0))
                        .px(px(8.0))
                        .rounded(px(6.0))
                        .auto_focus(move || settings.recording.get())
                        .element_ref(&capture)
                        .on_key_down(move |event: &KeyEvent| record_shortcut(settings, translation_shortcut, event)),
                )
                .child(mini_dynamic(
                    move || if settings.recording.get() { t!("island.launcher_cancel") } else { t!("island.launcher_record") },
                    dark,
                    move || {
                        if settings.recording.get_untracked() {
                            settings.recording.set(false);
                        } else {
                            settings.recording.set(true);
                            capture.focus();
                        }
                    },
                ))
                .child(mini(t!("island.launcher_reset"), dark, move || {
                    settings.shortcut_draft.set("Alt+Space".to_string());
                    apply_shortcut(settings, translation_shortcut);
                }))
        }))
        .child(when(move || double_ctrl(settings), move || {
            text(move || t!("island.launcher_double_ctrl_hint"))
                .text_size(px(10.0))
                .text_color(move || palette(dark.get()).faint)
        }))
        .child(toggle_row(move || t!("island.launcher_search_apps"), settings.search_apps, settings, dark))
        .child(toggle_row(move || t!("island.launcher_search_files"), settings.search_files, settings, dark))
        .child(toggle_row(move || t!("island.launcher_search_web"), settings.search_web, settings, dark))
        .child(
            view()
                .flex_row()
                .items_center()
                .gap(px(5.0))
                .child(
                    text(move || t!("island.launcher_engine"))
                        .text_size(px(13.0))
                        .text_color(move || palette(dark.get()).muted),
                )
                .child(pill(move || "Bing".to_string(), Rc::new(move || settings.engine.get() == "bing"), move || { settings.engine.set("bing".to_string()); save_tick(settings); }, dark))
                .child(pill(move || "Google".to_string(), Rc::new(move || settings.engine.get() == "google"), move || { settings.engine.set("google".to_string()); save_tick(settings); }, dark))
                .child(pill(move || "Baidu".to_string(), Rc::new(move || settings.engine.get() == "baidu"), move || { settings.engine.set("baidu".to_string()); save_tick(settings); }, dark))
                .child(pill(move || "DuckDuckGo".to_string(), Rc::new(move || settings.engine.get() == "duckduckgo"), move || { settings.engine.set("duckduckgo".to_string()); save_tick(settings); }, dark)),
        )
        .child(
            view()
                .flex_row()
                .items_center()
                .gap(px(6.0))
                .child(
                    text(move || t!("island.launcher_folders"))
                        .flex_1()
                        .text_size(px(13.0))
                        .text_color(move || palette(dark.get()).muted),
                )
                .child(when(move || !custom(), {
                    let defaults = defaults.clone();
                    move || {
                        let defaults = defaults.clone();
                        mini(t!("island.launcher_custom_folders"), dark, move || {
                            settings.folders.set(defaults.clone());
                            settings.folder_mode.set("custom".to_string());
                            save_tick(settings);
                        })
                    }
                }))
                .child(when(custom, move || {
                    mini(t!("island.launcher_default_folders"), dark, move || {
                        settings.folder_mode.set("default".to_string());
                        settings.folders.set(Vec::new());
                        save_tick(settings);
                    })
                })),
        )
        .child(when(move || !custom(), {
            let default_text = default_text.clone();
            move || {
                let default_text = default_text.clone();
                text(move || default_text.clone())
                    .text_size(px(10.0))
                    .text_color(move || palette(dark.get()).faint)
            }
        }))
        .child(when(custom, move || {
            view()
                .flex_col()
                .gap(px(3.0))
                .child(each(settings.folders, |folder| folder.clone(), move |folder, _| {
                    view()
                        .flex_row()
                        .items_center()
                        .gap(px(5.0))
                        .child(text(move || folder.get()).truncate().flex_1().text_size(px(10.0)).text_color(move || palette(dark.get()).faint))
                        .child(mini("✕", dark, move || remove_folder(settings, folder.get())))
                }))
                .child(
                    view()
                        .flex_row()
                        .items_center()
                        .gap(px(5.0))
                        .child(
                            input()
                                .value(move || settings.folder_draft.get())
                                .placeholder(move || t!("island.launcher_folder_placeholder"))
                                .text_color(move || palette(dark.get()).fg)
                                .bg(move || palette(dark.get()).input)
                                .h(px(28.0))
                                .flex_1()
                                .px(px(8.0))
                                .rounded(px(6.0))
                                .on_change(move |text: &str| settings.folder_draft.set(text.to_string()))
                                .on_submit(move |_text: &str| add_folder(settings)),
                        )
                        .child(mini(t!("island.launcher_add_folder"), dark, move || add_folder(settings))),
                )
        }))
        .child(when(move || !settings.error.get().is_empty(), move || {
            text(move || settings.error.get())
                .text_size(px(10.0))
                .text_color(move || palette(dark.get()).warning)
        }))
}