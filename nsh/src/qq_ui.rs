//! QQ notification panel backed by the fixture toast pipeline.

use std::{cell::RefCell, time::Duration};

use nsh_core::{element::scroll, prelude::*};

use crate::palette;
use crate::qq::{self, Notification, Snapshot, Status};

thread_local! {
    static SERVICE: RefCell<Option<qq::Service>> = const { RefCell::new(None) };
}

#[derive(Clone, Copy)]
pub struct QqState {
    pub snapshot: Signal<Snapshot>,
    pub items: Signal<Vec<Notification>>,
}

fn with_service<R>(f: impl FnOnce(&mut qq::Service) -> R) -> Option<R> {
    SERVICE.with(|slot| slot.borrow_mut().as_mut().map(f))
}

fn apply(state: QqState, snapshot: Snapshot) {
    state.items.set(snapshot.items.clone());
    state.snapshot.set(snapshot);
}

fn poll(state: QqState) {
    if let Some(snapshot) = with_service(|service| service.poll()) {
        apply(state, snapshot);
    }
}

pub fn start(enabled: Signal<bool>) -> QqState {
    let snapshot = signal(Snapshot::default());
    let items = signal(Vec::new());
    SERVICE.with(|slot| {
        *slot.borrow_mut() = Some(qq::Service::new(enabled.get_untracked()));
    });
    let state = QqState { snapshot, items };
    effect(move || {
        let on = enabled.get();
        if let Some(next) = with_service(|service| {
            service.set_enabled(on);
            service.poll()
        }) {
            apply(state, next);
        }
    });
    set_interval(Duration::from_secs(1), move || poll(state));
    poll(state);
    state
}

fn open(state: QqState) {
    if let Some(Err(error)) = with_service(|service| service.open()) {
        let mut snapshot = state.snapshot.get_untracked();
        snapshot.error = Some(error);
        state.snapshot.set(snapshot);
    }
}

fn clear(state: QqState) {
    if let Some(next) = with_service(|service| {
        service.clear();
        service.snapshot()
    }) {
        apply(state, next);
    }
}

fn retry(state: QqState) {
    poll(state);
}

fn status_label(status: Status) -> String {
    match status {
        Status::Disabled => t!("island.qq_disabled"),
        Status::Waiting => t!("island.qq_waiting"),
        Status::NotRunning => t!("island.qq_not_running"),
        Status::Listening => t!("island.qq_listening"),
        Status::PermissionDenied => t!("island.qq_permission_denied"),
        Status::PermissionRequired => t!("island.qq_permission_required"),
        Status::Error => t!("island.qq_error"),
    }
}

fn mini(label: impl Fn() -> String + 'static, dark: Signal<bool>, press: impl Fn() + 'static) -> impl IntoElement {
    text(label)
        .flex_shrink(0.0)
        .px(px(7.0))
        .py(px(2.0))
        .rounded(px(5.0))
        .bg(move || palette(dark.get()).input)
        .text_size(px(10.0))
        .text_color(move || palette(dark.get()).muted)
        .on_press(press)
}

pub fn panel(state: QqState, dark: Signal<bool>) -> impl IntoElement {
    let needs_retry = move || {
        matches!(
            state.snapshot.get().status,
            Status::NotRunning | Status::Error | Status::PermissionDenied | Status::PermissionRequired
        )
    };
    view()
        .flex_col()
        .gap(px(5.0))
        .px(px(18.0))
        .py(px(6.0))
        .child(
            view()
                .flex_row()
                .items_center()
                .gap(px(6.0))
                .child(
                    text(move || t!("island.qq_title"))
                        .text_size(px(12.0))
                        .text_color(move || palette(dark.get()).muted),
                )
                .child(
                    text(move || status_label(state.snapshot.get().status))
                        .text_size(px(10.0))
                        .text_color(move || match state.snapshot.get().status {
                            Status::Listening => palette(dark.get()).success,
                            Status::Error | Status::PermissionDenied | Status::PermissionRequired => palette(dark.get()).warning,
                            _ => palette(dark.get()).faint,
                        }),
                )
                .child(view().flex_1()),
        )
        .child(
            view()
                .flex_row()
                .justify_end()
                .gap(px(4.0))
                .child(mini(move || t!("island.qq_open"), dark, move || open(state)))
                .child(when(needs_retry, move || mini(move || t!("island.qq_retry"), dark, move || retry(state))))
                .child(mini(move || t!("island.qq_clear"), dark, move || clear(state))),
        )
        .child(when(
            move || state.snapshot.get().error.is_some(),
            move || {
                text(move || state.snapshot.get().error.clone().unwrap_or_default())
                    .text_size(px(10.0))
                    .text_color(move || palette(dark.get()).warning)
            },
        ))
        .child(
            view()
                .max_h(px(132.0))
                .w_full()
                .child(
                    scroll().scrollbar(false).child(
                        view()
                            .flex_col()
                            .gap(px(3.0))
                            .child(when(move || state.items.get().is_empty(), move || {
                                text(move || t!("island.qq_empty"))
                                    .text_size(px(11.0))
                                    .text_color(move || palette(dark.get()).faint)
                            }))
                            .child(each(state.items, |item| item.id.clone(), move |item, _| {
                                let result = item.get();
                                let body_check = result.body.clone();
                                let body_text = result.body.clone();
                                view()
                                    .flex_col()
                                    .gap(px(1.0))
                                    .px(px(7.0))
                                    .py(px(4.0))
                                    .rounded(px(6.0))
                                    .bg(move || palette(dark.get()).input)
                                    .on_press(move || open(state))
                                    .child(
                                        text(move || result.title.clone())
                                            .text_size(px(11.0))
                                            .text_color(move || palette(dark.get()).fg),
                                    )
                                    .child(when(move || !body_check.is_empty(), move || {
                                        text({
                                            let value = body_text.clone();
                                            move || value.clone()
                                        })
                                            .truncate()
                                            .text_size(px(9.5))
                                            .text_color(move || palette(dark.get()).faint)
                                    }))
                            })),
                    ),
                ),
        )
}
