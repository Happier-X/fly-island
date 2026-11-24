//! Background update checks and the update section in settings.

use std::{sync::Mutex, time::Duration};

use nsh_core::{prelude::*, Color};

use crate::{palette, update};

#[derive(Clone, Copy)]
pub struct UpdateState {
    pub endpoint: Signal<String>,
    pub status: Signal<String>,
    pub available: Signal<Option<update::Available>>,
    pub busy: Signal<bool>,
    pub error: Signal<String>,
}

enum Event {
    Check(Result<Option<update::Available>, String>),
    Install(Result<(), String>),
}

static EVENT: Mutex<Option<Event>> = Mutex::new(None);

pub fn start(endpoint: String, auto_check: bool) -> UpdateState {
    let state = UpdateState {
        endpoint: signal(endpoint),
        status: signal(String::new()),
        available: signal(None),
        busy: signal(false),
        error: signal(String::new()),
    };
    spawn(async move {
        if auto_check && !state.endpoint.get_untracked().trim().is_empty() {
            check_now(state);
        }
        loop {
            timer(Duration::from_millis(150)).await;
            let Some(event) = EVENT.lock().ok().and_then(|mut event| event.take()) else {
                continue;
            };
            state.busy.set(false);
            match event {
                Event::Check(Ok(Some(available))) => {
                    state.status.set(format!("发现新版本 {}", available.version));
                    state.available.set(Some(available));
                    state.error.set(String::new());
                }
                Event::Check(Ok(None)) => {
                    state.status.set("已是最新版本".to_string());
                    state.available.set(None);
                    state.error.set(String::new());
                }
                Event::Check(Err(error)) | Event::Install(Err(error)) => {
                    state.error.set(error);
                }
                Event::Install(Ok(())) => {
                    state.status.set("更新已应用，正在重启".to_string());
                    state.error.set(String::new());
                    if std::env::var_os("NSH_UPDATE_TARGET_DIR").is_none() {
                        std::process::exit(0);
                    }
                }
            }
        }
    });
    state
}

pub fn check_now(state: UpdateState) {
    if state.busy.get_untracked() {
        return;
    }
    let endpoint = state.endpoint.get_untracked();
    if endpoint.trim().is_empty() {
        state.status.set("未配置更新地址".to_string());
        return;
    }
    state.busy.set(true);
    state.status.set("正在检查更新…".to_string());
    state.error.set(String::new());
    std::thread::spawn(move || {
        let result = update::check(&endpoint);
        if let Ok(mut event) = EVENT.lock() {
            *event = Some(Event::Check(result));
        }
    });
}

pub fn install_now(state: UpdateState) {
    let Some(available) = state.available.get_untracked() else {
        return;
    };
    if state.busy.get_untracked() {
        return;
    }
    state.busy.set(true);
    state.status.set("正在下载并校验更新…".to_string());
    state.error.set(String::new());
    std::thread::spawn(move || {
        let result = (|| {
            let staging = std::env::temp_dir().join("focusd-island-nsh-update");
            let root = update::download_and_stage(&available, &staging)?;
            let target = update::target_dir()?;
            update::install_and_restart(&root, &target)
        })();
        if let Ok(mut event) = EVENT.lock() {
            *event = Some(Event::Install(result));
        }
    });
}

pub fn signature_note(available: &update::Available) -> String {
    match &available.signature {
        Some(_) => "清单含签名字段，但当前未配置公钥，签名未验证".to_string(),
        None => "未签名包：只校验 SHA-256；Windows SmartScreen 可能提示".to_string(),
    }
}

pub fn status_color(state: UpdateState, dark: Signal<bool>) -> Color {
    if !state.error.get().is_empty() {
        palette(dark.get()).warning
    } else if state.busy.get() {
        palette(dark.get()).muted
    } else {
        palette(dark.get()).accent
    }
}
