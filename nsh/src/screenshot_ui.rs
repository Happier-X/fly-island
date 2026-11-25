//! Screenshot section in the settings window.

use std::{collections::BTreeMap, rc::Rc, sync::Mutex, time::Duration};

use nsh_core::{element::input, prelude::*};

use crate::palette;
use crate::screenshot::{self, ScreenshotSettings};

#[derive(Clone, Copy)]
pub struct ScreenshotSettingsState {
    pub enabled: Signal<bool>,
    pub shortcut: Signal<String>,
    pub save_format: Signal<String>,
    pub save_dir: Signal<String>,
    pub auto_save: Signal<bool>,
    pub ocr_language: Signal<String>,
    pub ocr_auto_copy: Signal<bool>,
    pub ocr_available: Signal<Vec<String>>,
    pub longshot_max_height: Signal<String>,
    pub colors: Signal<Vec<String>>,
    pub name_style: Signal<String>,
    pub ui_scale: Signal<String>,
    pub tool_keys: Signal<BTreeMap<String, String>>,
    pub hidden_toolbar: Signal<Vec<String>>,
    pub recording: Signal<bool>,
    pub error: Signal<String>,
}

impl ScreenshotSettingsState {
    pub fn new(settings: &ScreenshotSettings) -> Self {
        Self {
            enabled: signal(settings.enabled),
            shortcut: signal(settings.shortcut.clone()),
            save_format: signal(settings.save_format.clone()),
            save_dir: signal(settings.save_dir.clone()),
            auto_save: signal(settings.auto_save),
            ocr_language: signal(settings.ocr_language.clone()),
            ocr_auto_copy: signal(settings.ocr_auto_copy),
            ocr_available: signal(Vec::new()),
            longshot_max_height: signal(settings.longshot_max_height.to_string()),
            colors: signal(settings.colors.clone()),
            name_style: signal(settings.name_style.clone()),
            ui_scale: signal(settings.ui_scale.clone()),
            tool_keys: signal(settings.tool_keys.clone()),
            hidden_toolbar: signal(settings.hidden_toolbar.clone()),
            recording: signal(false),
            error: signal(screenshot::last_error().unwrap_or_default()),
        }
    }

    fn snapshot(self) -> ScreenshotSettings {
        ScreenshotSettings {
            enabled: self.enabled.get_untracked(),
            shortcut: self.shortcut.get_untracked(),
            save_format: self.save_format.get_untracked(),
            save_dir: self.save_dir.get_untracked(),
            auto_save: self.auto_save.get_untracked(),
            ocr_language: self.ocr_language.get_untracked(),
            ocr_auto_copy: self.ocr_auto_copy.get_untracked(),
            longshot_max_height: self.longshot_max_height.get_untracked().trim().parse().unwrap_or(screenshot::DEFAULT_LONGSHOT_MAX),
            colors: self.colors.get_untracked(),
            name_style: self.name_style.get_untracked(),
            ui_scale: self.ui_scale.get_untracked(),
            tool_keys: self.tool_keys.get_untracked(),
            hidden_toolbar: self.hidden_toolbar.get_untracked(),
        }
    }
}

static OCR_LANGS: Mutex<Option<Vec<String>>> = Mutex::new(None);

/// Ask WinRT for installed OCR packs once and publish them to the settings row.
pub fn load_ocr_languages(state: ScreenshotSettingsState) {
    std::thread::spawn(|| {
        let languages = airsnip::ocr::available_languages().unwrap_or_default();
        if let Ok(mut slot) = OCR_LANGS.lock() {
            *slot = Some(languages);
        }
    });
    spawn(async move {
        for _ in 0..40 {
            timer(Duration::from_millis(250)).await;
            let languages = OCR_LANGS.lock().ok().and_then(|mut slot| slot.take());
            if let Some(languages) = languages {
                state.ocr_available.set(languages);
                break;
            }
        }
    });
}

pub(crate) fn persist(state: ScreenshotSettingsState) {
    match screenshot::save(&state.snapshot()) {
        Ok(settings) => {
            state.shortcut.set(settings.shortcut);
            state.save_format.set(settings.save_format);
            state.save_dir.set(settings.save_dir);
            state.ocr_language.set(settings.ocr_language);
            state.longshot_max_height.set(settings.longshot_max_height.to_string());
            state.colors.set(settings.colors);
            state.name_style.set(settings.name_style);
            state.ui_scale.set(settings.ui_scale);
            state.tool_keys.set(settings.tool_keys);
            state.hidden_toolbar.set(settings.hidden_toolbar);
            state.error.set(String::new());
        }
        Err(error) => state.error.set(error),
    }
}

fn apply_shortcut(state: ScreenshotSettingsState, label: String) {
    state.shortcut.set(label);
    state.recording.set(false);
    persist(state);
}

fn record_shortcut(state: ScreenshotSettingsState, event: &KeyEvent) {
    if !state.recording.get_untracked() {
        return;
    }
    if event.key == "Escape" {
        state.recording.set(false);
        return;
    }
    if crate::translation_ui::is_modifier_key(&event.key) {
        return;
    }
    if let Some(label) = crate::translation_ui::shortcut_label(event) {
        apply_shortcut(state, label);
    }
}

fn reset_shortcut(state: ScreenshotSettingsState) {
    state.recording.set(false);
    apply_shortcut(state, screenshot::default_shortcut());
}

pub fn settings_section(state: ScreenshotSettingsState, dark: Signal<bool>) -> impl IntoElement {
    let capture = element_ref();
    view()
        .flex_col()
        .gap(px(5.0))
        .child(
            view()
                .flex_row()
                .items_center()
                .gap(px(6.0))
                .child(
                    text(move || t!("island.screenshot_enabled"))
                        .flex_1()
                        .text_size(px(13.0))
                        .text_color(move || palette(dark.get()).muted),
                )
                .child(pill(
                    move || t!("island.on"),
                    Rc::new(move || state.enabled.get()),
                    move || {
                        state.enabled.set(true);
                        persist(state);
                    },
                    dark,
                ))
                .child(pill(
                    move || t!("island.off"),
                    Rc::new(move || !state.enabled.get()),
                    move || {
                        state.enabled.set(false);
                        persist(state);
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
                    text(move || t!("island.screenshot_shortcut"))
                        .text_size(px(13.0))
                        .text_color(move || palette(dark.get()).muted),
                )
                .child(
                    input()
                        .readonly(true)
                        .value(move || {
                            if state.recording.get() {
                                t!("island.screenshot_recording")
                            } else {
                                state.shortcut.get()
                            }
                        })
                        .text_color(move || palette(dark.get()).fg)
                        .bg(move || palette(dark.get()).input)
                        .h(px(28.0))
                        .w(px(150.0))
                        .px(px(8.0))
                        .rounded(px(6.0))
                        .auto_focus(move || state.recording.get())
                        .element_ref(&capture)
                        .on_key_down(move |event: &KeyEvent| record_shortcut(state, event)),
                )
                .child(small_button_dynamic(
                    move || {
                        if state.recording.get() {
                            t!("island.screenshot_cancel")
                        } else {
                            t!("island.screenshot_record")
                        }
                    },
                    dark,
                    move || {
                        if state.recording.get_untracked() {
                            state.recording.set(false);
                        } else {
                            state.recording.set(true);
                            capture.focus();
                        }
                    },
                ))
                .child(small_button(
                    t!("island.screenshot_reset"),
                    dark,
                    move || reset_shortcut(state),
                )),
        )
        .child(
            view()
                .flex_row()
                .items_center()
                .gap(px(5.0))
                .child(
                    text(move || t!("island.screenshot_format"))
                        .text_size(px(13.0))
                        .text_color(move || palette(dark.get()).muted),
                )
                .child(format_pill(state, "png", "PNG", dark))
                .child(format_pill(state, "jpeg", "JPEG", dark))
                .child(format_pill(state, "webp", "WebP", dark)),
        )
        .child(
            view()
                .flex_row()
                .items_center()
                .gap(px(6.0))
                .child(
                    text(move || t!("island.screenshot_dir"))
                        .text_size(px(13.0))
                        .text_color(move || palette(dark.get()).muted),
                )
                .child(
                    input()
                        .value(move || state.save_dir.get())
                        .placeholder(move || t!("island.screenshot_dir_placeholder"))
                        .placeholder_color(move || palette(dark.get()).faint)
                        .text_color(move || palette(dark.get()).fg)
                        .bg(move || palette(dark.get()).input)
                        .h(px(30.0))
                        .flex_1()
                        .px(px(10.0))
                        .rounded(px(8.0))
                        .on_change(move |text: &str| {
                            state.save_dir.set(text.to_string());
                            persist(state);
                        }),
                ),
        )
        .child(
            view()
                .flex_row()
                .items_center()
                .gap(px(6.0))
                .child(
                    text(move || t!("island.screenshot_auto_save"))
                        .flex_1()
                        .text_size(px(13.0))
                        .text_color(move || palette(dark.get()).muted),
                )
                .child(pill(
                    move || t!("island.on"),
                    Rc::new(move || state.auto_save.get()),
                    move || {
                        state.auto_save.set(true);
                        persist(state);
                    },
                    dark,
                ))
                .child(pill(
                    move || t!("island.off"),
                    Rc::new(move || !state.auto_save.get()),
                    move || {
                        state.auto_save.set(false);
                        persist(state);
                    },
                    dark,
                )),
        )
        .child(
            view()
                .flex_row()
                .flex_wrap()
                .items_center()
                .gap(px(5.0))
                .child(
                    text(move || t!("island.screenshot_ocr_language"))
                        .text_size(px(13.0))
                        .text_color(move || palette(dark.get()).muted),
                )
                .child(pill(
                    move || t!("island.screenshot_ocr_auto"),
                    Rc::new(move || state.ocr_language.get() == "auto"),
                    move || {
                        state.ocr_language.set("auto".to_string());
                        persist(state);
                    },
                    dark,
                ))
                .child(language_pill(state, "zh", "中文", "zh-Hans-CN", dark))
                .child(language_pill(state, "en", "English", "en-US", dark)),
        )
        .child(
            text(move || {
                let languages = state.ocr_available.get();
                if languages.is_empty() {
                    t!("island.screenshot_ocr_missing")
                } else {
                    t!("island.screenshot_ocr_installed", langs = languages.join("、"))
                }
            })
            .text_size(px(11.0))
            .text_color(move || palette(dark.get()).faint),
        )
        .child(
            view()
                .flex_row()
                .items_center()
                .gap(px(6.0))
                .child(
                    text(move || t!("island.screenshot_ocr_auto_copy"))
                        .flex_1()
                        .text_size(px(13.0))
                        .text_color(move || palette(dark.get()).muted),
                )
                .child(pill(
                    move || t!("island.on"),
                    Rc::new(move || state.ocr_auto_copy.get()),
                    move || {
                        state.ocr_auto_copy.set(true);
                        persist(state);
                    },
                    dark,
                ))
                .child(pill(
                    move || t!("island.off"),
                    Rc::new(move || !state.ocr_auto_copy.get()),
                    move || {
                        state.ocr_auto_copy.set(false);
                        persist(state);
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
                    text(move || t!("island.screenshot_longshot_max"))
                        .flex_1()
                        .text_size(px(13.0))
                        .text_color(move || palette(dark.get()).muted),
                )
                .child(
                    input()
                        .value(move || state.longshot_max_height.get())
                        .text_color(move || palette(dark.get()).fg)
                        .bg(move || palette(dark.get()).input)
                        .h(px(30.0))
                        .w(px(110.0))
                        .px(px(8.0))
                        .rounded(px(8.0))
                        .on_change(move |text: &str| {
                            state.longshot_max_height.set(text.to_string());
                            persist(state);
                        }),
                ),
        )
        .child(crate::screenshot_advanced_ui::advanced_section(state, dark))
        .child(
            text(move || t!("island.screenshot_shortcut_hint"))
                .text_size(px(11.0))
                .text_color(move || palette(dark.get()).faint),
        )
        .child(when(
            move || !state.error.get().is_empty(),
            move || {
                text(move || state.error.get())
                    .text_size(px(12.0))
                    .text_color(move || palette(dark.get()).warning)
            },
        ))
}

fn format_pill(
    state: ScreenshotSettingsState,
    value: &'static str,
    label: &'static str,
    dark: Signal<bool>,
) -> impl IntoElement {
    pill(
        move || label.to_string(),
        Rc::new(move || state.save_format.get() == value),
        move || {
            state.save_format.set(value.to_string());
            persist(state);
        },
        dark,
    )
}

fn language_pill(
    state: ScreenshotSettingsState,
    prefix: &'static str,
    label: &'static str,
    fallback: &'static str,
    dark: Signal<bool>,
) -> impl IntoElement {
    pill(
        move || label.to_string(),
        Rc::new(move || {
            state
                .ocr_language
                .get()
                .to_ascii_lowercase()
                .starts_with(prefix)
        }),
        move || {
            let tag = state
                .ocr_available
                .get_untracked()
                .into_iter()
                .find(|tag| tag.to_ascii_lowercase().starts_with(prefix))
                .unwrap_or_else(|| fallback.to_string());
            state.ocr_language.set(tag);
            persist(state);
        },
        dark,
    )
}

pub(crate) fn pill(
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

fn small_button(label: String, dark: Signal<bool>, press: impl Fn() + 'static) -> impl IntoElement {
    text(move || label.clone())
        .px(px(7.0))
        .py(px(2.0))
        .rounded(px(5.0))
        .bg(move || palette(dark.get()).input)
        .text_size(px(10.0))
        .text_color(move || palette(dark.get()).muted)
        .on_press(press)
}

pub(crate) fn small_button_dynamic(
    label: impl Fn() -> String + 'static,
    dark: Signal<bool>,
    press: impl Fn() + 'static,
) -> impl IntoElement {
    text(label)
        .px(px(7.0))
        .py(px(2.0))
        .rounded(px(5.0))
        .bg(move || palette(dark.get()).input)
        .text_size(px(10.0))
        .text_color(move || palette(dark.get()).muted)
        .on_press(press)
}
