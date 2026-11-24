//! Screenshot section in the settings window.

use std::rc::Rc;

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
        }
    }
}

fn persist(state: ScreenshotSettingsState) {
    match screenshot::save(&state.snapshot()) {
        Ok(settings) => {
            state.shortcut.set(settings.shortcut);
            state.save_format.set(settings.save_format);
            state.save_dir.set(settings.save_dir);
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

fn small_button_dynamic(
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
