//! FocuSD Island on NSH: Phase 2 shell and clipboard pilot.
//!
//! The Win32 host (nsh/host) owns the top-level window and embeds this root
//! through AppView. The island keeps the capsule, adds a searchable
//! text/image clipboard with favorites, and a small settings page.

mod ai;
mod battery;
mod launcher;
mod launcher_ui;
mod lyrics;
mod lyrics_ui;
mod mail;
mod mail_ui;
mod media;
mod qq;
mod qq_ui;
pub mod screenshot;
mod screenshot_ui;
mod screenshot_advanced_ui;
mod sound;
mod translation;
mod translation_ui;
pub mod update;
mod update_ui;
mod clipboard;
mod file_shelf;
mod file_shelf_media;
pub mod file_shelf_drag;
pub mod file_shelf_picker;
mod file_shelf_ui;
mod ui;
pub mod focus;
#[cfg(feature = "embedded")]
pub mod embedded;
mod preferences;
mod window;

use std::{
    rc::Rc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use nsh_core::{element::input, prelude::*, Color};

use ai::{AgentSession, AgentState, AiSnapshot};
use battery::PeripheralBatteryDevice;
use clipboard::{ClipItem, ClipKind, ClipboardHistory};
use launcher_ui::{LauncherSettingsState, LauncherState};
use lyrics_ui::LyricsState;
use translation_ui::{TranslationSettings, TranslationState};
use media::MediaState;
use ui::{icons, theme, widgets};

#[cfg(feature = "embedded")]
fn wechat_set_enabled(enabled: bool) {
    embedded::set_wechat_enabled(enabled);
}
#[cfg(not(feature = "embedded"))]
fn wechat_set_enabled(_enabled: bool) {}

#[cfg(feature = "embedded")]
fn wechat_status() -> (bool, Option<u64>) {
    (embedded::wechat_available(), embedded::wechat_pending())
}
#[cfg(not(feature = "embedded"))]
fn wechat_status() -> (bool, Option<u64>) {
    (false, None)
}

#[cfg(feature = "embedded")]
fn wechat_request_focus() {
    embedded::request_focus_wechat();
}
#[cfg(not(feature = "embedded"))]
fn wechat_request_focus() {}

#[cfg(feature = "embedded")]
fn take_dropped_files() -> Vec<String> {
    embedded::take_dropped_files()
}


#[cfg(feature = "embedded")]
fn take_drag_result() -> Option<Result<(), String>> {
    embedded::take_drag_result()
}


#[cfg(feature = "embedded")]
pub(crate) fn request_file_drag(paths: Vec<String>) -> Result<(), String> {
    embedded::request_drag(paths);
    Ok(())
}

#[cfg(not(feature = "embedded"))]
pub(crate) fn request_file_drag(_paths: Vec<String>) -> Result<(), String> {
    Err("拖出文件只支持宿主窗口。".to_string())
}

#[cfg(feature = "embedded")]
fn take_pick_result() -> Option<Result<Vec<String>, String>> {
    embedded::take_pick_result()
}


#[cfg(feature = "embedded")]
pub(crate) fn request_file_pick() {
    embedded::request_pick();
}

#[cfg(not(feature = "embedded"))]
pub(crate) fn request_file_pick() {}


// The standalone build sizes its window with these; the embedded host keeps its own copy
// in `host/src/island.rs`, so they read as unused there.
#[allow(dead_code)] const COLLAPSED_WIDTH: f32 = 320.0;
#[allow(dead_code)] const COLLAPSED_HEIGHT: f32 = 40.0;
#[allow(dead_code)] const EXPANDED_WIDTH: f32 = 620.0;
#[allow(dead_code)] const EXPANDED_HEIGHT: f32 = 420.0;

#[derive(Clone, Copy)]
pub(crate) struct Palette {
    bg: Color,
    fg: Color,
    muted: Color,
    faint: Color,
    border: Color,
    row: Color,
    input: Color,
    accent: Color,
    running: Color,
    warning: Color,
    success: Color,
}

pub(crate) fn palette(dark: bool) -> Palette {
    // The dark side is the Tauri island's own palette (`src/styles/tokens.css`); a light island
    // keeps the same alphas over a paper surface.
    if dark {
        Palette {
            bg: theme::ISLAND_BG,
            fg: theme::TEXT_1,
            muted: theme::TEXT_2,
            faint: theme::TEXT_3,
            border: theme::WHITE_A08,
            row: theme::WHITE_A04,
            input: theme::WHITE_A06,
            accent: theme::ACCENT,
            running: Color::rgba(91, 156, 245, 255),
            warning: theme::WARNING,
            success: theme::SUCCESS,
        }
    } else {
        Palette {
            bg: rgba(0xF7F7F5FF).into(),
            fg: rgb(0x1B1D21).into(),
            muted: rgba(0x1B1D21AE).into(),
            faint: rgba(0x1B1D218A).into(),
            border: rgba(0x00000014).into(),
            row: rgba(0x0000000A).into(),
            input: rgba(0x0000000F).into(),
            accent: rgb(0x2E86B8).into(),
            running: rgb(0x2F6FD0).into(),
            warning: rgb(0xB57616).into(),
            success: rgb(0x2E8B62).into(),
        }
    }
}

#[derive(Clone, Copy)]
struct AiState {
    enabled: Signal<bool>,
    snapshot: Signal<AiSnapshot>,
    sessions: Signal<Vec<AgentSession>>,
    watermark: Signal<i64>,
    kinds: [Signal<bool>; 4],
}

#[derive(Clone, Copy)]
struct LyricsSettingsState {
    folder: Signal<String>,
    online: Signal<bool>,
    chinese: Signal<bool>,
}

#[derive(Clone, Copy)]
struct SettingsState {
    dark: Signal<bool>,
    clipboard_enabled: Signal<bool>,
    capture_images: Signal<bool>,
    max_items: Signal<usize>,
    ai: AiState,
    wechat_enabled: Signal<bool>,
    qq_enabled: Signal<bool>,
    mail: mail_ui::MailState,
    sound_enabled: Signal<bool>,
    lyrics: LyricsSettingsState,
    music_sources: Signal<Vec<media::KnownSource>>,
    translation: TranslationSettings,
    launcher: LauncherSettingsState,
    screenshot: screenshot_ui::ScreenshotSettingsState,
    shelf_show_count: Signal<bool>,
    shelf: file_shelf_ui::ShelfState,
    update: update_ui::UpdateState,
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
    let wechat_enabled = signal(saved.wechat_enabled);
    let qq_enabled = signal(saved.qq_enabled);
    let mail = mail_ui::start(saved.mail_accounts.clone());
    let qq = qq_ui::start(qq_enabled);
    let sound_enabled = signal(saved.sound_enabled);
    let shelf_show_count = signal(saved.shelf_show_count);
    let shelf = file_shelf_ui::start(saved.shelf_max_items, saved.shelf_double_click.clone());
    let update = update_ui::start(saved.update_endpoint.clone(), true);
    let lyrics_folder = signal(saved.lyrics_folder.clone());
    let lyrics_online = signal(saved.lyrics_online);
    let lyrics_chinese = signal(saved.lyrics_chinese);
    let music_sources = signal(saved.music_sources.clone());
    let translation_enabled = signal(saved.translation_enabled);
    let translation_shortcut = signal(saved.translation_shortcut.clone());
    let translation_direction = signal(saved.translation_direction.clone());
    let translation_provider = signal(saved.translation_provider.clone());
    let translation_custom_base_url = signal(saved.translation_custom_base_url.clone());
    let translation_models = signal(saved.translation_models.clone());
    let translation_auto = signal(saved.translation_auto);
    let translation_recording = signal(false);
    let translation_key_draft = signal(String::new());
    let translation_key_saved = signal(0u64);
    let translation_key_present = signal(false);
    let translation_error = signal(String::new());
    let translation_dirty = signal(0u64);
    let launcher_enabled = signal(saved.launcher_enabled);
    let launcher_shortcut = signal(saved.launcher_shortcut.clone());
    let launcher_search_apps = signal(saved.launcher_search_apps);
    let launcher_search_files = signal(saved.launcher_search_files);
    let launcher_search_web = signal(saved.launcher_search_web);
    let launcher_engine = signal(saved.launcher_engine.clone());
    let launcher_folder_mode = signal(saved.launcher_folder_mode.clone());
    let launcher_folders = signal(saved.launcher_folders.clone());
    let launcher_shortcut_draft = signal(saved.launcher_shortcut.clone());
    let launcher_recording = signal(false);
    let launcher_folder_draft = signal(String::new());
    let launcher_error = signal(String::new());
    let launcher_dirty = signal(0u64);
    let launcher_settings = LauncherSettingsState {
        enabled: launcher_enabled,
        shortcut: launcher_shortcut,
        search_apps: launcher_search_apps,
        search_files: launcher_search_files,
        search_web: launcher_search_web,
        engine: launcher_engine,
        folder_mode: launcher_folder_mode,
        folders: launcher_folders,
        shortcut_draft: launcher_shortcut_draft,
        recording: launcher_recording,
        folder_draft: launcher_folder_draft,
        error: launcher_error,
        dirty: launcher_dirty,
    };
    let screenshot_settings = screenshot::load();
    if let Err(error) = screenshot::start(&screenshot_settings) {
        log::warn!("screenshot unavailable: {error}");
    }
    let screenshot_state = screenshot_ui::ScreenshotSettingsState::new(&screenshot_settings);
    screenshot_ui::load_ocr_languages(screenshot_state);
    let translation_settings = TranslationSettings {
        enabled: translation_enabled,
        shortcut: translation_shortcut,
        direction: translation_direction,
        provider: translation_provider,
        custom_base_url: translation_custom_base_url,
        models: translation_models,
        auto: translation_auto,
        recording: translation_recording,
        key_draft: translation_key_draft,
        key_saved: translation_key_saved,
        key_present: translation_key_present,
        error: translation_error,
        dirty: translation_dirty,
    };
    wechat_set_enabled(saved.wechat_enabled);
    let ai_enabled = signal(saved.ai_enabled);
    let notify_done = signal(saved.notify_done);
    let notify_question = signal(saved.notify_question);
    let notify_scheduled = signal(saved.notify_scheduled);
    let notify_error = signal(saved.notify_error);
    let ai_status = ai::start(ai_enabled);
    let ai_sessions = signal(Vec::<AgentSession>::new());
    effect(move || {
        let list = ai_status.snapshot.get().sessions;
        ai_sessions.set(list.into_iter().take(4).collect());
    });
    let known_sound_keys = signal(Vec::<String>::new());
    let sound_ready = signal(false);
    effect(move || {
        let snapshot = ai_status.snapshot.get();
        let current: Vec<String> = snapshot.events.iter().map(|event| event.key.clone()).collect();
        if !sound_ready.get_untracked() {
            sound_ready.set(true);
            known_sound_keys.set(current);
            return;
        }
        let previous = known_sound_keys.get_untracked();
        let watermark = ai_status.watermark.get_untracked();
        let kinds = [
            notify_done.get_untracked(),
            notify_question.get_untracked(),
            notify_scheduled.get_untracked(),
            notify_error.get_untracked(),
        ];
        let fresh: Vec<&str> = snapshot
            .events
            .iter()
            .filter(|event| !previous.contains(&event.key) && event.at > watermark)
            .filter(|event| match event.kind {
                "done" => kinds[0],
                "question" => kinds[1],
                "scheduled" => kinds[2],
                "error" => kinds[3],
                _ => false,
            })
            .map(|event| event.kind)
            .collect();
        known_sound_keys.set(current);
        if fresh.is_empty() || !sound_enabled.get_untracked() {
            return;
        }
        if fresh.contains(&"error") {
            sound::play("error");
        } else if fresh.contains(&"question") {
            sound::play("question");
        } else {
            sound::play("complete");
        }
    });

    let ai_state = AiState {
        enabled: ai_enabled,
        snapshot: ai_status.snapshot,
        sessions: ai_sessions,
        watermark: ai_status.watermark,
        kinds: [notify_done, notify_question, notify_scheduled, notify_error],
    };
    let settings_state = SettingsState {
        dark,
        clipboard_enabled,
        capture_images,
        max_items,
        ai: ai_state,
        wechat_enabled,
        qq_enabled,
        mail,
        sound_enabled,
        lyrics: LyricsSettingsState { folder: lyrics_folder, online: lyrics_online, chinese: lyrics_chinese },
        music_sources,
        translation: translation_settings,
        launcher: launcher_settings,
        screenshot: screenshot_state,
        shelf_show_count,
        shelf,
        update,
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


    let batteries = signal(Vec::<PeripheralBatteryDevice>::new());
    let read_batteries = Rc::new(move || {
        spawn(async move {
            batteries.set(battery::read_batteries());
        });
    });
    read_batteries();
    let poll_batteries = read_batteries.clone();
    set_interval(Duration::from_secs(60), move || poll_batteries());

    let wechat = signal(wechat_status());
    let read_wechat = Rc::new(move || wechat.set(wechat_status()));
    read_wechat();
    let poll_wechat = read_wechat.clone();
    set_interval(Duration::from_secs(1), move || poll_wechat());

    let media_state = signal(Option::<MediaState>::None);
    media::set_source_order(saved.music_sources.clone());
    effect(move || media::set_source_order(music_sources.get()));
    let read_media = Rc::new(move || {
        let order = music_sources.get_untracked();
        spawn(async move {
            if let Ok(next) = media::state() {
                let mut sources = order;
                if media::remember_sources(&mut sources, &next.sessions) {
                    music_sources.set(sources);
                    save_settings(settings_state);
                }
                media_state.set(Some(next));
            }
        });
    });
    read_media();
    let poll_media = read_media.clone();
    set_interval(Duration::from_millis(1500), move || poll_media());
    let lyrics = lyrics_ui::start(media_state, lyrics_folder, lyrics_online, lyrics_chinese);

    translation::hotkey::start(&saved.translation_shortcut, saved.translation_enabled);
    let island_page = signal(false);
    let shelf_page = signal(false);

    #[cfg(feature = "embedded")]
    {
        let paths: Vec<String> = std::env::args()
            .skip(1)
            .filter(|arg| std::path::Path::new(arg).is_absolute())
            .collect();
        if !paths.is_empty() {
            file_shelf_ui::add_dropped(shelf, paths);
            shelf_page.set(true);
            window::set_expanded(expanded, true);
        }
    }
    let translation_active = signal(false);
    let translation = translation_ui::start(translation_settings, translation_active);
    launcher::hotkey::start(&saved.launcher_shortcut, saved.launcher_enabled);
    launcher::ensure_index(saved.launcher_search_apps, saved.launcher_search_files, &launcher::effective_folders(&saved.launcher_folder_mode, &saved.launcher_folders));
    let launcher = launcher_ui::start(launcher_settings);
    effect(move || {
        let _ = launcher_dirty.get();
        save_settings(settings_state);
    });
    effect(move || translation_active.set(translation.open.get() || island_page.get()));
    effect(move || {
        if island_page.get() && !translation_settings.enabled.get() {
            island_page.set(false);
        }
    });
    effect(move || {
        let _ = translation_dirty.get();
        save_settings(settings_state);
    });
    effect(move || {
        let _ = mail.dirty.get();
        save_settings(settings_state);
    });
    window::configure(expanded);
    window::install_tray(expanded);
    window::install_shortcut(expanded);
    #[cfg(feature = "embedded")]
    embedded::start(expanded, settings_open);

    effect(move || {
        if shelf_page.get() {
            file_shelf_ui::refresh(shelf);
        }
    });

    #[cfg(feature = "embedded")]
    {
        let shelf_state = shelf;
        let expanded_signal = expanded;
        let shelf_page_signal = shelf_page;
        let island_page_signal = island_page;
        spawn(async move {
            loop {
                timer(Duration::from_millis(80)).await;
                if let Some(result) = take_pick_result() {
                    match result {
                        Ok(paths) => file_shelf_ui::add_dropped(shelf_state, paths),
                        Err(error) => shelf_state.error.set(error),
                    }
                }
                if let Some(result) = take_drag_result() {
                    file_shelf_ui::drag_finished(shelf_state, result);
                }
                let paths = take_dropped_files();
                if paths.is_empty() {
                    continue;
                }
                file_shelf_ui::add_dropped(shelf_state, paths);
                shelf_page_signal.set(true);
                island_page_signal.set(false);
                shelf_state.dropping.set(true);
                if !expanded_signal.get_untracked() {
                    window::set_expanded(expanded_signal, true);
                }
                timer(Duration::from_millis(700)).await;
                shelf_state.dropping.set(false);
            }
        });
    }

    view()
        .flex_1()
        .items_center()
        .justify_center()
        .child(when(expanded, move || {
            expanded_island(expanded, settings_open, island_page, shelf_page, shelf, query, items, visible, dark, ai_state, media_state, lyrics, translation, translation_settings, launcher, launcher_settings, batteries, wechat, wechat_enabled, qq, qq_enabled)
        }))
        .child(when(move || !expanded.get(), move || {
            collapsed_island(expanded, shelf, shelf_show_count, items, dark, ai_state, media_state, lyrics)
        }))
        .child(settings_window(settings_open, settings_state, items, media_state))
        .child(translation_ui::translation_window(translation, translation_settings, settings_open, dark))
        .child(when(move || launcher.open.get(), move || launcher_ui::launcher_window(launcher, launcher_settings, dark)))
}

fn collapsed_island(
    expanded: Signal<bool>,
    shelf: file_shelf_ui::ShelfState,
    shelf_show_count: Signal<bool>,
    items: Signal<Vec<ClipItem>>,
    dark: Signal<bool>,
    ai: AiState,
    media_state: Signal<Option<MediaState>>,
    lyrics: LyricsState,
) -> impl IntoElement {
    let pulse_color = move || {
        if !ai.enabled.get() {
            return theme::PULSE;
        }
        ai_state_color(palette(dark.get()), ai.snapshot.get().state())
    };
    view()
        .relative()
        .w_full()
        .h(px(theme::COLLAPSED_HEIGHT))
        .flex_row()
        .items_center()
        .gap(px(theme::SPACE_MD))
        .rounded(px(theme::COLLAPSED_RADIUS))
        .overflow_hidden()
        .bg(move || palette(dark.get()).bg)
        .bg_linear_to_b()
        .gradient_from(theme::COLLAPSED_TOP)
        .gradient_to(theme::COLLAPSED_BOTTOM)
        .shadow(theme::collapsed_shadow(false))
        .cursor_pointer()
        .text_color(move || palette(dark.get()).fg)
        // The leading 8px strip drags the window; the rest of the bar opens it.
        .child(
            view()
                .absolute()
                .left(px(0.0))
                .top(px(0.0))
                .w(px(8.0))
                .h(px(theme::COLLAPSED_HEIGHT))
                .window_drag(true),
        )
        // The pulse dot: the agent's state color, lit as the Tauri dot is.
        .child(
            view()
                .ml(px(theme::COLLAPSED_PADDING_X))
                .w(px(9.0))
                .h(px(9.0))
                .flex_shrink(0.0)
                .rounded(px(4.5))
                .bg(pulse_color)
                .shadow(theme::soft_glow(theme::PULSE_GLOW, 14.0)),
        )
        .child(when(
            move || ai.enabled.get() && ai.snapshot.get().state().is_some(),
            move || {
                text(move || ai_inline_label(ai))
                    .flex_shrink(0.0)
                    .text_size(px(theme::HINT_SIZE))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(move || ai_state_color(palette(dark.get()), ai.snapshot.get().state()))
            },
        ))
        .child(lyrics_ui::collapsed(lyrics, media_state, dark))
        .child(when(move || ai_unseen(ai) > 0, move || {
            widgets::count_badge(
                move || ai_unseen(ai).to_string(),
                theme::ISLAND_BG,
                theme::WARNING,
            )
            .on_press(move || open_cindy(ai))
        }))
        .child(when(
            move || shelf_show_count.get() && !shelf.items.get().is_empty(),
            move || {
                view()
                    .h(px(22.0))
                    .px(px(7.0))
                    .flex_shrink(0.0)
                    .flex_row()
                    .items_center()
                    .gap(px(4.0))
                    .rounded(px(11.0))
                    .bg(theme::WHITE_A08)
                    .text_color(theme::TEXT_2)
                    .child(widgets::icon(icons::INBOX, 13.0))
                    .child(
                        text(move || shelf.items.get().len().to_string())
                            .text_size(px(theme::HINT_SIZE))
                            .font_weight(FontWeight::SEMIBOLD),
                    )
            },
        ))
        .child(
            text(move || t!("island.count", count = items.get().len()))
                .flex_shrink(0.0)
                .text_size(px(theme::LABEL_SIZE))
                .text_color(move || palette(dark.get()).faint),
        )
        .child(
            view()
                .mr(px(theme::COLLAPSED_PADDING_X))
                .flex_shrink(0.0)
                .text_color(theme::TEXT_3)
                .child(widgets::icon(icons::CHEVRON_UP, 14.0)),
        )
        .on_press(move || window::set_expanded(expanded, true))
}
enum CopyTarget {
    Text(String),
    Image(String),
    None,
}

fn copy_target(item: &ClipItem) -> CopyTarget {
    match item.kind {
        ClipKind::Text => CopyTarget::Text(item.text.clone()),
        ClipKind::Image => item
            .image_url
            .clone()
            .map_or(CopyTarget::None, CopyTarget::Image),
    }
}

fn expanded_island(
    expanded: Signal<bool>,
    settings_open: Signal<bool>,
    island_page: Signal<bool>,
    shelf_page: Signal<bool>,
    shelf: file_shelf_ui::ShelfState,
    query: Signal<String>,
    items: Signal<Vec<ClipItem>>,
    visible: Signal<Vec<ClipItem>>,
    dark: Signal<bool>,
    ai: AiState,
    media_state: Signal<Option<MediaState>>,
    lyrics: LyricsState,
    translation: TranslationState,
    translation_settings: TranslationSettings,
    launcher: LauncherState,
    launcher_settings: LauncherSettingsState,
    batteries: Signal<Vec<PeripheralBatteryDevice>>,
    wechat: Signal<(bool, Option<u64>)>,
    wechat_enabled: Signal<bool>,
    qq: qq_ui::QqState,
    qq_enabled: Signal<bool>,
) -> impl IntoElement {
    view()
        .w_full()
        .h_full()
        .min_w(px(0.0))
        .flex_col()
        .rounded(px(theme::EXPANDED_RADIUS))
        .overflow_hidden()
        .bg(move || palette(dark.get()).bg)
        .bg_linear_to_b()
        .gradient_from(theme::CARD_TOP)
        .gradient_to(Color::TRANSPARENT)
        .shadow(theme::card_shadow())
        .child(expanded_header(
            expanded,
            settings_open,
            island_page,
            shelf_page,
            launcher,
            launcher_settings,
            dark,
            ai,
        ))
        .child(
            view()
                .flex_1()
                .min_w(px(0.0))
                .mb(px(theme::ISLAND_PADDING_X))
                .flex_col()
                .rounded(px(theme::PANEL_RADIUS))
                .overflow_hidden()
                .child(when(move || island_page.get(), move || {
                    translation_ui::island_panel(translation, translation_settings, settings_open, dark)
                }))
                .child(when(move || shelf_page.get(), move || file_shelf_ui::panel(shelf, dark)))
                .child(when(move || !island_page.get() && !shelf_page.get(), move || {
                    view()
                        .flex_1()
                        .w_full()
                        .min_w(px(0.0))
                        .flex_col()
                        .child(when(move || ai.enabled.get(), move || ai_panel(ai, dark)))
                        .child(when(
                            move || media_state.get().as_ref().is_some_and(|media| media.available),
                            move || media_panel(media_state, dark),
                        ))
                        .child(when(
                            move || !media_state.get().as_ref().is_some_and(|media| media.available),
                            move || media_idle(media_state, dark),
                        ))
                        .child(when(
                            move || media_state.get().as_ref().is_some_and(|media| media.available),
                            move || lyrics_ui::panel(lyrics, media_state, dark),
                        ))
                        .child(when(move || !batteries.get().is_empty(), move || battery_panel(batteries, dark)))
                        .child(wechat_panel(wechat, wechat_enabled, dark))
                        .child(when(move || qq_enabled.get(), move || qq_ui::panel(qq, dark)))
                        .child(search_and_list(query, items, visible, dark))
                })),
        )
}

fn expanded_header(
    expanded: Signal<bool>,
    settings_open: Signal<bool>,
    island_page: Signal<bool>,
    shelf_page: Signal<bool>,
    launcher: LauncherState,
    launcher_settings: LauncherSettingsState,
    dark: Signal<bool>,
    ai: AiState,
) -> impl IntoElement {
    let on_clipboard = move || !island_page.get() && !shelf_page.get();
    // Two overlaid rows: the title row packs from the left, the actions row packs
    // from the right. A single row with both cannot keep the actions inside the
    // window here, and absolute positioning inside the row crashes the layout.
    view()
        .flex_col()
        .w_full()
        .mt(px(18.0))
        .mb(px(theme::ISLAND_GAP))
        .child(
            view()
                .relative()
                .flex_row()
                .items_center()
                .w_full()
                .h(px(theme::HEADER_HEIGHT))
                .child(
                    view()
                        .absolute()
                        .left(px(0.0))
                        .top(px(0.0))
                        .w(px(8.0))
                        .h(px(theme::HEADER_HEIGHT))
                        .window_drag(true),
                )
                .child(
                    view()
                        .ml(px(theme::ISLAND_PADDING_X))
                        .flex_row()
                        .items_center()
                        .gap(px(9.0))
                        .flex_shrink(0.0)
                        .child(
                            view()
                                .text_color(theme::ACCENT)
                                .child(widgets::icon(icons::CLIPBOARD, 15.0)),
                        )
                        .child(
                            text(move || t!("island.title"))
                                .text_size(px(theme::BRAND_SIZE))
                                .font_weight(FontWeight::BOLD)
                                .text_color(move || palette(dark.get()).fg),
                        ),
                )
                .child(
                    view()
                        .ml(px(theme::SPACE_XL))
                        .flex_row()
                        .items_center()
                        .gap(px(theme::SPACE_XS))
                        .flex_shrink(0.0)
                        .child(widgets::page_dot(theme::DOT_CLIPBOARD, on_clipboard()).on_press(
                            move || {
                                island_page.set(false);
                                shelf_page.set(false);
                            },
                        ))
                        .child(
                            widgets::page_dot(theme::DOT_SHELF, shelf_page.get()).on_press(
                                move || {
                                    island_page.set(false);
                                    shelf_page.set(true);
                                },
                            ),
                        )
                        .child(
                            widgets::page_dot(theme::DOT_TRANSLATION, island_page.get()).on_press(
                                move || {
                                    island_page.set(true);
                                    shelf_page.set(false);
                                },
                            ),
                        ),
                ),
        )
        .child(
            view()
                .flex_row()
                .items_center()
                .w_full()
                .h(px(theme::HEADER_HEIGHT))
                .mt(px(-theme::HEADER_HEIGHT))
                .child(
                    view()
                        .ml(px(theme::ISLAND_ACTIONS_INSET))
                        .w(px(theme::ISLAND_ACTIONS_WIDTH))
                        .flex_row()
                        .items_center()
                        .gap(px(4.0))
                        .child(when(move || ai_unseen(ai) > 0, move || {
                            widgets::count_badge(
                                move || ai_unseen(ai).to_string(),
                                theme::ISLAND_BG,
                                theme::WARNING,
                            )
                            .on_press(move || open_cindy(ai))
                        }))
                        .child(
                            widgets::icon_button(icons::SEARCH, 16.0, theme::TEXT_2)
                                .on_press(move || launcher_ui::open_now(launcher, launcher_settings)),
                        )
                        .child(when(move || settings_open.get(), move || {
                            widgets::icon_button(icons::X, 16.0, theme::TEXT_2)
                                .on_press(move || settings_open.set(false))
                        }))
                        .child(when(move || !settings_open.get(), move || {
                            widgets::icon_button(icons::SETTINGS, 16.0, theme::TEXT_2)
                                .on_press(move || settings_open.set(true))
                        }))
                        .child(
                            widgets::icon_button(icons::CHEVRON_UP, 16.0, theme::TEXT_2)
                                .on_press(move || window::set_expanded(expanded, false)),
                        ),
                ),
        )
}

fn ai_panel(ai: AiState, dark: Signal<bool>) -> impl IntoElement {
    view()
        .flex_col()
        .gap(px(3.0))
        .px(px(14.0))
        .pb(px(6.0))
        .child(ai_summary_row(ai, dark))
        .child(each(ai.sessions, |session| session.id.clone(), move |session, _| {
            ai_row(ai, session, dark)
        }))
}

fn ai_summary_row(ai: AiState, dark: Signal<bool>   ) -> impl IntoElement {
    view()
        .flex_row()
        .items_center()
        .gap(px(8.0))
        .child(
            text(move || t!("island.ai"))
                .text_size(px(12.0))
                .font_weight(FontWeight::MEDIUM)
                .text_color(move || ai_state_color(palette(dark.get()), ai.snapshot.get().state())),
        )
        .child(
            text(move || ai_counts(ai))
                .text_size(px(11.0))
                .text_color(move || palette(dark.get()).muted),
        )
        .child(view().flex_1())
        .child(
            text(move || t!("island.ai_focus"))
                .text_size(px(10.0))
                .text_color(move || palette(dark.get()).accent)
                .on_press(move || open_cindy(ai)),
        )
}

fn ai_row(ai: AiState, session: ReadSignal<AgentSession>, dark: Signal<bool>) -> impl IntoElement {
    let session = session.get();
    let state = session.state;
    let color = ai_state_color(palette(dark.get()), Some(state));
    view()
        .flex_row()
        .items_center()
        .gap(px(6.0))
        .px(px(8.0))
        .py(px(2.0))
        .rounded(px(6.0))
        .active(|style| style.bg(palette(dark.get()).row))
        .on_press(move || open_cindy(ai))
        .child(view().w(px(5.0)).h(px(5.0)).rounded(px(3.0)).bg(color))
        .child(
            text(ai_state_label(state))
                .text_size(px(9.0))
                .text_color(color),
        )
        .child(
            text(session.agent.clone())
                .text_size(px(10.0))
                .text_color(move || palette(dark.get()).faint),
        )
        .child(
            text(session.title.clone())
                .flex_1()
                .truncate()
                .text_size(px(11.0))
                .text_color(move || palette(dark.get()).fg),
        )
}

fn ai_state_color(palette: Palette, state: Option<AgentState>) -> Color {
    match state {
        Some(AgentState::Attention) => palette.warning,
        Some(AgentState::Running) => palette.running,
        Some(AgentState::Done) => palette.success,
        None => palette.faint,
    }
}

fn ai_state_label(state: AgentState) -> String {
    match state {
        AgentState::Attention => t!("island.ai_attention"),
        AgentState::Running => t!("island.ai_running"),
        AgentState::Done => t!("island.ai_done"),
    }
}

fn ai_inline_label(ai: AiState) -> String {
    let snapshot = ai.snapshot.get();
    match snapshot.state() {
        Some(AgentState::Attention) => {
            t!("island.ai_attention_count", count = snapshot.count(AgentState::Attention))
        }
        Some(AgentState::Running) => {
            t!("island.ai_running_count", count = snapshot.count(AgentState::Running))
        }
        Some(AgentState::Done) => t!("island.ai_done"),
        None => String::new(),
    }
}

fn ai_counts(ai: AiState) -> String {
    let snapshot = ai.snapshot.get();
    let mut parts = Vec::new();
    let running = snapshot.count(AgentState::Running);
    let attention = snapshot.count(AgentState::Attention);
    let done = snapshot.count(AgentState::Done);
    if running > 0 {
        parts.push(t!("island.ai_running_count", count = running));
    }
    if attention > 0 {
        parts.push(t!("island.ai_attention_count", count = attention));
    }
    if done > 0 {
        parts.push(t!("island.ai_done_count", count = done));
    }
    if parts.is_empty() {
        t!("island.ai_idle")
    } else {
        parts.join(" · ")
    }
}

fn ai_unseen(ai: AiState) -> usize {
    ai::unseen(
        &ai.snapshot.get(),
        ai.watermark.get(),
        [
            ai.kinds[0].get(),
            ai.kinds[1].get(),
            ai.kinds[2].get(),
            ai.kinds[3].get(),
        ],
    )
}

fn open_cindy(ai: AiState) {
    spawn(async move {
        if ai::focus_cindy() {
            ai::mark_seen(ai.snapshot, ai.watermark);
        }
    });
}

fn battery_panel(batteries: Signal<Vec<PeripheralBatteryDevice>>, dark: Signal<bool>   ) -> impl IntoElement {
    view()
        .flex_row()
        .items_center()
        .gap(px(6.0))
        .px(px(14.0))
        .pb(px(6.0))
        .child(
            text("🔋")
                .text_size(px(12.0))
                .text_color(move || palette(dark.get()).muted),
        )
        .child(
            text(move || {
                batteries
                    .get()
                    .iter()
                    .take(4)
                    .map(|device| format!("{} {}%", device.name, device.level))
                    .collect::<Vec<_>>()
                    .join(" · ")
            })
            .flex_1()
            .truncate()
            .text_size(px(10.5))
            .text_color(move || palette(dark.get()).muted),
        )
}

fn wechat_panel(
    wechat: Signal<(bool, Option<u64>)>,
    enabled: Signal<bool>,
    dark: Signal<bool>,
) -> impl IntoElement {
    view()
        .flex_row()
        .items_center()
        .gap(px(6.0))
        .px(px(14.0))
        .pb(px(6.0))
        .child(
            text("💬")
                .text_size(px(12.0))
                .text_color(move || palette(dark.get()).accent),
        )
        .child(
            text(move || t!("island.wechat"))
                .text_size(px(11.0))
                .text_color(move || palette(dark.get()).fg),
        )
        .child(
            text(move || {
                if !enabled.get() {
                    t!("island.wechat_disabled")
                } else {
                    let (available, pending) = wechat.get();
                    if available && pending.is_some() {
                        t!("island.wechat_pending")
                    } else if available {
                        t!("island.wechat_listening")
                    } else {
                        t!("island.wechat_missing")
                    }
                }
            })
            .flex_1()
            .text_size(px(10.0))
            .text_color(move || palette(dark.get()).muted),
        )
        .child(when(
            move || enabled.get() && wechat.get().0,
            move || {
                text(move || t!("island.wechat_open"))
                    .text_size(px(10.0))
                    .text_color(move || palette(dark.get()).accent)
                    .on_press(wechat_request_focus)
            },
        ))
}

fn wechat_toggle_row(state: SettingsState   ) -> impl IntoElement {
    view()
        .flex_row()
        .items_center()
        .gap(px(8.0))
        .child(
            text(move || t!("island.wechat"))
                .flex_1()
                .text_size(px(13.0))
                .text_color(move || palette(state.dark.get()).muted),
        )
        .child(pill(
            move || t!("island.on"),
            Rc::new(move || state.wechat_enabled.get()),
            move || {
                state.wechat_enabled.set(true);
                wechat_set_enabled(true);
                save_settings(state);
            },
            state.dark,
        ))
        .child(pill(
            move || t!("island.off"),
            Rc::new(move || !state.wechat_enabled.get()),
            move || {
                state.wechat_enabled.set(false);
                wechat_set_enabled(false);
                save_settings(state);
            },
            state.dark,
        ))
}

fn qq_toggle_row(state: SettingsState) -> impl IntoElement {
    view()
        .flex_row()
        .items_center()
        .gap(px(8.0))
        .child(
            text(move || t!("island.qq_title"))
                .flex_1()
                .text_size(px(13.0))
                .text_color(move || palette(state.dark.get()).muted),
        )
        .child(pill(
            move || t!("island.on"),
            Rc::new(move || state.qq_enabled.get()),
            move || {
                state.qq_enabled.set(true);
                save_settings(state);
            },
            state.dark,
        ))
        .child(pill(
            move || t!("island.off"),
            Rc::new(move || !state.qq_enabled.get()),
            move || {
                state.qq_enabled.set(false);
                save_settings(state);
            },
            state.dark,
        ))
}
fn media_panel(media_state: Signal<Option<MediaState>>, dark: Signal<bool>   ) -> impl IntoElement {
    view()
        .flex_col()
        .gap(px(3.0))
        .px(px(14.0))
        .pb(px(5.0))
        .child(
            view()
                .flex_row()
                .items_center()
                .gap(px(6.0))
                .child(
                    text("♪")
                        .text_size(px(12.0))
                        .text_color(move || media_color(media_state, dark)),
                )
                .child(
                    image(move || {
                        media_state
                            .get()
                            .and_then(|media| media.thumbnail)
                            .unwrap_or_default()
                    })
                    .w(px(30.0))
                    .h(px(30.0))
                    .rounded(px(4.0)),
                )
                .child(media_control("⏮", media_state, dark, media::previous))
                .child(media_play_control(media_state, dark))
                .child(media_control("⏭", media_state, dark, media::next))
                .child(media_shuffle(media_state, dark))
                .child(media_repeat(media_state, dark))
                .child(
                    text(move || {
                        let state = media_state.get();
                        let title = state
                            .as_ref()
                            .and_then(|media| media.title.clone())
                            .unwrap_or_else(|| t!("island.media_unknown"));
                        let artist = state
                            .as_ref()
                            .and_then(|media| media.artist.clone())
                            .unwrap_or_default();
                        if artist.is_empty() {
                            title
                        } else {
                            format!("{title} — {artist}")
                        }
                    })
                    .flex_1()
                    .truncate()
                    .text_size(px(11.5))
                    .text_color(move || palette(dark.get()).fg),
                )
                .child(
                    text(move || {
                        let peak = media_state
                            .get()
                            .as_ref()
                            .map(|media| media.audio_peak)
                            .unwrap_or(0.0);
                        if peak > 0.12 { "▇" } else if peak > 0.04 { "▅" } else if peak > 0.01 { "▃" } else { "▁" }
                    })
                    .text_size(px(12.0))
                    .text_color(move || media_color(media_state, dark)),
                )
                .child(media_volume_control("−", -0.1, media_state, dark))
                .child(
                    text(move || {
                        let volume = media_state
                            .get()
                            .as_ref()
                            .map(|media| media.volume)
                            .unwrap_or(0.0);
                        format!("{:.0}%", volume * 100.0)
                    })
                    .text_size(px(10.0))
                    .text_color(move || palette(dark.get()).muted),
                )
                .child(media_volume_control("+", 0.1, media_state, dark)),
        )
        .child(when(
            move || media_state.get().as_ref().is_some_and(|media| media.duration_ms > 0),
            move || media_progress(media_state, dark),
        ))
}

const PROGRESS_WIDTH: f32 = 240.0;

#[derive(Clone, Copy, PartialEq)]
struct DragSeek {
    start_x: f32,
    current_x: f32,
}

fn unix_millis() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |time| time.as_millis() as i64)
}

fn media_position(media: &MediaState, drag: Option<DragSeek>, now: i64) -> i64 {
    let duration = media.duration_ms.max(0);
    if duration == 0 {
        return 0;
    }
    if let Some(drag) = drag {
        return ((drag.current_x.clamp(0.0, PROGRESS_WIDTH) / PROGRESS_WIDTH) as f64 * duration as f64).round() as i64;
    }
    let moved = if media.playback_status == "playing" { (now - media.updated_at).max(0) } else { 0 };
    media.position_ms.saturating_add(moved).clamp(0, duration)
}

fn media_fraction(media: &MediaState, drag: Option<DragSeek>, now: i64) -> f32 {
    if media.duration_ms <= 0 {
        0.0
    } else {
        (media_position(media, drag, now) as f32 / media.duration_ms as f32).clamp(0.0, 1.0)
    }
}

fn seek_progress(media_state: Signal<Option<MediaState>>, x: f32) {
    let Some(media) = media_state.get_untracked() else {
        return;
    };
    if !media.can_seek || media.duration_ms <= 0 {
        return;
    }
    let duration = media.duration_ms;
    let target = ((x.clamp(0.0, PROGRESS_WIDTH) / PROGRESS_WIDTH) as f64 * duration as f64).round() as i64;
    let target = target.clamp(0, duration);
    spawn(async move {
        if let Ok(next) = media::seek(target) {
            media_state.set(Some(next));
        }
    });
}

fn media_progress(
    media_state: Signal<Option<MediaState>>,
    dark: Signal<bool>,
) -> impl IntoElement {
    let drag = signal(None::<DragSeek>);
    let now = signal(unix_millis());
    set_interval(Duration::from_millis(500), move || now.set(unix_millis()));
    view()
        .flex_row()
        .items_center()
        .gap(px(6.0))
        .child(
            view()
                .relative()
                .w(px(PROGRESS_WIDTH))
                .h(px(14.0))
                .items_center()
                .child(
                    view()
                        .relative()
                        .w(px(PROGRESS_WIDTH))
                        .h(px(4.0))
                        .rounded(px(2.0))
                        .bg(move || palette(dark.get()).border)
                        .child(
                            view()
                                .h(px(4.0))
                                .rounded(px(2.0))
                                .bg(move || media_color(media_state, dark))
                                .w(move || {
                                    let fraction = media_state
                                        .get()
                                        .as_ref()
                                        .map_or(0.0, |media| media_fraction(media, drag.get(), now.get()));
                                    px(fraction * PROGRESS_WIDTH)
                                }),
                        )
                        .child(
                            view()
                                .absolute()
                                .top(px(-1.0))
                                .left(move || {
                                    let fraction = media_state
                                        .get()
                                        .as_ref()
                                        .map_or(0.0, |media| media_fraction(media, drag.get(), now.get()));
                                    px(fraction * PROGRESS_WIDTH - 3.0)
                                })
                                .w(px(6.0))
                                .h(px(6.0))
                                .rounded(px(3.0))
                                .bg(move || media_color(media_state, dark)),
                        ),
                )
                .on_press_down(move |event: &nsh_core::PressEvent| {
                    if !media_state
                        .get_untracked()
                        .is_some_and(|media| media.can_seek && media.duration_ms > 0)
                    {
                        return;
                    }
                    let x = event.position.x.0.clamp(0.0, PROGRESS_WIDTH);
                    drag.set(Some(DragSeek { start_x: x, current_x: x }));
                })
                .on_mouse_move(move |event: &nsh_core::MouseEvent| {
                    let Some(mut current) = drag.get_untracked() else {
                        return;
                    };
                    if event.pressed_button.is_none() {
                        return;
                    }
                    current.current_x = event.position.x.0.clamp(0.0, PROGRESS_WIDTH);
                    drag.set(Some(current));
                })
                .on_mouse_up(move |event: &nsh_core::MouseEvent| {
                    let Some(mut current) = drag.get_untracked() else {
                        return;
                    };
                    current.current_x = event.position.x.0.clamp(0.0, PROGRESS_WIDTH);
                    drag.set(None);
                    seek_progress(media_state, current.current_x);
                })
                .on_pan_start(move |_event: &nsh_core::element::PanEvent| {
                    if drag.get_untracked().is_none() {
                        drag.set(Some(DragSeek { start_x: 0.0, current_x: 0.0 }));
                    }
                })
                .on_pan_end(move |event: &nsh_core::element::PanEvent| {
                    let Some(current) = drag.get_untracked() else {
                        return;
                    };
                    drag.set(None);
                    if event.cancelled {
                        return;
                    }
                    seek_progress(media_state, current.start_x + event.translation.x.0);
                }),
        )
        .child(
            text(move || {
                let state = media_state.get();
                let position = state
                    .as_ref()
                    .map_or(0, |media| media_position(media, drag.get(), now.get()));
                let duration = state.as_ref().map_or(0, |media| media.duration_ms);
                format!("{} / {}", media_time(position), media_time(duration))
            })
            .text_size(px(9.5))
            .text_color(move || palette(dark.get()).faint),
        )
}

fn media_idle(media_state: Signal<Option<MediaState>>, dark: Signal<bool>) -> impl IntoElement {
    view()
        .flex_row()
        .items_center()
        .gap(px(8.0))
        .px(px(14.0))
        .pb(px(5.0))
        .child(
            view()
                .w(px(28.0))
                .h(px(28.0))
                .rounded(px(14.0))
                .items_center()
                .justify_center()
                .bg(move || palette(dark.get()).row)
                .border(px(1.0))
                .border_color(move || palette(dark.get()).border)
                .child(
                    view()
                        .w(px(10.0))
                        .h(px(10.0))
                        .rounded(px(5.0))
                        .items_center()
                        .justify_center()
                        .bg(move || palette(dark.get()).input)
                        .child(view().w(px(2.0)).h(px(2.0)).rounded(px(1.0)).bg(move || palette(dark.get()).faint)),
                ),
        )
        .child(
            view()
                .flex_col()
                .child(
                    text(move || t!("island.media_idle"))
                        .text_size(px(12.0))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(move || palette(dark.get()).fg),
                )
                .child(
                    text(move || t!("island.media_idle_hint"))
                        .truncate()
                        .text_size(px(9.5))
                        .text_color(move || palette(dark.get()).faint),
                ),
        )
        .child(view().flex_1())
        .child(
            text(move || format!("▶ {}", t!("island.media_resume")))
                .px(px(8.0))
                .py(px(3.0))
                .rounded(px(6.0))
                .bg(move || palette(dark.get()).input)
                .text_size(px(10.0))
                .text_color(move || palette(dark.get()).muted)
                .on_press(move || {
                    spawn(async move {
                        if let Ok(next) = media::play_pause() {
                            media_state.set(Some(next));
                        }
                    });
                }),
        )
}

fn media_time(ms: i64) -> String {
    let total = (ms.max(0) / 1000) as u32;
    format!("{}:{:02}", total / 60, total % 60)
}

fn media_color(media_state: Signal<Option<MediaState>>, dark: Signal<bool>) -> Color {
    media_state
        .get_untracked()
        .and_then(|media| media.tint)
        .and_then(|tint| u32::from_str_radix(tint.trim_start_matches('#'), 16).ok())
        .map(|value| rgb(value).into())
        .unwrap_or_else(|| palette(dark.get_untracked()).accent)
}

fn media_shuffle(media_state: Signal<Option<MediaState>>, dark: Signal<bool>) -> impl IntoElement {
    text("🔀")
        .text_size(px(12.0))
        .text_color(move || {
            if media_state
                .get()
                .as_ref()
                .is_some_and(|media| media.shuffle_active == Some(true))
            {
                media_color(media_state, dark)
            } else {
                palette(dark.get()).faint
            }
        })
        .px(px(3.0))
        .on_press(move || {
            spawn(async move {
                if let Ok(next) = media::toggle_shuffle() {
                    media_state.set(Some(next));
                }
            });
        })
}

fn media_repeat(media_state: Signal<Option<MediaState>>, dark: Signal<bool>) -> impl IntoElement {
    text(move || {
        match media_state
            .get()
            .as_ref()
            .map(|media| media.repeat_mode)
            .unwrap_or("unknown")
        {
            "track" => "🔂",
            "list" => "🔁",
            _ => "🔁",
        }
    })
    .text_size(px(12.0))
    .text_color(move || {
        if media_state
            .get()
            .as_ref()
            .is_some_and(|media| media.repeat_mode != "none" && media.repeat_mode != "unknown")
        {
            media_color(media_state, dark)
        } else {
            palette(dark.get()).faint
        }
    })
    .px(px(3.0))
    .on_press(move || {
        spawn(async move {
            if let Ok(next) = media::cycle_repeat() {
                media_state.set(Some(next));
            }
        });
    })
}

fn media_control(
    label: &'static str,
    media_state: Signal<Option<MediaState>>,
    dark: Signal<bool>,
    command: fn() -> Result<MediaState, String>,
) -> impl IntoElement {
    text(label)
        .text_size(px(14.0))
        .text_color(move || palette(dark.get()).muted)
        .px(px(4.0))
        .on_press(move || {
            spawn(async move {
                if let Ok(next) = command() {
                    media_state.set(Some(next));
                }
            });
        })
}

fn media_play_control(
    media_state: Signal<Option<MediaState>>,
    dark: Signal<bool>,
) -> impl IntoElement {
    text(move || {
        if media_state
            .get()
            .as_ref()
            .is_some_and(|media| media.playback_status == "playing")
        {
            "⏸"
        } else {
            "▶"
        }
    })
    .text_size(px(14.0))
    .text_color(move || palette(dark.get()).muted)
    .px(px(4.0))
    .on_press(move || {
        spawn(async move {
            if let Ok(next) = media::play_pause() {
                media_state.set(Some(next));
            }
        });
    })
}

fn media_volume_control(
    label: &'static str,
    delta: f32,
    media_state: Signal<Option<MediaState>>,
    dark: Signal<bool>,
) -> impl IntoElement {
    text(label)
        .text_size(px(13.0))
        .text_color(move || palette(dark.get()).muted)
        .px(px(4.0))
        .on_press(move || {
            let level = media_state
                .get_untracked()
                .as_ref()
                .map(|media| media.volume)
                .unwrap_or(0.5);
            spawn(async move {
                if let Ok(next) = media::set_volume((level + delta).clamp(0.0, 1.0)) {
                    media_state.set(Some(next));
                }
            });
        })
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
        .gap(px(theme::SPACE_LG))
        .pt(px(theme::PANEL_PADDING))
        .pb(px(18.0))
        // `.clipboard-search`: a 32px field, white 6% fill, focus ring soft.
        .child(
            view()
                .flex_row()
                .items_center()
                .gap(px(theme::SPACE_MD))
                .h(px(32.0))
                .px(px(9.0))
                .rounded(px(theme::CARD_RADIUS))
                .bg(theme::WHITE_A06)
                .text_color(theme::TEXT_3)
                .child(widgets::icon(icons::SEARCH, 14.0))
                .child(
                    input()
                        .flex_1()
                        .value(move || query.get())
                        .placeholder(move || t!("island.search"))
                        .placeholder_color(theme::TEXT_PLACEHOLDER)
                        .text_color(theme::TEXT_1)
                        .text_size(px(theme::BODY_SIZE))
                        .on_focus(move || focus::set_input_focused(true))
                        .on_blur(move || focus::set_input_focused(false))
                        .on_change(move |text: &str| {
                            query.set(text.to_string());
                        }),
                )
                .child(when(move || !query.get().is_empty(), move || {
                    view()
                        .w(px(24.0))
                        .h(px(24.0))
                        .flex_shrink(0.0)
                        .items_center()
                        .justify_center()
                        .rounded(px(12.0))
                        .text_color(theme::TEXT_2)
                        .transition()
                        .duration(140.0)
                        .hover(|s| s.bg(theme::WHITE_A10).text_color(white()))
                        .on_press(move || query.set(String::new()))
                        .child(widgets::icon(icons::X, 13.0))
                })),
        )
        .child(
            scroll()
                .flex_1()
                .scrollbar(false)
                .pr(px(4.0))
                .child(when(
                    move || visible.get().is_empty() && !query.get().is_empty(),
                    move || no_results(dark),
                ))
                .child(when(move || items.get().is_empty(), move || empty_state(dark)))
                .child(each(visible, |item| item.id, move |item, _| {
                    history_row(item, items)
                })),
        )
}

/// One `.clipboard-item`: 44px tall, a 30px lead, a one-line preview, and the
/// actions that matter (a star when favorited, copy on hover), over a hairline.
fn history_row(item: ReadSignal<ClipItem>, items: Signal<Vec<ClipItem>>) -> impl IntoElement {
    let snapshot = item.get();
    let copy = snapshot.clone();
    let id = snapshot.id;
    let lead: AnyElement = if snapshot.kind == ClipKind::Image {
        match snapshot.image_url.clone() {
            Some(url) => view()
                .w(px(30.0))
                .h(px(30.0))
                .flex_shrink(0.0)
                .overflow_hidden()
                .rounded(px(6.0))
                .bg(theme::WHITE_A04)
                .child(image(url).w(px(30.0)).h(px(30.0)).object_fit(ResizeMode::Cover))
                .into_element(),
            None => widgets::icon_chip(icons::IMAGE, 30.0, theme::TEXT_2, theme::WHITE_A04)
                .rounded(px(6.0))
                .into_element(),
        }
    } else {
        view()
            .w(px(30.0))
            .h(px(30.0))
            .flex_shrink(0.0)
            .items_center()
            .justify_center()
            .text_color(theme::TEXT_2)
            .child(widgets::icon(icons::CLIPBOARD, 14.0))
            .into_element()
    };
    view()
        .group()
        .flex_row()
        .items_center()
        .gap(px(theme::SPACE_MD))
        .h(px(44.0))
        .px(px(theme::SPACE_MD))
        .border_b(px(1.0))
        .border_b_color(theme::WHITE_A08)
        .transition()
        .duration(150.0)
        .hover(|s| s.bg(theme::WHITE_A04))
        .child(
            view()
                .flex_1()
                .min_w(px(0.0))
                .h_full()
                .flex_row()
                .items_center()
                .gap(px(theme::SPACE_MD))
                .on_press(move || match copy_target(&copy) {
                    CopyTarget::Text(text) => {
                        let _ = set_clipboard(text);
                    }
                    CopyTarget::Image(url) => {
                        if !clipboard::write_image(&url) {
                            set_clipboard_image(url);
                        }
                    }
                    CopyTarget::None => {}
                })
                .child(lead)
                .child(
                    text(snapshot.text.chars().take(160).collect::<String>())
                        .truncate()
                        .text_size(px(theme::BODY_SIZE))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(theme::TEXT_1),
                ),
        )
        // At rest a row shows only what it holds; copy appears on hover, as the
        // Tauri row does.
        .child(
            view()
                .w(px(26.0))
                .h(px(26.0))
                .flex_shrink(0.0)
                .items_center()
                .justify_center()
                .rounded(px(13.0))
                .text_color(theme::TEXT_3)
                .opacity(0.0)
                .transition()
                .duration(150.0)
                .group_hover(|s| s.opacity(1.0))
                .hover(|s| s.bg(theme::WHITE_A16).text_color(white()))
                .on_press(move || match copy_target(&snapshot) {
                    CopyTarget::Text(text) => {
                        let _ = set_clipboard(text);
                    }
                    CopyTarget::Image(url) => {
                        if !clipboard::write_image(&url) {
                            set_clipboard_image(url);
                        }
                    }
                    CopyTarget::None => {}
                })
                .child(widgets::icon(icons::COPY, 14.0)),
        )
        .child(when(move || item.get().favorite, move || {
            view()
                .w(px(26.0))
                .h(px(26.0))
                .flex_shrink(0.0)
                .items_center()
                .justify_center()
                .rounded(px(13.0))
                .text_color(theme::TEXT_1)
                .hover(|s| s.bg(theme::WHITE_A16))
                .on_press(move || clipboard::toggle_favorite(items, id))
                .child(widgets::icon(icons::STAR, 14.0))
        }))
        .child(when(move || !item.get().favorite, move || {
            view()
                .w(px(26.0))
                .h(px(26.0))
                .flex_shrink(0.0)
                .items_center()
                .justify_center()
                .rounded(px(13.0))
                .text_color(theme::TEXT_3)
                .opacity(0.0)
                .transition()
                .duration(150.0)
                .group_hover(|s| s.opacity(1.0))
                .hover(|s| s.bg(theme::WHITE_A16).text_color(white()))
                .on_press(move || clipboard::toggle_favorite(items, id))
                .child(widgets::icon(icons::STAR, 14.0))
        }))
}
fn settings_window(
    open: Signal<bool>,
    state: SettingsState,
    items: Signal<Vec<ClipItem>>,
    media_state: Signal<Option<MediaState>>,
) -> impl IntoElement {
    nsh_core::element::desktop::window()
        .id("settings")
        .kind(nsh_core::desktop::WindowKind::Normal)
        .title(t!("island.settings"))
        .default_size(size(px(520.0), px(820.0)))
        .open(open)
        .on_open_change(move |event| open.set(event.open))
        .content(move || settings_content(open, state, items, media_state))
}

fn settings_content(
    open: Signal<bool>,
    state: SettingsState,
    items: Signal<Vec<ClipItem>>,
    media_state: Signal<Option<MediaState>>,
   ) -> impl IntoElement {
    scroll()
        .flex_1()
        .bg(move || palette(state.dark.get()).bg)
        .child(
            view()
                .flex_col()
                .gap(px(14.0))
                .p(px(20.0))
                .text_color(move || palette(state.dark.get()).fg)
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
        .child(section_label(move || t!("island.messages"), state))
        .child(qq_toggle_row(state))
        .child(mail_ui::settings_section(state.mail, state.dark))
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
        .child(section_label(move || t!("island.ai"), state))
        .child(toggle_row(move || t!("island.ai_enabled"), state.ai.enabled, state))
        .child(toggle_row(move || t!("island.notify_done"), state.ai.kinds[0], state))
        .child(toggle_row(move || t!("island.notify_question"), state.ai.kinds[1], state))
        .child(toggle_row(move || t!("island.notify_scheduled"), state.ai.kinds[2], state))
        .child(toggle_row(move || t!("island.notify_error"), state.ai.kinds[3], state))
        .child(toggle_row(move || t!("island.sound"), state.sound_enabled, state))
        .child(
            text(move || ai_source_label(state))
                .text_size(px(10.0))
                .text_color(move || palette(state.dark.get()).faint),
        )
        .child(wechat_toggle_row(state))
        .child(music_sources_section(state, media_state))
        .child(shelf_settings(state, state.dark))
        .child(update_settings(state, state.dark))
        .child(translation_ui::settings_section(state.translation, state.dark))
        .child(launcher_ui::settings_section(state.launcher, state.translation.shortcut, state.dark))
        .child(section_label(move || t!("island.screenshot"), state))
        .child(screenshot_ui::settings_section(state.screenshot, state.dark))
        .child(section_label(move || t!("island.lyrics"), state))
        .child(lyrics_folder_row(state))
        .child(toggle_row(move || t!("island.lyrics_online"), state.lyrics.online, state))
        .child(toggle_row(move || t!("island.lyrics_chinese"), state.lyrics.chinese, state))
        .child(view().flex_1())
        .child(
            text(move || t!("island.about", version = "0.1.0"))
                .text_size(px(11.0))
                .text_color(move || palette(state.dark.get()).faint),
        )
        )
}

fn section_label(label: impl Fn() -> String + 'static, state: SettingsState) -> impl IntoElement {
    text(label)
        .text_size(px(12.0))
        .text_color(move || palette(state.dark.get()).muted)
        .mt(px(6.0))
}

fn toggle_row(
    label: impl Fn() -> String + 'static,
    value: Signal<bool>,
    state: SettingsState,
) -> impl IntoElement {
    view()
        .flex_row()
        .items_center()
        .gap(px(8.0))
        .child(
            text(label)
                .flex_1()
                .text_size(px(13.0))
                .text_color(move || palette(state.dark.get()).muted),
        )
        .child(pill(
            move || t!("island.on"),
            Rc::new(move || value.get()),
            move || {
                value.set(true);
                save_settings(state);
            },
            state.dark,
        ))
        .child(pill(
            move || t!("island.off"),
            Rc::new(move || !value.get()),
            move || {
                value.set(false);
                save_settings(state);
            },
            state.dark,
        ))
}

fn music_sources_section(state: SettingsState, media_state: Signal<Option<MediaState>>) -> impl IntoElement {
    view()
        .flex_col()
        .gap(px(4.0))
        .child(section_label(move || t!("island.music_sources"), state))
        .child(
            text(move || t!("island.music_sources_hint"))
                .text_size(px(10.0))
                .text_color(move || palette(state.dark.get()).faint),
        )
        .child(when(move || state.music_sources.get().is_empty(), move || {
            text(move || t!("island.music_sources_empty"))
                .text_size(px(11.0))
                .text_color(move || palette(state.dark.get()).faint)
        }))
        .child(each(state.music_sources, |source| source.id.clone(), move |source, index| {
            music_source_row(state, media_state, source, index)
        }))
}

fn music_source_row(
    state: SettingsState,
    media_state: Signal<Option<MediaState>>,
    source: ReadSignal<media::KnownSource>,
    index: ReadSignal<usize>,
) -> impl IntoElement {
    let id = source.get().id;
    let playing_id = id.clone();
    let open_id = id.clone();
    let move_up = move || {
        let mut list = state.music_sources.get_untracked();
        let from = index.get_untracked();
        if from > 0 && from < list.len() {
            list.swap(from, from - 1);
            state.music_sources.set(list);
            save_settings(state);
        }
    };
    let move_down = move || {
        let mut list = state.music_sources.get_untracked();
        let from = index.get_untracked();
        if from + 1 < list.len() {
            list.swap(from, from + 1);
            state.music_sources.set(list);
            save_settings(state);
        }
    };
    view()
        .flex_row()
        .items_center()
        .gap(px(6.0))
        .px(px(6.0))
        .py(px(3.0))
        .rounded(px(6.0))
        .bg(move || palette(state.dark.get()).row)
        .child(
            view()
                .w(px(6.0))
                .h(px(6.0))
                .rounded(px(3.0))
                .bg(move || {
                    let media = media_state.get();
                    let playing = media.as_ref().is_some_and(|media| media.sessions.iter().any(|session| session.id == playing_id && session.playing));
                    let open = media.as_ref().is_some_and(|media| media.sessions.iter().any(|session| session.id == open_id));
                    if playing {
                        palette(state.dark.get()).accent
                    } else if open {
                        palette(state.dark.get()).muted
                    } else {
                        palette(state.dark.get()).faint
                    }
                }),
        )
        .child(
            text(move || source.get().label)
                .flex_1()
                .truncate()
                .text_size(px(12.0))
                .text_color(move || palette(state.dark.get()).fg),
        )
        .child(
            text(move || {
                let media = media_state.get();
                let playing = media.as_ref().is_some_and(|media| media.sessions.iter().any(|session| session.id == id && session.playing));
                let open = media.as_ref().is_some_and(|media| media.sessions.iter().any(|session| session.id == id));
                if playing {
                    t!("island.music_source_playing")
                } else if open {
                    t!("island.music_source_open")
                } else {
                    t!("island.music_source_closed")
                }
            })
            .text_size(px(9.0))
            .text_color(move || palette(state.dark.get()).faint),
        )
        .child(
            text("↑")
                .px(px(5.0))
                .py(px(1.0))
                .rounded(px(4.0))
                .bg(move || palette(state.dark.get()).input)
                .text_size(px(10.0))
                .text_color(move || palette(state.dark.get()).muted)
                .on_press(move_up),
        )
        .child(
            text("↓")
                .px(px(5.0))
                .py(px(1.0))
                .rounded(px(4.0))
                .bg(move || palette(state.dark.get()).input)
                .text_size(px(10.0))
                .text_color(move || palette(state.dark.get()).muted)
                .on_press(move_down),
        )
}
fn update_settings(state: SettingsState, dark: Signal<bool>) -> impl IntoElement {
    view()
        .flex_col()
        .gap(px(7.0))
        .child(section_label(move || t!("island.update_title"), state))
        .child(
            input()
                .value(move || state.update.endpoint.get())
                .placeholder(move || t!("island.update_endpoint_placeholder"))
                .placeholder_color(move || palette(dark.get()).faint)
                .text_color(move || palette(dark.get()).fg)
                .bg(move || palette(dark.get()).input)
                .h(px(30.0))
                .px(px(10.0))
                .rounded(px(8.0))
                .on_change(move |text: &str| {
                    state.update.endpoint.set(text.to_string());
                    save_settings(state);
                }),
        )
        .child(
            view()
                .flex_row()
                .gap(px(8.0))
                .child(pill(move || t!("island.update_check"), Rc::new(|| false), move || update_ui::check_now(state.update), dark))
                .child(when(move || state.update.available.get().is_some(), move || {
                    pill(move || t!("island.update_install"), Rc::new(|| false), move || update_ui::install_now(state.update), dark)
                })),
        )
        .child(when(move || !state.update.status.get().is_empty(), move || {
            text(move || state.update.status.get()).text_size(px(11.0)).text_color(move || update_ui::status_color(state.update, dark))
        }))
        .child(when(move || !state.update.error.get().is_empty(), move || {
            text(move || state.update.error.get()).text_size(px(11.0)).text_color(move || palette(dark.get()).warning)
        }))
        .child(when(move || state.update.available.get().is_some(), move || {
            view().flex_col().gap(px(3.0))
                .child(text(move || state.update.available.get().and_then(|item| item.notes).unwrap_or_default()).text_size(px(10.0)).text_color(move || palette(dark.get()).muted))
                .child(text(move || state.update.available.get().map(|item| update_ui::signature_note(&item)).unwrap_or_default()).text_size(px(10.0)).text_color(move || palette(dark.get()).faint))
        }))
}

fn shelf_settings(state: SettingsState, dark: Signal<bool>) -> impl IntoElement {
    view()
        .flex_col()
        .gap(px(8.0))
        .child(section_label(move || t!("island.shelf_title"), state))
        .child(
            view()
                .flex_row()
                .items_center()
                .gap(px(8.0))
                .child(text(move || t!("island.shelf_max")).flex_1().text_size(px(13.0)).text_color(move || palette(dark.get()).muted))
                .child(
                    input()
                        .w(px(70.0))
                        .h(px(28.0))
                        .px(px(8.0))
                        .rounded(px(7.0))
                        .bg(move || palette(dark.get()).input)
                        .text_color(move || palette(dark.get()).fg)
                        .value(move || state.shelf.max_items.get().to_string())
                        .on_change(move |text: &str| {
                            if let Ok(value) = text.trim().parse::<usize>() {
                                state.shelf.max_items.set(value.clamp(file_shelf::MIN_MAX_ITEMS, file_shelf::MAX_MAX_ITEMS));
                                file_shelf_ui::refresh(state.shelf);
                                save_settings(state);
                            }
                        }),
                ),
        )
        .child(
            view()
                .flex_row()
                .items_center()
                .gap(px(8.0))
                .child(text(move || t!("island.shelf_double_click")).flex_1().text_size(px(13.0)).text_color(move || palette(dark.get()).muted))
                .child(pill(move || t!("island.shelf_open"), Rc::new(move || state.shelf.double_click.get() == "open"), move || { state.shelf.double_click.set("open".into()); save_settings(state); }, dark))
                .child(pill(move || t!("island.shelf_reveal"), Rc::new(move || state.shelf.double_click.get() == "reveal"), move || { state.shelf.double_click.set("reveal".into()); save_settings(state); }, dark)),
        )
        .child(
            view()
                .flex_row()
                .items_center()
                .gap(px(8.0))
                .child(text(move || t!("island.shelf_show_count")).flex_1().text_size(px(13.0)).text_color(move || palette(dark.get()).muted))
                .child(pill(move || t!("island.on"), Rc::new(move || state.shelf_show_count.get()), move || { state.shelf_show_count.set(true); save_settings(state); }, dark))
                .child(pill(move || t!("island.off"), Rc::new(move || !state.shelf_show_count.get()), move || { state.shelf_show_count.set(false); save_settings(state); }, dark)),
        )
        .child(pill(move || t!("island.shelf_clear"), Rc::new(|| false), move || file_shelf_ui::clear(state.shelf), dark))
}

fn lyrics_folder_row(state: SettingsState) -> impl IntoElement {
    view()
        .flex_col()
        .gap(px(4.0))
        .child(
            text(move || t!("island.lyrics_folder"))
                .text_size(px(13.0))
                .text_color(move || palette(state.dark.get()).muted),
        )
        .child(
            input()
                .value(move || state.lyrics.folder.get())
                .placeholder(move || t!("island.lyrics_folder_placeholder"))
                .placeholder_color(move || palette(state.dark.get()).faint)
                .text_color(move || palette(state.dark.get()).fg)
                .bg(move || palette(state.dark.get()).input)
                .h(px(30.0))
                .px(px(10.0))
                .rounded(px(8.0))
                .on_change(move |text: &str| {
                    state.lyrics.folder.set(text.to_string());
                    save_settings(state);
                }),
        )
}

fn ai_source_label(state: SettingsState) -> String {
    let snapshot = state.ai.snapshot.get();
    if let Some(error) = snapshot.error.as_deref() {
        return t!("island.ai_source_failed", error = error);
    }
    if snapshot.database_path.is_empty() {
        return t!("island.ai_source_missing");
    }
    t!("island.ai_source_ready", count = snapshot.sessions.len())
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

fn empty_state(dark: Signal<bool>   ) -> impl IntoElement {
    view().items_center().justify_center().py(px(36.0)).child(
        text(move || t!("island.empty"))
            .text_size(px(12.0))
            .text_color(move || palette(dark.get()).faint),
    )
}

fn no_results(dark: Signal<bool>   ) -> impl IntoElement {
    view().items_center().justify_center().py(px(36.0)).child(
        text(move || t!("island.no_results"))
            .text_size(px(12.0))
            .text_color(move || palette(dark.get()).faint),
    )
}

fn save_settings(state: SettingsState) {
    preferences::save(&preferences::Preferences {
        dark: state.dark.get_untracked(),
        locale: locale().as_str().to_string(),
        clipboard_enabled: state.clipboard_enabled.get_untracked(),
        capture_images: state.capture_images.get_untracked(),
        max_items: state.max_items.get_untracked(),
        ai_enabled: state.ai.enabled.get_untracked(),
        notify_done: state.ai.kinds[0].get_untracked(),
        notify_question: state.ai.kinds[1].get_untracked(),
        notify_scheduled: state.ai.kinds[2].get_untracked(),
        notify_error: state.ai.kinds[3].get_untracked(),
        wechat_enabled: state.wechat_enabled.get_untracked(),
        sound_enabled: state.sound_enabled.get_untracked(),
        lyrics_folder: state.lyrics.folder.get_untracked(),
        lyrics_online: state.lyrics.online.get_untracked(),
        lyrics_chinese: state.lyrics.chinese.get_untracked(),
        music_sources: state.music_sources.get_untracked(),
        translation_enabled: state.translation.enabled.get_untracked(),
        translation_shortcut: state.translation.shortcut.get_untracked(),
        translation_direction: state.translation.direction.get_untracked(),
        translation_provider: state.translation.provider.get_untracked(),
        translation_custom_base_url: state.translation.custom_base_url.get_untracked(),
        translation_auto: state.translation.auto.get_untracked(),
        translation_models: state.translation.models.get_untracked(),
        launcher_enabled: state.launcher.enabled.get_untracked(),
        launcher_shortcut: state.launcher.shortcut.get_untracked(),
        launcher_search_apps: state.launcher.search_apps.get_untracked(),
        launcher_search_files: state.launcher.search_files.get_untracked(),
        launcher_search_web: state.launcher.search_web.get_untracked(),
        launcher_engine: state.launcher.engine.get_untracked(),
        launcher_folder_mode: state.launcher.folder_mode.get_untracked(),
        launcher_folders: state.launcher.folders.get_untracked(),
        shelf_max_items: state.shelf.max_items.get_untracked(),
        shelf_show_count: state.shelf_show_count.get_untracked(),
        shelf_double_click: state.shelf.double_click.get_untracked(),
        update_endpoint: state.update.endpoint.get_untracked(),
        qq_enabled: state.qq_enabled.get_untracked(),
        mail_accounts: state.mail.accounts.get_untracked(),
    });
}

#[cfg(test)]
mod rust_only_tests {
    #[test]
    fn the_expanded_state_is_larger_than_the_collapsed_state() {
        let collapsed = std::hint::black_box((super::COLLAPSED_WIDTH, super::COLLAPSED_HEIGHT));
        let expanded = std::hint::black_box((super::EXPANDED_WIDTH, super::EXPANDED_HEIGHT));
        assert!(collapsed.0 < expanded.0);
        assert!(collapsed.1 < expanded.1);
    }

    #[test]
    fn a_clipboard_row_copies_text_and_image_sources() {
        let text = super::ClipItem {
            id: 1,
            kind: super::ClipKind::Text,
            text: "hello".into(),
            image_url: None,
            width: 0,
            height: 0,
            favorite: false,
        };
        assert!(matches!(super::copy_target(&text), super::CopyTarget::Text(value) if value == "hello"));
        let image = super::ClipItem {
            id: 2,
            kind: super::ClipKind::Image,
            text: String::new(),
            image_url: Some("data:image/png;base64,AA==".into()),
            width: 1,
            height: 1,
            favorite: false,
        };
        assert!(matches!(super::copy_target(&image), super::CopyTarget::Image(_)));
        let missing = super::ClipItem { image_url: None, ..image };
        assert!(matches!(super::copy_target(&missing), super::CopyTarget::None));
    }
}
