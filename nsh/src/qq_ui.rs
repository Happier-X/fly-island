//! QQ notification panel backed by the fixture toast pipeline.

use std::{cell::RefCell, sync::mpsc, thread, time::Duration};

use nsh_core::{element::scroll, prelude::*};

use crate::palette;
use crate::qq::{self, Notification, Snapshot, Status};

enum Command {
    Enabled(bool),
    Clear,
    Poll,
}

thread_local! {
    static COMMANDS: RefCell<Option<mpsc::Sender<Command>>> = const { RefCell::new(None) };
    static SNAPSHOTS: RefCell<Option<mpsc::Receiver<Snapshot>>> = const { RefCell::new(None) };
}

#[derive(Clone, Copy)]
pub struct QqState {
    pub snapshot: Signal<Snapshot>,
    pub items: Signal<Vec<Notification>>,
}

fn send(command: Command) {
    COMMANDS.with(|slot| {
        if let Some(sender) = slot.borrow().as_ref() {
            let _ = sender.send(command);
        }
    });
}

fn apply(state: QqState, snapshot: Snapshot) {
    state.items.set(snapshot.items.clone());
    state.snapshot.set(snapshot);
}

fn drain(state: QqState) {
    SNAPSHOTS.with(|slot| {
        if let Some(receiver) = slot.borrow().as_ref() {
            while let Ok(snapshot) = receiver.try_recv() {
                apply(state, snapshot);
            }
        }
    });
}

pub fn start(enabled: Signal<bool>) -> QqState {
    let snapshot = signal(Snapshot::default());
    let items = signal(Vec::new());
    let (command_tx, command_rx) = mpsc::channel();
    let (snapshot_tx, snapshot_rx) = mpsc::channel();
    COMMANDS.with(|slot| *slot.borrow_mut() = Some(command_tx));
    SNAPSHOTS.with(|slot| *slot.borrow_mut() = Some(snapshot_rx));
    let initial = enabled.get_untracked();
    thread::spawn(move || {
        let mut service = qq::Service::new(initial);
        let _ = snapshot_tx.send(service.snapshot());
        loop {
            match command_rx.recv_timeout(Duration::from_secs(1)) {
                Ok(Command::Enabled(on)) => {
                    service.set_enabled(on);
                    let _ = snapshot_tx.send(service.poll());
                }
                Ok(Command::Clear) => {
                    service.clear();
                    let _ = snapshot_tx.send(service.snapshot());
                }
                Ok(Command::Poll) => {
                    let _ = snapshot_tx.send(service.poll());
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    let _ = snapshot_tx.send(service.poll());
                }
            }
        }
    });
    let state = QqState { snapshot, items };
    effect(move || send(Command::Enabled(enabled.get())));
    set_interval(Duration::from_millis(250), move || drain(state));
    state
}

fn open(state: QqState) {
    if let Err(error) = qq::open_qq() {
        let mut snapshot = state.snapshot.get_untracked();
        snapshot.error = Some(error);
        state.snapshot.set(snapshot);
    }
}

fn clear(_state: QqState) {
    send(Command::Clear);
}

fn retry(_state: QqState) {
    send(Command::Poll);
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
        .min_w(px(34.0))
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
                .child(when(
                    move || matches!(
                        state.snapshot.get().status,
                        Status::PermissionDenied | Status::PermissionRequired
                    ),
                    move || mini(move || t!("island.qq_settings"), dark, move || qq::request_access()),
                ))
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
