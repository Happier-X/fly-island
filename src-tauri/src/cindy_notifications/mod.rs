//! Cindy 通知：Cindy 做完了、在等你回答、跑完的定时与后台任务、以及出错。
//!
//! Cindy 是这台机器上唯一跑 agent 的程序，它自己的本地数据库是数据源（`cindy.rs`）。
//! 岛上只显示一个「有未处理通知」的角标，点开后把 Cindy 的窗口拿到前台，
//! 不在岛上展开面板、不显示会话列表。v3.1.0 及以前写入 Claude Code / Codex 的钩子已经废弃，
//! `legacy_hooks.rs` 负责把它们一次性删掉。

mod cindy;
mod legacy_hooks;
mod window_focus;

use std::time::{SystemTime, UNIX_EPOCH};

use tauri::AppHandle;

pub(crate) use cindy::CindyNotificationSnapshot;

/// 读一次 Cindy 的数据库：四类事件、数据库路径、读取时间和失败原因。
///
/// 读不出来时返回的快照带着原因（设置页显示），命令本身不失败：
/// Cindy 没装、还没建库或正被它自己独占，都不该让岛的轮询报错。
#[tauri::command]
pub async fn get_cindy_notifications() -> Result<CindyNotificationSnapshot, String> {
    tauri::async_runtime::spawn_blocking(cindy::scan)
        .await
        .map_err(|error| format!("Failed to join the Cindy notification task: {error}"))
}

/// Brings Cindy to the front. False when it is not running or has no window.
#[tauri::command]
pub async fn focus_agent_window() -> Result<bool, String> {
    tauri::async_runtime::spawn_blocking(|| window_focus::focus_app_window("cindy"))
        .await
        .map_err(|error| format!("Failed to join the window focus task: {error}"))
}

/// Whether Cindy is the window in front: the island then keeps quiet about new notifications.
#[tauri::command]
pub async fn agent_window_in_front() -> Result<bool, String> {
    tauri::async_runtime::spawn_blocking(|| window_focus::app_window_in_front("cindy"))
        .await
        .map_err(|error| format!("Failed to join the window focus task: {error}"))
}

/// Removes the hook entries older builds wrote into Claude Code and Codex settings, once.
pub(crate) fn clean_up_legacy_hooks(app: &AppHandle) -> Result<(), String> {
    legacy_hooks::run_once(app)
}

pub(super) fn now_unix_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or_default()
}

/// This module's Tauri commands. `lib.rs` chains every module's list into one
/// `generate_handler!`, so adding a command only touches this file.
macro_rules! tauri_commands {
    ($next:ident [$($done:tt)*] $($rest:tt)*) => {
        $next!([$($done)*
            crate::cindy_notifications::get_cindy_notifications,
            crate::cindy_notifications::focus_agent_window,
            crate::cindy_notifications::agent_window_in_front,
        ] $($rest)*)
    };
}
pub(crate) use tauri_commands;
