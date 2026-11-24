//! Mail account settings: provider presets, credential storage and the
//! header-first fixture reader.

use std::{cell::RefCell, rc::Rc};

use nsh_core::{element::{input, scroll}, prelude::*};

use crate::mail::{self, Account, Header, Snapshot, Status};
use crate::palette;

thread_local! {
    static SERVICE: RefCell<Option<mail::Service>> = const { RefCell::new(None) };
}

#[derive(Clone, Copy)]
pub struct MailState {
    pub accounts: Signal<Vec<Account>>,
    pub provider: Signal<String>,
    pub email: Signal<String>,
    pub host: Signal<String>,
    pub secret: Signal<String>,
    pub snapshot: Signal<Snapshot>,
    pub headers: Signal<Vec<Header>>,
    pub error: Signal<String>,
    pub dirty: Signal<u64>,
}

fn with_service<R>(f: impl FnOnce(&mut mail::Service) -> R) -> Option<R> {
    SERVICE.with(|slot| slot.borrow_mut().as_mut().map(f))
}

fn apply(state: MailState, snapshot: Snapshot) {
    state.headers.set(snapshot.headers.clone());
    state.snapshot.set(snapshot);
}

fn probe(state: MailState) {
    if let Some(snapshot) = with_service(|service| service.probe()) {
        apply(state, snapshot);
    }
}

fn sync(state: MailState) {
    probe(state);
}

fn save(state: MailState) {
    let address = match mail::normalize_email(&state.email.get_untracked()) {
        Ok(address) => address,
        Err(error) => {
            state.error.set(error);
            return;
        }
    };
    let host = match mail::resolve_host(&address, &state.host.get_untracked()) {
        Ok(host) => host,
        Err(error) => {
            state.error.set(error);
            return;
        }
    };
    let secret = state.secret.get_untracked();
    if let Err(error) = mail::save_secret(&address, &secret) {
        state.error.set(error);
        return;
    }
    let account = Account { email: address, host, enabled: true };
    let result = with_service(|service| {
        service.upsert(account.clone())?;
        Ok::<_, String>(service.accounts().to_vec())
    });
    match result {
        Some(Ok(accounts)) => {
            state.accounts.set(accounts);
            state.secret.set(String::new());
            state.error.set(String::new());
            state.dirty.set(state.dirty.get_untracked() + 1);
            probe(state);
        }
        Some(Err(error)) => state.error.set(error),
        None => state.error.set("邮箱服务尚未就绪。".to_string()),
    }
}

fn remove(state: MailState, email: String) {
    let result = with_service(|service| {
        service.remove(&email)?;
        Ok::<_, String>(service.accounts().to_vec())
    });
    match result {
        Some(Ok(accounts)) => {
            state.accounts.set(accounts);
            state.error.set(String::new());
            state.dirty.set(state.dirty.get_untracked() + 1);
            if let Some(snapshot) = with_service(|service| service.snapshot()) {
                apply(state, snapshot);
            }
        }
        Some(Err(error)) => state.error.set(error),
        None => state.error.set("邮箱服务尚未就绪。".to_string()),
    }
}

fn read_body(state: MailState, id: String) {
    if let Some(snapshot) = with_service(|service| service.body(&id)) {
        apply(state, snapshot);
    }
}

fn set_provider(state: MailState, id: &str) {
    state.provider.set(id.to_string());
    if let Some(preset) = mail::PROVIDERS.iter().find(|provider| provider.id == id) {
        state.host.set(preset.host.to_string());
    }
}

fn status_label(status: Status) -> String {
    match status {
        Status::Unconfigured => t!("island.mail_unconfigured"),
        Status::Saved => t!("island.mail_saved"),
        Status::Ready => t!("island.mail_ready"),
        Status::AuthFailed => t!("island.mail_auth_failed"),
        Status::Timeout => t!("island.mail_timeout"),
        Status::Offline => t!("island.mail_offline"),
        Status::Error => t!("island.mail_error"),
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

fn pill(label: impl Fn() -> String + 'static, active: Rc<dyn Fn() -> bool + 'static>, press: impl Fn() + 'static, dark: Signal<bool>) -> impl IntoElement {
    let background = active.clone();
    view()
        .px(px(8.0))
        .py(px(2.0))
        .rounded(px(6.0))
        .bg(move || if background() { palette(dark.get()).accent } else { palette(dark.get()).input })
        .on_press(press)
        .child(
            text(label)
                .text_size(px(10.0))
                .text_color(move || if active() { palette(dark.get()).bg } else { palette(dark.get()).muted }),
        )
}

pub fn start(accounts: Vec<Account>) -> MailState {
    let provider = signal(mail::provider_for(&accounts.first().cloned().map_or_else(String::new, |account| account.email)).id.to_string());
    let email = signal(String::new());
    let host = signal(String::new());
    let secret = signal(String::new());
    let accounts_signal = signal(accounts.clone());
    let snapshot = signal(Snapshot::default());
    let headers = signal(Vec::new());
    let error = signal(String::new());
    let dirty = signal(0u64);
    SERVICE.with(|slot| {
        let mut service = mail::Service::new(accounts.clone());
        if !accounts.is_empty() && std::env::var_os("NSH_MAIL_FIXTURE").is_some() {
            service.probe();
        }
        *slot.borrow_mut() = Some(service);
    });
    if let Some(mut initial) = with_service(|service| service.snapshot()) {
        if !accounts.is_empty()
            && std::env::var_os("NSH_MAIL_FIXTURE").is_none()
            && !accounts.iter().any(|account| mail::has_secret(&account.email).unwrap_or(false))
        {
            initial.status = Status::Unconfigured;
        }
        snapshot.set(initial);
    }
    MailState { accounts: accounts_signal, provider, email, host, secret, snapshot, headers, error, dirty }
}

pub fn settings_section(state: MailState, dark: Signal<bool>) -> impl IntoElement {
    let needs_retry = move || {
        matches!(
            state.snapshot.get().status,
            Status::Unconfigured | Status::AuthFailed | Status::Timeout | Status::Offline | Status::Error
        )
    };
    view()
        .flex_col()
        .gap(px(5.0))
        .child(
            view()
                .flex_row()
                .items_center()
                .gap(px(6.0))
                .child(
                    text(move || t!("island.mail_title"))
                        .text_size(px(12.0))
                        .text_color(move || palette(dark.get()).muted)
                        .mt(px(6.0)),
                )
                .child(
                    text(move || status_label(state.snapshot.get().status))
                        .text_size(px(10.0))
                        .text_color(move || match state.snapshot.get().status {
                            Status::Ready => palette(dark.get()).success,
                            Status::AuthFailed | Status::Timeout | Status::Offline | Status::Error => palette(dark.get()).warning,
                            _ => palette(dark.get()).faint,
                        }),
                )
                .child(view().flex_1())
                .child(mini(move || t!("island.mail_sync"), dark, move || sync(state)))
                .child(when(needs_retry, move || mini(move || t!("island.mail_retry"), dark, move || probe(state)))),
        )
        .child(
            text(move || t!("island.mail_hint"))
                .text_size(px(10.0))
                .text_color(move || palette(dark.get()).faint),
        )
        .child(
            view()
                .flex_row()
                .gap(px(4.0))
                .flex_wrap()
                .children(mail::PROVIDERS.iter().map(|provider| {
                    let id = provider.id;
                    pill(
                        move || provider.label.to_string(),
                        Rc::new(move || state.provider.get() == id),
                        move || set_provider(state, id),
                        dark,
                    )
                })),
        )
        .child(
            view()
                .flex_row()
                .gap(px(5.0))
                .items_center()
                .child(
                    input()
                        .value(move || state.email.get())
                        .placeholder(move || t!("island.mail_email"))
                        .text_color(move || palette(dark.get()).fg)
                        .bg(move || palette(dark.get()).input)
                        .h(px(28.0))
                        .flex_1()
                        .px(px(8.0))
                        .rounded(px(6.0))
                        .on_change(move |text: &str| state.email.set(text.to_string())),
                )
                .child(
                    input()
                        .value(move || state.host.get())
                        .placeholder(move || t!("island.mail_host"))
                        .text_color(move || palette(dark.get()).fg)
                        .bg(move || palette(dark.get()).input)
                        .h(px(28.0))
                        .w(px(150.0))
                        .px(px(8.0))
                        .rounded(px(6.0))
                        .on_change(move |text: &str| state.host.set(text.to_string())),
                ),
        )
        .child(
            view()
                .flex_row()
                .gap(px(5.0))
                .items_center()
                .child(
                    input()
                        .value(move || state.secret.get())
                        .placeholder(move || t!("island.mail_auth_code"))
                        .text_color(move || palette(dark.get()).fg)
                        .bg(move || palette(dark.get()).input)
                        .h(px(28.0))
                        .flex_1()
                        .px(px(8.0))
                        .rounded(px(6.0))
                        .on_change(move |text: &str| state.secret.set(text.to_string())),
                )
                .child(mini(move || t!("island.mail_save"), dark, move || save(state))),
        )
        .child(when(move || !state.error.get().is_empty(), move || {
            text(move || state.error.get())
                .text_size(px(10.0))
                .text_color(move || palette(dark.get()).warning)
        }))
        .child(when(move || state.snapshot.get().error.is_some(), move || {
            text(move || state.snapshot.get().error.clone().unwrap_or_default())
                .text_size(px(10.0))
                .text_color(move || palette(dark.get()).warning)
        }))
        .child(each(state.accounts, |account| account.email.clone(), move |account, _| {
            let item = account.get();
            let email_text = item.email.clone();
            let email_remove = item.email.clone();
            view()
                .flex_row()
                .items_center()
                .gap(px(5.0))
                .child(
                    text(move || email_text.clone())
                        .flex_1()
                        .truncate()
                        .text_size(px(10.0))
                        .text_color(move || palette(dark.get()).muted),
                )
                .child(
                    text(move || item.host.clone())
                        .text_size(px(9.0))
                        .text_color(move || palette(dark.get()).faint),
                )
                .child(mini(|| "✕".to_string(), dark, move || remove(state, email_remove.clone())))
        }))
        .child(when(move || !state.snapshot.get().headers.is_empty(), move || {
            view()
                .flex_col()
                .gap(px(2.0))
                .child(
                    text(move || t!("island.mail_headers"))
                        .text_size(px(10.0))
                        .text_color(move || palette(dark.get()).muted),
                )
                .child(each(state.headers, |header| header.id.clone(), move |header, _| {
                    let item = header.get();
                    let id = item.id.clone();
                    let title = format!("{} — {}", item.from, item.subject);
                    let date = item.date.clone();
                    let unread = item.unread;
                    view()
                        .flex_row()
                        .items_center()
                        .gap(px(5.0))
                        .px(px(6.0))
                        .py(px(3.0))
                        .rounded(px(5.0))
                        .bg(move || palette(dark.get()).input)
                        .child(
                            text(move || title.clone())
                                .flex_1()
                                .truncate()
                                .text_size(px(10.0))
                                .text_color(move || if unread { palette(dark.get()).fg } else { palette(dark.get()).muted }),
                        )
                        .child(
                            text(move || date.clone())
                                .text_size(px(8.5))
                                .text_color(move || palette(dark.get()).faint),
                        )
                        .child(mini(move || t!("island.mail_read_body"), dark, move || read_body(state, id.clone())))
                }))
        }))
        .child(when(move || !state.snapshot.get().body.is_empty(), move || {
            view()
                .max_h(px(110.0))
                .w_full()
                .rounded(px(6.0))
                .bg(move || palette(dark.get()).input)
                .p(px(7.0))
                .child(
                    scroll().scrollbar(false).child(
                        text(move || state.snapshot.get().body.clone())
                            .text_size(px(10.0))
                            .text_color(move || palette(dark.get()).muted),
                    ),
                )
        }))
}
