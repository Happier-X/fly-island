//! The translation window and its settings section.

use std::{
    cell::RefCell,
    collections::BTreeMap,
    rc::Rc,
    sync::{mpsc, Mutex, OnceLock},
    time::Duration,
};

use nsh_core::{
    desktop::{WindowAttributes, WindowFrameStyle, WindowKind, WindowLevel, WindowPosition},
    element::{input, scroll},
    prelude::*,
};

use crate::palette;
use crate::translation::{self, hotkey};

static RESULT_SENDER: OnceLock<Mutex<mpsc::Sender<(u64, Result<String, String>)>>> = OnceLock::new();

#[derive(Clone, Copy)]
pub struct TranslationSettings {
    pub enabled: Signal<bool>,
    pub shortcut: Signal<String>,
    pub direction: Signal<String>,
    pub provider: Signal<String>,
    pub custom_base_url: Signal<String>,
    pub models: Signal<BTreeMap<String, String>>,
    pub auto: Signal<bool>,
    pub recording: Signal<bool>,
    pub key_draft: Signal<String>,
    pub key_saved: Signal<u64>,
    pub key_present: Signal<bool>,
    pub error: Signal<String>,
    pub dirty: Signal<u64>,
}

#[derive(Clone, Copy)]
pub struct TranslationState {
    pub open: Signal<bool>,
    pub input: Signal<String>,
    pub output: Signal<String>,
    pub error: Signal<String>,
    pub loading: Signal<bool>,
    pub copied: Signal<bool>,
    request_id: Signal<u64>,
    last_submitted: Signal<String>,
}

fn save_tick(settings: TranslationSettings) {
    settings.dirty.set(settings.dirty.get_untracked() + 1);
}

pub fn start(settings: TranslationSettings, active: Signal<bool>) -> TranslationState {
    let open = signal(false);
    let input = signal(String::new());
    let output = signal(String::new());
    let error = signal(String::new());
    let loading = signal(false);
    let copied = signal(false);
    let request_id = signal(0u64);
    let last_submitted = signal(String::new());
    let state = TranslationState { open, input, output, error, loading, copied, request_id, last_submitted };
    let (sender, receiver) = mpsc::channel::<(u64, Result<String, String>)>();
    let _ = RESULT_SENDER.set(Mutex::new(sender));
    let receiver = Rc::new(RefCell::new(receiver));
    let was_open = signal(false);
    let was_active = signal(false);

    set_interval(Duration::from_millis(100), move || {
        while let Ok((id, result)) = receiver.borrow_mut().try_recv() {
            if id == request_id.get_untracked() {
                loading.set(false);
                match result {
                    Ok(text) => output.set(text),
                    Err(failure) => error.set(failure),
                }
            }
        }
    });
    set_interval(Duration::from_millis(150), move || {
        if hotkey::take_request() {
            open.set(!open.get_untracked());
        }
    });
    effect(move || {
        let provider = settings.provider.get();
        let _ = settings.key_saved.get();
        settings.key_present.set(translation::has_secret(&provider).unwrap_or(false));
    });
    effect(move || {
        let _ = settings.direction.get();
        let _ = settings.provider.get();
        let _ = settings.custom_base_url.get();
        let _ = settings.models.get();
        last_submitted.set(String::new());
    });
    effect(move || {
        let enabled = settings.auto.get();
        let is_active = active.get();
        let present = settings.key_present.get();
        let loading_now = loading.get();
        let text = input.get().trim().to_string();
        if !enabled || !is_active || !present || loading_now || text.is_empty() || text.chars().count() > translation::MAX_CHARACTERS {
            return;
        }
        if text == last_submitted.get_untracked() {
            return;
        }
        let expected = text;
        set_timeout(Duration::from_millis(500), move || {
            if input.get_untracked().trim() == expected
                && !loading.get_untracked()
                && settings.auto.get_untracked()
                && active.get_untracked()
                && settings.key_present.get_untracked()
            {
                submit(state, settings);
            }
        });
    });
    effect(move || {
        let is_active = active.get();
        if is_active && !was_active.get_untracked() && input.get_untracked().trim().is_empty() {
            if let Ok(Some(text)) = translation::read_clipboard() {
                input.set(text);
            }
        }
        was_active.set(is_active);
    });
    effect(move || {
        let is_open = open.get();
        let _ = settings.provider.get();
        let _ = settings.key_saved.get();
        if is_open && !was_open.get_untracked() {
            request_id.set(request_id.get_untracked() + 1);
            input.set(String::new());
            output.set(String::new());
            error.set(String::new());
            copied.set(false);
            loading.set(false);
            if let Ok(Some(text)) = translation::read_clipboard() {
                input.set(text);
            }
        }
        was_open.set(is_open);
    });

    state
}

fn submit(state: TranslationState, settings: TranslationSettings) {
    if state.loading.get_untracked() {
        return;
    }
    let text = state.input.get_untracked().trim().to_string();
    if text.is_empty() {
        state.error.set(t!("island.translation_empty_source"));
        return;
    }
    if text.chars().count() > translation::MAX_CHARACTERS {
        state.error.set(t!("island.translation_too_long"));
        return;
    }
    let provider = settings.provider.get_untracked();
    let secret = match translation::read_secret(&provider) {
        Ok(Some(secret)) => secret,
        Ok(None) => {
            settings.key_present.set(false);
            state.error.set(t!("island.translation_no_key"));
            return;
        }
        Err(failure) => {
            state.error.set(failure);
            return;
        }
    };
    let direction = settings.direction.get_untracked();
    let base_url = settings.custom_base_url.get_untracked();
    let base_url = (!base_url.trim().is_empty()).then_some(base_url);
    let model = settings.models.get_untracked().get(&provider).cloned().filter(|model| !model.trim().is_empty());
    let id = state.request_id.get_untracked() + 1;
    state.request_id.set(id);
    state.last_submitted.set(text.clone());
    state.loading.set(true);
    state.output.set(String::new());
    state.error.set(String::new());
    state.copied.set(false);
    let Some(sender) = RESULT_SENDER.get() else {
        state.loading.set(false);
        state.error.set("翻译请求服务尚未准备好。".to_string());
        return;
    };
    let Ok(sender) = sender.lock() else {
        state.loading.set(false);
        state.error.set("翻译请求服务不可用。".to_string());
        return;
    };
    let sender = sender.clone();
    blocking(move || {
        let result = translation::translate_blocking(text, direction, provider, base_url, model, secret);
        let _ = sender.send((id, result));
    });
}

fn paste(state: TranslationState) {
    match translation::read_clipboard() {
        Ok(Some(text)) => {
            state.input.set(text);
            state.error.set(String::new());
            state.copied.set(false);
        }
        Ok(None) => state.error.set(t!("island.translation_empty_clipboard")),
        Err(failure) => state.error.set(failure),
    }
}

fn copy(state: TranslationState) {
    let text = state.output.get_untracked();
    if text.is_empty() {
        return;
    }
    match translation::copy_result(&text) {
        Ok(()) => state.copied.set(true),
        Err(failure) => state.error.set(failure),
    }
}

fn clear(state: TranslationState) {
    state.request_id.set(state.request_id.get_untracked() + 1);
    state.input.set(String::new());
    state.output.set(String::new());
    state.error.set(String::new());
    state.copied.set(false);
    state.loading.set(false);
}

fn apply_shortcut_label(settings: TranslationSettings, label: String) {
    if let Some(owner) = shortcut_owner(&label) {
        settings.error.set(t!("island.translation_shortcut_taken", shortcut = label, owner = owner));
        settings.recording.set(false);
        return;
    }
    match hotkey::set_shortcut(&label, settings.enabled.get_untracked()) {
        Ok(label) => {
            settings.shortcut.set(label);
            settings.recording.set(false);
            settings.error.set(String::new());
            save_tick(settings);
        }
        Err(failure) => {
            settings.recording.set(false);
            settings.error.set(failure);
        }
    }
}

fn reset_shortcut(settings: TranslationSettings) {
    settings.recording.set(false);
    apply_shortcut_label(settings, "Ctrl+Shift+F".to_string());
}

fn is_modifier_key(key: &str) -> bool {
    matches!(key, "Control" | "Shift" | "Alt" | "Meta" | "AltGraph" | "OS")
}

fn shortcut_key(key: &str, code: &str) -> Option<String> {
    if let Some(letter) = code.strip_prefix("Key").filter(|letter| letter.len() == 1) {
        return Some(letter.to_ascii_uppercase());
    }
    if let Some(digit) = code.strip_prefix("Digit").filter(|digit| digit.len() == 1) {
        return Some(digit.to_string());
    }
    if code.len() >= 2 && code.len() <= 3 && code.starts_with('F') && code[1..].chars().all(|c| c.is_ascii_digit()) {
        return Some(code.to_string());
    }
    match code {
        "Semicolon" => return Some(";".to_string()),
        "Equal" => return Some("=".to_string()),
        "Comma" => return Some(",".to_string()),
        "Minus" => return Some("-".to_string()),
        "Period" => return Some(".".to_string()),
        "Slash" => return Some("/".to_string()),
        "Backquote" => return Some("\u{0060}".to_string()),
        "BracketLeft" => return Some("[".to_string()),
        "Backslash" => return Some("\\\\".to_string()),
        "BracketRight" => return Some("]".to_string()),
        "Quote" => return Some("'".to_string()),
        _ => {}
    }
    match key {
        " " => Some("Space".to_string()),
        "Tab" => Some("Tab".to_string()),
        "Enter" => Some("Enter".to_string()),
        "Backspace" => Some("Backspace".to_string()),
        "Delete" => Some("Delete".to_string()),
        "Insert" => Some("Insert".to_string()),
        "Home" => Some("Home".to_string()),
        "End" => Some("End".to_string()),
        "PageUp" => Some("PageUp".to_string()),
        "PageDown" => Some("PageDown".to_string()),
        "ArrowUp" => Some("Up".to_string()),
        "ArrowDown" => Some("Down".to_string()),
        "ArrowLeft" => Some("Left".to_string()),
        "ArrowRight" => Some("Right".to_string()),
        key if key.chars().count() == 1 && key.chars().next().is_some_and(|c| c.is_ascii_alphanumeric()) => Some(key.to_ascii_uppercase()),
        _ => None,
    }
}

fn shortcut_label(event: &KeyEvent) -> Option<String> {
    if event.repeat {
        return None;
    }
    let key = shortcut_key(&event.key, &event.code)?;
    let mut parts = Vec::new();
    if event.modifiers.ctrl {
        parts.push("Ctrl".to_string());
    }
    if event.modifiers.alt {
        parts.push("Alt".to_string());
    }
    if event.modifiers.shift {
        parts.push("Shift".to_string());
    }
    if event.modifiers.cmd {
        parts.push("Win".to_string());
    }
    parts.push(key);
    hotkey::parse(&parts.join("+")).map(|binding| binding.label)
}

fn reserved_shortcut(label: &str) -> bool {
    matches!(label, "Ctrl+Shift+I" | "Ctrl+Shift+O" | "Ctrl+Shift+M" | "Ctrl+Shift+T")
}

fn shortcut_owner(label: &str) -> Option<String> {
    if !reserved_shortcut(label) {
        return None;
    }
    match label {
        "Ctrl+Shift+I" => Some(t!("island.shortcut_owner_island")),
        "Ctrl+Shift+O" => Some(t!("island.shortcut_owner_through")),
        "Ctrl+Shift+M" => Some(t!("island.shortcut_owner_monitor")),
        "Ctrl+Shift+T" => Some(t!("island.shortcut_owner_tuck")),
        _ => None,
    }
}

fn record_shortcut(settings: TranslationSettings, event: &KeyEvent) {
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
        apply_shortcut_label(settings, label);
    }
}

fn set_enabled(settings: TranslationSettings, enabled: bool) {
    match hotkey::set_shortcut(&settings.shortcut.get_untracked(), enabled) {
        Ok(label) => {
            settings.enabled.set(enabled);
            settings.shortcut.set(label);
            settings.error.set(String::new());
            save_tick(settings);
        }
        Err(failure) => settings.error.set(failure),
    }
}

fn small_button(label: impl Into<String> + 'static, dark: Signal<bool>, press: impl Fn() + 'static) -> impl IntoElement {
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

fn small_button_dynamic(label: impl Fn() -> String + 'static, dark: Signal<bool>, press: impl Fn() + 'static) -> impl IntoElement {
    text(label)
        .px(px(7.0))
        .py(px(2.0))
        .rounded(px(5.0))
        .bg(move || palette(dark.get()).input)
        .text_size(px(10.0))
        .text_color(move || palette(dark.get()).muted)
        .on_press(press)
}
fn pill(
    label: impl Fn() -> String + 'static,
    active: Rc<dyn Fn() -> bool + 'static>,
    press: impl Fn() + 'static,
    dark: Signal<bool>,
) -> impl IntoElement {
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

fn provider_text(settings: TranslationSettings) -> String {
    let provider = settings.provider.get();
    let name = translation::provider_name(&provider);
    let model = settings
        .models
        .get()
        .get(&provider)
        .cloned()
        .filter(|model| !model.trim().is_empty())
        .or_else(|| translation::default_model(&provider).map(str::to_string));
    model.map_or_else(|| name.to_string(), |model| format!("{name} · {model}"))
}

pub fn translation_window(
    state: TranslationState,
    settings: TranslationSettings,
    settings_open: Signal<bool>,
    dark: Signal<bool>,
) -> impl IntoElement {
    let mut attributes = WindowAttributes::for_kind(WindowKind::Normal);
    attributes.level = Some(WindowLevel::AboveDock);
    attributes.transparent = Some(true);
    attributes.frame = Some(WindowFrameStyle::None);
    attributes.resizable = Some(true);
    attributes.taskbar = Some(false);
    attributes.shadow = Some(false);
    attributes.activate = Some(true);
    nsh_core::element::desktop::window()
        .id("translation")
        .kind(WindowKind::Normal)
        .title(t!("island.translation_title"))
        .default_size(size(px(480.0), px(640.0)))
        .default_position(WindowPosition::Center)
        .restore_bounds(true)
        .attributes(attributes)
        .open(state.open)
        .on_open_change(move |event| state.open.set(event.open))
        .content(move || content(state, settings, settings_open, dark))
}

pub fn island_panel(
    state: TranslationState,
    settings: TranslationSettings,
    settings_open: Signal<bool>,
    dark: Signal<bool>,
) -> impl IntoElement {
    let count = move || {
        let text = state.input.get();
        format!("{} / {}", text.chars().count(), translation::MAX_CHARACTERS)
    };
    view()
        .flex_1()
        .w_full()
        .flex_col()
        .gap(px(5.0))
        .px(px(14.0))
        .pb(px(8.0))
        .child(
            view()
                .flex_row()
                .items_center()
                .gap(px(5.0))
                .child(pill(
                    move || t!("island.translation_auto"),
                    Rc::new(move || settings.direction.get() == "auto"),
                    move || {
                        settings.direction.set("auto".to_string());
                        save_tick(settings);
                    },
                    dark,
                ))
                .child(pill(
                    move || t!("island.translation_zh_en"),
                    Rc::new(move || settings.direction.get() == "zh-en"),
                    move || {
                        settings.direction.set("zh-en".to_string());
                        save_tick(settings);
                    },
                    dark,
                ))
                .child(pill(
                    move || t!("island.translation_en_zh"),
                    Rc::new(move || settings.direction.get() == "en-zh"),
                    move || {
                        settings.direction.set("en-zh".to_string());
                        save_tick(settings);
                    },
                    dark,
                ))
                .child(view().flex_1())
                .child(
                    text(move || provider_text(settings))
                        .truncate()
                        .text_size(px(9.0))
                        .text_color(move || palette(dark.get()).faint),
                ),
        )
        .child(
            view()
                .flex_row()
                .items_center()
                .gap(px(5.0))
                .child(
                    text(move || t!("island.translation_source"))
                        .text_size(px(10.0))
                        .text_color(move || palette(dark.get()).muted),
                )
                .child(
                    text(count)
                        .text_size(px(9.0))
                        .text_color(move || palette(dark.get()).faint),
                )
                .child(view().flex_1())
                .child(small_button(t!("island.translation_paste"), dark, move || paste(state)))
                .child(small_button(t!("island.translation_clear"), dark, move || clear(state))),
        )
        .child(
            input()
                .multiline(true)
                .submit_behavior(SubmitBehavior::Submit)
                .value(move || state.input.get())
                .placeholder(move || t!("island.translation_placeholder"))
                .placeholder_color(move || palette(dark.get()).faint)
                .text_color(move || palette(dark.get()).fg)
                .bg(move || palette(dark.get()).input)
                .h(px(104.0))
                .p(px(8.0))
                .rounded(px(8.0))
                .text_size(px(12.5))
                .on_change(move |text: &str| {
                    state.input.set(text.to_string());
                    state.error.set(String::new());
                    state.copied.set(false);
                })
                .on_submit(move |_text: &str| submit(state, settings)),
        )
        .child(
            view()
                .flex_row()
                .items_center()
                .gap(px(5.0))
                .child(
                    text(move || t!("island.translation_result"))
                        .text_size(px(10.0))
                        .text_color(move || palette(dark.get()).muted),
                )
                .child(view().flex_1())
                .child(when(move || !state.output.get().is_empty(), move || {
                    small_button_dynamic(
                        move || if state.copied.get() { t!("island.translation_copied") } else { t!("island.translation_copy") },
                        dark,
                        move || copy(state),
                    )
                })),
        )
        .child(
            view()
                .flex_1()
                .w_full()
                .rounded(px(8.0))
                .bg(move || palette(dark.get()).input)
                .p(px(8.0))
                .child(
                    scroll()
                        .flex_1()
                        .scrollbar(false)
                        .child(when(move || state.loading.get(), move || {
                            text(move || t!("island.translation_loading"))
                                .text_size(px(11.0))
                                .text_color(move || palette(dark.get()).muted)
                        }))
                        .child(when(move || !state.loading.get() && !state.output.get().is_empty(), move || {
                            text(move || state.output.get())
                                .selectable(true)
                                .text_size(px(12.0))
                                .text_color(move || palette(dark.get()).fg)
                        }))
                        .child(when(move || !state.loading.get() && state.output.get().is_empty(), move || {
                            text(move || t!("island.translation_empty_result"))
                                .text_size(px(10.0))
                                .text_color(move || palette(dark.get()).faint)
                        })),
                ),
        )
        .child(
            view()
                .flex_row()
                .items_center()
                .gap(px(6.0))
                .child(
                    text(move || {
                        let failure = state.error.get();
                        if !failure.is_empty() {
                            failure
                        } else if !settings.key_present.get() {
                            t!("island.translation_no_key")
                        } else {
                            t!("island.translation_enter")
                        }
                    })
                    .flex_1()
                    .truncate()
                    .text_size(px(9.5))
                    .text_color(move || if !state.error.get().is_empty() { palette(dark.get()).warning } else { palette(dark.get()).faint })
                    .on_press(move || {
                        if settings.key_present.get_untracked() == false {
                            settings_open.set(true);
                        }
                    }),
                )
                .child(
                    text(move || if state.error.get().is_empty() { t!("island.translation_translate") } else { t!("island.translation_retry") })
                        .px(px(11.0))
                        .py(px(4.0))
                        .rounded(px(7.0))
                        .bg(move || palette(dark.get()).accent)
                        .text_size(px(11.0))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(move || palette(dark.get()).bg)
                        .on_press(move || submit(state, settings)),
                ),
        )
}
fn content(
    state: TranslationState,
    settings: TranslationSettings,
    settings_open: Signal<bool>,
    dark: Signal<bool>,
) -> impl IntoElement {
    let count = move || {
        let text = state.input.get();
        format!("{} / {}", text.chars().count(), translation::MAX_CHARACTERS)
    };
    view()
        .flex_1()
        .w_full()
        .h_full()
        .overflow_hidden()
        .rounded(px(10.0))
        .border(px(1.0))
        .border_color(move || palette(dark.get()).border)
        .bg(move || palette(dark.get()).bg)
        .text_color(move || palette(dark.get()).fg)
        .p(px(12.0))
        .flex_col()
        .gap(px(8.0))
        .child(
            view()
                .w_full()
                .h(px(24.0))
                .flex_row()
                .items_center()
                .gap(px(8.0))
                .window_drag(true)
                .child(
                    text(move || t!("island.translation"))
                        .text_size(px(14.0))
                        .font_weight(FontWeight::SEMIBOLD),
                )
                .child(
                    text(move || settings.shortcut.get())
                        .text_size(px(9.0))
                        .text_color(move || palette(dark.get()).faint),
                )
                .child(view().flex_1())
                .child(
                    text(move || t!("island.translation_go_settings"))
                        .text_size(px(11.0))
                        .text_color(move || palette(dark.get()).muted)
                        .on_press(move || settings_open.set(true)),
                )
                .child(
                    text("✕")
                        .text_size(px(13.0))
                        .text_color(move || palette(dark.get()).muted)
                        .on_press(move || state.open.set(false)),
                ),
        )
        .child(
            view()
                .flex_row()
                .items_center()
                .gap(px(5.0))
                .child(pill(
                    move || t!("island.translation_auto"),
                    Rc::new(move || settings.direction.get() == "auto"),
                    move || {
                        settings.direction.set("auto".to_string());
                        save_tick(settings);
                    },
                    dark,
                ))
                .child(pill(
                    move || t!("island.translation_zh_en"),
                    Rc::new(move || settings.direction.get() == "zh-en"),
                    move || {
                        settings.direction.set("zh-en".to_string());
                        save_tick(settings);
                    },
                    dark,
                ))
                .child(pill(
                    move || t!("island.translation_en_zh"),
                    Rc::new(move || settings.direction.get() == "en-zh"),
                    move || {
                        settings.direction.set("en-zh".to_string());
                        save_tick(settings);
                    },
                    dark,
                ))
                .child(view().flex_1())
                .child(
                    text(move || provider_text(settings))
                        .truncate()
                        .text_size(px(9.0))
                        .text_color(move || palette(dark.get()).faint),
                ),
        )
        .child(
            view()
                .flex_row()
                .items_center()
                .gap(px(5.0))
                .child(
                    text(move || t!("island.translation_source"))
                        .text_size(px(10.0))
                        .text_color(move || palette(dark.get()).muted),
                )
                .child(
                    text(count)
                        .text_size(px(9.0))
                        .text_color(move || palette(dark.get()).faint),
                )
                .child(view().flex_1())
                .child(small_button(t!("island.translation_paste"), dark, move || paste(state)))
                .child(small_button(t!("island.translation_clear"), dark, move || clear(state))),
        )
        .child(
            input()
                .multiline(true)
                .submit_behavior(SubmitBehavior::Submit)
                .value(move || state.input.get())
                .placeholder(move || t!("island.translation_placeholder"))
                .placeholder_color(move || palette(dark.get()).faint)
                .text_color(move || palette(dark.get()).fg)
                .bg(move || palette(dark.get()).input)
                .h(px(180.0))
                .p(px(8.0))
                .rounded(px(8.0))
                .text_size(px(13.0))
                .auto_focus(true)
                .on_change(move |text: &str| {
                    state.input.set(text.to_string());
                    state.error.set(String::new());
                    state.copied.set(false);
                })
                .on_submit(move |_text: &str| submit(state, settings)),
        )
        .child(
            view()
                .flex_row()
                .items_center()
                .gap(px(5.0))
                .child(
                    text(move || t!("island.translation_result"))
                        .text_size(px(10.0))
                        .text_color(move || palette(dark.get()).muted),
                )
                .child(view().flex_1())
                .child(when(move || !state.output.get().is_empty(), move || {
                    small_button_dynamic(
                        move || if state.copied.get() { t!("island.translation_copied") } else { t!("island.translation_copy") },
                        dark,
                        move || copy(state),
                    )
                })),
        )
        .child(
            view()
                .flex_1()
                .w_full()
                .rounded(px(8.0))
                .bg(move || palette(dark.get()).input)
                .p(px(8.0))
                .child(
                    scroll()
                        .flex_1()
                        .scrollbar(false)
                        .child(when(
                            move || state.loading.get(),
                            move || {
                                text(move || t!("island.translation_loading"))
                                    .text_size(px(12.0))
                                    .text_color(move || palette(dark.get()).muted)
                            },
                        ))
                        .child(when(
                            move || !state.loading.get() && !state.output.get().is_empty(),
                            move || {
                                text(move || state.output.get())
                                    .selectable(true)
                                    .text_size(px(13.0))
                                    .text_color(move || palette(dark.get()).fg)
                            },
                        ))
                        .child(when(
                            move || !state.loading.get() && state.output.get().is_empty(),
                            move || {
                                text(move || t!("island.translation_empty_result"))
                                    .text_size(px(11.0))
                                    .text_color(move || palette(dark.get()).faint)
                            },
                        )),
                ),
        )
        .child(
            view()
                .flex_row()
                .items_center()
                .gap(px(8.0))
                .child(
                    text(move || {
                        let failure = state.error.get();
                        if !failure.is_empty() {
                            failure
                        } else if !settings.key_present.get() {
                            t!("island.translation_no_key")
                        } else {
                            t!("island.translation_enter")
                        }
                    })
                    .flex_1()
                    .truncate()
                    .text_size(px(10.0))
                    .text_color(move || {
                        if !state.error.get().is_empty() {
                            palette(dark.get()).warning
                        } else {
                            palette(dark.get()).faint
                        }
                    }),
                )
                .child(
                    text(move || if state.error.get().is_empty() { t!("island.translation_translate") } else { t!("island.translation_retry") })
                        .px(px(12.0))
                        .py(px(5.0))
                        .rounded(px(7.0))
                        .bg(move || palette(dark.get()).accent)
                        .text_size(px(12.0))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(move || palette(dark.get()).bg)
                        .on_press(move || submit(state, settings)),
                ),
        )
        .child(
            view()
                .absolute()
                .right(px(4.0))
                .bottom(px(4.0))
                .w(px(12.0))
                .h(px(12.0))
                .window_resize(nsh_core::desktop::ResizeEdge::BottomRight),
        )
}

fn provider_pill(settings: TranslationSettings, id: &'static str, dark: Signal<bool>) -> impl IntoElement {
    pill(
        move || translation::provider_name(id).to_string(),
        Rc::new(move || settings.provider.get() == id),
        move || {
            settings.provider.set(id.to_string());
            settings.key_draft.set(String::new());
            settings.error.set(String::new());
            save_tick(settings);
        },
        dark,
    )
}

fn save_secret(settings: TranslationSettings) {
    let provider = settings.provider.get_untracked();
    let value = settings.key_draft.get_untracked();
    match translation::save_secret(&provider, &value) {
        Ok(()) => {
            settings.key_draft.set(String::new());
            settings.key_saved.set(settings.key_saved.get_untracked() + 1);
            settings.error.set(String::new());
        }
        Err(failure) => settings.error.set(failure),
    }
}

fn delete_secret(settings: TranslationSettings) {
    let provider = settings.provider.get_untracked();
    match translation::delete_secret(&provider) {
        Ok(()) => {
            settings.key_draft.set(String::new());
            settings.key_saved.set(settings.key_saved.get_untracked() + 1);
            settings.error.set(String::new());
        }
        Err(failure) => settings.error.set(failure),
    }
}

fn set_model(settings: TranslationSettings, value: &str) {
    let provider = settings.provider.get_untracked();
    let mut models = settings.models.get_untracked();
    if value.trim().is_empty() {
        models.remove(&provider);
    } else {
        models.insert(provider, value.to_string());
    }
    settings.models.set(models);
    save_tick(settings);
}

pub fn settings_section(settings: TranslationSettings, dark: Signal<bool>) -> impl IntoElement {
    let capture = element_ref();
    let custom = move || settings.provider.get() == "custom";
    view()
        .flex_col()
        .gap(px(5.0))
        .child(
            text(move || t!("island.translation_settings"))
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
                    text(move || t!("island.translation_enabled"))
                        .flex_1()
                        .text_size(px(13.0))
                        .text_color(move || palette(dark.get()).muted),
                )
                .child(pill(
                    move || t!("island.on"),
                    Rc::new(move || settings.enabled.get()),
                    move || set_enabled(settings, true),
                    dark,
                ))
                .child(pill(
                    move || t!("island.off"),
                    Rc::new(move || !settings.enabled.get()),
                    move || set_enabled(settings, false),
                    dark,
                )),
        )
        .child(
            view()
                .flex_row()
                .items_center()
                .gap(px(6.0))
                .child(
                    text(move || t!("island.translation_auto_translate"))
                        .flex_1()
                        .text_size(px(13.0))
                        .text_color(move || palette(dark.get()).muted),
                )
                .child(pill(
                    move || t!("island.on"),
                    Rc::new(move || settings.auto.get()),
                    move || {
                        settings.auto.set(true);
                        save_tick(settings);
                    },
                    dark,
                ))
                .child(pill(
                    move || t!("island.off"),
                    Rc::new(move || !settings.auto.get()),
                    move || {
                        settings.auto.set(false);
                        save_tick(settings);
                    },
                    dark,
                )),
        )
        .child(
            view()
                .flex_row()
                .items_center()
                .gap(px(6.0))
                .child(
                    text(move || t!("island.translation_shortcut"))
                        .text_size(px(13.0))
                        .text_color(move || palette(dark.get()).muted),
                )
                .child(
                    input()
                        .readonly(true)
                        .value(move || if settings.recording.get() { t!("island.translation_recording") } else { settings.shortcut.get() })
                        .text_color(move || palette(dark.get()).fg)
                        .bg(move || palette(dark.get()).input)
                        .h(px(28.0))
                        .w(px(150.0))
                        .px(px(8.0))
                        .rounded(px(6.0))
                        .auto_focus(move || settings.recording.get())
                        .element_ref(&capture)
                        .on_key_down(move |event: &KeyEvent| record_shortcut(settings, event)),
                )
                .child(small_button_dynamic(
                    move || if settings.recording.get() { t!("island.translation_cancel") } else { t!("island.translation_record") },
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
                .child(small_button(t!("island.translation_reset"), dark, move || reset_shortcut(settings))),
        )
        .child(
            view()
                .flex_row()
                .items_center()
                .gap(px(5.0))
                .child(
                    text(move || t!("island.translation_direction"))
                        .text_size(px(13.0))
                        .text_color(move || palette(dark.get()).muted),
                )
                .child(pill(
                    move || t!("island.translation_auto"),
                    Rc::new(move || settings.direction.get() == "auto"),
                    move || {
                        settings.direction.set("auto".to_string());
                        save_tick(settings);
                    },
                    dark,
                ))
                .child(pill(
                    move || t!("island.translation_zh_en"),
                    Rc::new(move || settings.direction.get() == "zh-en"),
                    move || {
                        settings.direction.set("zh-en".to_string());
                        save_tick(settings);
                    },
                    dark,
                ))
                .child(pill(
                    move || t!("island.translation_en_zh"),
                    Rc::new(move || settings.direction.get() == "en-zh"),
                    move || {
                        settings.direction.set("en-zh".to_string());
                        save_tick(settings);
                    },
                    dark,
                )),
        )
        .child(
            view()
                .flex_row()
                .flex_wrap()
                .gap(px(4.0))
                .child(
                    text(move || t!("island.translation_provider"))
                        .text_size(px(13.0))
                        .text_color(move || palette(dark.get()).muted),
                )
                .child(provider_pill(settings, "deepseek", dark))
                .child(provider_pill(settings, "volcengine", dark))
                .child(provider_pill(settings, "dashscope", dark))
                .child(provider_pill(settings, "moonshot", dark))
                .child(provider_pill(settings, "siliconflow", dark))
                .child(provider_pill(settings, "openai", dark))
                .child(provider_pill(settings, "custom", dark)),
        )
        .child(when(custom, move || {
            view()
                .flex_col()
                .gap(px(3.0))
                .child(
                    text(move || t!("island.translation_custom_base_url"))
                        .text_size(px(12.0))
                        .text_color(move || palette(dark.get()).muted),
                )
                .child(
                    input()
                        .value(move || settings.custom_base_url.get())
                        .placeholder("https://example.com/v1")
                        .text_color(move || palette(dark.get()).fg)
                        .bg(move || palette(dark.get()).input)
                        .h(px(28.0))
                        .px(px(8.0))
                        .rounded(px(6.0))
                        .on_change(move |text: &str| {
                            settings.custom_base_url.set(text.to_string());
                            save_tick(settings);
                        }),
                )
                .child(
                    text(move || t!("island.translation_base_url_hint"))
                        .text_size(px(9.0))
                        .text_color(move || palette(dark.get()).faint),
                )
        }))
        .child(
            view()
                .flex_row()
                .items_center()
                .gap(px(6.0))
                .child(
                    text(move || t!("island.translation_model"))
                        .text_size(px(13.0))
                        .text_color(move || palette(dark.get()).muted),
                )
                .child(
                    input()
                        .value(move || settings.models.get().get(&settings.provider.get()).cloned().unwrap_or_default())
                        .placeholder(move || {
                            translation::default_model(&settings.provider.get())
                                .map_or_else(|| t!("island.translation_model_required"), |model| t!("island.translation_model_default", model = model))
                        })
                        .text_color(move || palette(dark.get()).fg)
                        .bg(move || palette(dark.get()).input)
                        .h(px(28.0))
                        .flex_1()
                        .px(px(8.0))
                        .rounded(px(6.0))
                        .on_change(move |text: &str| set_model(settings, text)),
                ),
        )
        .child(
            view()
                .flex_row()
                .items_center()
                .gap(px(6.0))
                .child(
                    text(move || t!("island.translation_secret"))
                        .text_size(px(13.0))
                        .text_color(move || palette(dark.get()).muted),
                )
                .child(
                    text(move || if settings.key_present.get() { t!("island.translation_secret_saved") } else { t!("island.translation_secret_missing") })
                        .text_size(px(10.0))
                        .text_color(move || if settings.key_present.get() { palette(dark.get()).success } else { palette(dark.get()).faint }),
                ),
        )
        .child(
            view()
                .flex_row()
                .items_center()
                .gap(px(5.0))
                .child(
                    input()
                        .masked(true)
                        .value(move || settings.key_draft.get())
                        .placeholder(move || t!("island.translation_secret_placeholder"))
                        .text_color(move || palette(dark.get()).fg)
                        .bg(move || palette(dark.get()).input)
                        .h(px(28.0))
                        .flex_1()
                        .px(px(8.0))
                        .rounded(px(6.0))
                        .on_change(move |text: &str| settings.key_draft.set(text.to_string()))
                        .on_submit(move |_text: &str| save_secret(settings)),
                )
                .child(small_button(t!("island.translation_save"), dark, move || save_secret(settings)))
                .child(when(move || settings.key_present.get(), move || {
                    small_button(t!("island.translation_delete"), dark, move || delete_secret(settings))
                })),
        )
        .child(
            text(move || t!("island.translation_secret_hint"))
                .text_size(px(9.0))
                .text_color(move || palette(dark.get()).faint),
        )
        .child(when(move || !settings.error.get().is_empty(), move || {
            text(move || settings.error.get())
                .text_size(px(10.0))
                .text_color(move || palette(dark.get()).warning)
        }))
}
#[cfg(test)]
mod tests {
    use super::*;

    fn key(key: &str, code: &str, ctrl: bool, alt: bool, shift: bool, cmd: bool) -> KeyEvent {
        let mut event = KeyEvent::default();
        event.key = key.to_string().into();
        event.code = code.to_string().into();
        event.modifiers = nsh_core::keymap::Modifiers { ctrl, alt, shift, cmd };
        event
    }

    #[test]
    fn recorded_keys_become_the_same_labels_as_the_parser() {
        assert_eq!(shortcut_label(&key("F", "KeyF", true, false, true, false)).as_deref(), Some("Ctrl+Shift+F"));
        assert_eq!(shortcut_label(&key("ArrowLeft", "ArrowLeft", true, true, false, false)).as_deref(), Some("Ctrl+Alt+Left"));
        assert_eq!(shortcut_label(&key(";", "Semicolon", true, false, false, false)).as_deref(), Some("Ctrl+;"));
        assert_eq!(shortcut_label(&key("F", "KeyF", false, false, true, false)).as_deref(), Some("Shift+F"));
    }

    #[test]
    fn shortcuts_owned_by_the_island_are_reported() {
        assert!(reserved_shortcut("Ctrl+Shift+I"));
        assert!(reserved_shortcut("Ctrl+Shift+O"));
        assert!(reserved_shortcut("Ctrl+Shift+M"));
        assert!(reserved_shortcut("Ctrl+Shift+T"));
        assert!(!reserved_shortcut("Ctrl+Shift+Y"));
    }
}