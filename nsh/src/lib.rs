//! FocuSD Island on NSH: Phase 2 shell and clipboard pilot.
//!
//! The Win32 host (nsh/host) owns the top-level window and embeds this root
//! through AppView. The island keeps the capsule, adds a searchable
//! text/image clipboard with favorites, and a small settings page.

mod ai;
mod battery;
mod media;
mod clipboard;
pub mod focus;
#[cfg(feature = "embedded")]
pub mod embedded;
mod preferences;
mod window;

use std::{rc::Rc, time::Duration};

use nsh_core::{element::input, prelude::*, Color};

use ai::{AgentSession, AgentState, AiSnapshot};
use battery::PeripheralBatteryDevice;
use clipboard::{ClipItem, ClipKind, ClipboardHistory};
use media::MediaState;

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
    running: Color,
    warning: Color,
    success: Color,
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
            running: rgb(0x5B9CF5).into(),
            warning: rgb(0xE5A84A).into(),
            success: rgb(0x4CB782).into(),
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
struct SettingsState {
    dark: Signal<bool>,
    clipboard_enabled: Signal<bool>,
    capture_images: Signal<bool>,
    max_items: Signal<usize>,
    ai: AiState,
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

    let media_state = signal(Option::<MediaState>::None);
    let read_media = Rc::new(move || {
        spawn(async move {
            if let Ok(next) = media::state() {
                media_state.set(Some(next));
            }
        });
    });
    read_media();
    let poll_media = read_media.clone();
    set_interval(Duration::from_millis(1500), move || poll_media());

    window::configure(expanded);
    window::install_tray(expanded);
    window::install_shortcut(expanded);
    #[cfg(feature = "embedded")]
    embedded::start(expanded, settings_open);

    view()
        .flex_1()
        .items_center()
        .child(when(expanded, move || {
            expanded_island(expanded, settings_open, query, items, visible, dark, ai_state, media_state, batteries)
        }))
        .child(when(move || !expanded.get(), move || {
            collapsed_island(expanded, items, dark, ai_state)
        }))
        .child(settings_window(settings_open, settings_state, items))
}

fn collapsed_island(
    expanded: Signal<bool>,
    items: Signal<Vec<ClipItem>>,
    dark: Signal<bool>,
    ai: AiState,
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
        .child(ai_dot(dark, ai))
        .child(
            text(move || t!("island.title"))
                .text_size(px(13.0))
                .font_weight(FontWeight::MEDIUM)
                .text_color(move || palette(dark.get()).fg),
        )
        .child(when(
            move || ai.enabled.get() && ai.snapshot.get().state().is_some(),
            move || {
                text(move || ai_inline_label(ai))
                    .text_size(px(11.0))
                    .text_color(move || ai_state_color(palette(dark.get()), ai.snapshot.get().state()))
            },
        ))
        .child(view().flex_1())
        .child(when(move || ai_unseen(ai) > 0, move || {
            view()
                .px(px(6.0))
                .py(px(1.0))
                .rounded(px(9.0))
                .bg(move || palette(dark.get()).warning)
                .child(
                    text(move || ai_unseen(ai).to_string())
                        .text_size(px(9.0))
                        .text_color(move || palette(dark.get()).bg),
                )
                .on_press(move || open_cindy(ai))
        }))
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
    ai: AiState,
    media_state: Signal<Option<MediaState>>,
    batteries: Signal<Vec<PeripheralBatteryDevice>>,
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
        .child(expanded_header(expanded, settings_open, items, dark, ai))
        .child(when(move || ai.enabled.get(), move || ai_panel(ai, dark)))
        .child(when(
            move || media_state.get().as_ref().is_some_and(|media| media.available),
            move || media_panel(media_state, dark),
        ))
        .child(when(move || !batteries.get().is_empty(), move || battery_panel(batteries, dark)))
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
    ai: AiState,
) -> impl IntoElement {
    view()
        .flex_row()
        .items_center()
        .gap(px(10.0))
        .px(px(18.0))
        .h(px(44.0))
        .child(view().w(px(8.0)).h(px(40.0)).window_drag(true))
        .child(ai_dot(dark, ai))
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
        .child(when(move || ai_unseen(ai) > 0, move || {
            view()
                .px(px(6.0))
                .py(px(1.0))
                .rounded(px(9.0))
                .bg(move || palette(dark.get()).warning)
                .child(
                    text(move || ai_unseen(ai).to_string())
                        .text_size(px(9.0))
                        .text_color(move || palette(dark.get()).bg),
                )
                .on_press(move || open_cindy(ai))
        }))
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

fn ai_summary_row(ai: AiState, dark: Signal<bool>) -> impl IntoElement {
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

fn ai_dot(dark: Signal<bool>, ai: AiState) -> impl IntoElement {
    view()
        .w(px(7.0))
        .h(px(7.0))
        .rounded(px(4.0))
        .bg(move || {
            if !ai.enabled.get() {
                return palette(dark.get()).accent;
            }
            ai_state_color(palette(dark.get()), ai.snapshot.get().state())
        })
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

fn battery_panel(batteries: Signal<Vec<PeripheralBatteryDevice>>, dark: Signal<bool>) -> impl IntoElement {
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

fn media_panel(media_state: Signal<Option<MediaState>>, dark: Signal<bool>) -> impl IntoElement {
    view()
        .flex_row()
        .items_center()
        .gap(px(6.0))
        .px(px(14.0))
        .pb(px(6.0))
        .child(
            text("♪")
                .text_size(px(13.0))
                .text_color(move || palette(dark.get()).accent),
        )
        .child(media_control("⏮", media_state, dark, media::previous))
        .child(media_play_control(media_state, dark))
        .child(media_control("⏭", media_state, dark, media::next))
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
        .child(media_volume_control("+", 0.1, media_state, dark))
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
        .default_size(size(px(520.0), px(780.0)))
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
        .child(section_label(move || t!("island.ai"), state))
        .child(toggle_row(move || t!("island.ai_enabled"), state.ai.enabled, state))
        .child(toggle_row(move || t!("island.notify_done"), state.ai.kinds[0], state))
        .child(toggle_row(move || t!("island.notify_question"), state.ai.kinds[1], state))
        .child(toggle_row(move || t!("island.notify_scheduled"), state.ai.kinds[2], state))
        .child(toggle_row(move || t!("island.notify_error"), state.ai.kinds[3], state))
        .child(
            text(move || ai_source_label(state))
                .text_size(px(10.0))
                .text_color(move || palette(state.dark.get()).faint),
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
        ai_enabled: state.ai.enabled.get_untracked(),
        notify_done: state.ai.kinds[0].get_untracked(),
        notify_question: state.ai.kinds[1].get_untracked(),
        notify_scheduled: state.ai.kinds[2].get_untracked(),
        notify_error: state.ai.kinds[3].get_untracked(),
    });
}
