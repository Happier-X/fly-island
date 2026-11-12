//! AI agent status: the sessions Cindy runs, shown on the island.
//!
//! Cindy is the only agent host this build knows about. Its own database (cindy/mod.rs) is the
//! source: which sessions are running, which one waits on an answer, what finished recently.
//! The Codex / Claude Code hooks this module used to install are gone; the migration in
//! `legacy_hooks.rs` removes the ones already written to disk.

mod cindy;
mod legacy_hooks;
mod sessions;
mod snapshot;
pub(crate) mod sound_packs;
mod window_focus;

use tauri::AppHandle;

use snapshot::AgentStatusSnapshot;

/// Cindy 会话多久没动就不再算「最近」。
pub(super) const ACTIVE_WINDOW_MS: i64 = 2 * 60 * 60 * 1000;

#[tauri::command]
pub async fn get_agent_status() -> Result<AgentStatusSnapshot, String> {
    tauri::async_runtime::spawn_blocking(|| {
        let status_path = cindy::database_path()
            .map(|path| path.to_string_lossy().to_string())
            .unwrap_or_default();
        match cindy::list_sessions(ACTIVE_WINDOW_MS) {
            Ok(sessions) => cindy::status_snapshot(&sessions, status_path),
            // Cindy 没装、库还没建好、或正被它自己独占：岛上就是不显示，不是错误。
            Err(_) => cindy::status_snapshot(&[], status_path),
        }
    })
    .await
    .map_err(|error| format!("Failed to join the agent status task: {error}"))
}

/// Cindy sessions active recently, running and waiting ones first.
///
/// `window_hours` (1-24, default 2): how long ago a listed session may have last changed.
#[tauri::command]
pub async fn list_agent_sessions(
    window_hours: Option<u32>,
) -> Result<Vec<sessions::AgentSessionSummary>, String> {
    let window_ms = window_hours
        .map(|hours| i64::from(hours.clamp(1, 24)) * 60 * 60 * 1000)
        .unwrap_or(ACTIVE_WINDOW_MS);
    tauri::async_runtime::spawn_blocking(move || {
        cindy::list_sessions(window_ms)
            .map(|sessions| sessions.into_iter().map(sessions::from_cindy).collect())
    })
    .await
    .map_err(|error| format!("Failed to join the agent sessions task: {error}"))?
}

/// Brings Cindy to the front. False when it is not running or has no window.
#[tauri::command]
pub async fn focus_agent_window() -> Result<bool, String> {
    tauri::async_runtime::spawn_blocking(|| window_focus::focus_app_window("cindy"))
        .await
        .map_err(|error| format!("Failed to join the window focus task: {error}"))
}

/// Whether Cindy is the window in front: the island then leaves its alerts alone.
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

/// This module's Tauri commands. `lib.rs` chains every module's list into one
/// `generate_handler!`, so adding a command only touches this file.
macro_rules! tauri_commands {
    ($next:ident [$($done:tt)*] $($rest:tt)*) => {
        $next!([$($done)*
            crate::agent_status::get_agent_status,
            crate::agent_status::list_agent_sessions,
            crate::agent_status::focus_agent_window,
            crate::agent_status::agent_window_in_front,
            crate::agent_status::sound_packs::list_sound_packs,
            crate::agent_status::sound_packs::import_sound_pack,
            crate::agent_status::sound_packs::remove_sound_pack,
            crate::agent_status::sound_packs::read_sound_pack_file,
            crate::agent_status::sound_packs::open_sound_pack_folder,
        ] $($rest)*)
    };
}
pub(crate) use tauri_commands;
