//! Removes the hooks older builds installed into Claude Code and Codex.
//!
//! Up to v3.1.0 the island wrote status hooks into `~/.claude/settings.json` and a managed
//! block in `~/.codex/config.toml`, and those hooks are still there on machines that ran it.
//! The AI section now reads Cindy's own database instead, so the hooks have nobody to report
//! to and are removed once. Only entries carrying the island's own signature go: anything the
//! user wrote by hand stays.

use std::path::Path;

use serde_json::Value;
use tauri::{AppHandle, Manager};

use crate::system::windows_home_dir;

/// Every command the island wrote carries this in its path or arguments.
const HOOK_SIGNATURE: &str = "focusd-agent-";
const CODEX_BLOCK_BEGIN: &str = "# BEGIN FocuSD Agent Status Hooks";
const CODEX_BLOCK_END: &str = "# END FocuSD Agent Status Hooks";
const DONE_FLAG: &str = "legacy-agent-hooks-removed.flag";

/// Runs the cleanup at most once per install, recording it in the app data folder.
pub(super) fn run_once(app: &AppHandle) -> Result<(), String> {
    let app_dir = app
        .path()
        .app_data_dir()
        .map_err(|error| format!("Failed to locate the app data folder: {error}"))?;
    let flag = app_dir.join(DONE_FLAG);
    if flag.exists() {
        return Ok(());
    }
    let home_dir = windows_home_dir()?;
    // A failure leaves the flag unwritten, so the next launch tries again.
    remove_claude_hooks(&home_dir.join(".claude").join("settings.json"))?;
    remove_codex_hooks(&home_dir.join(".codex").join("config.toml"))?;
    std::fs::create_dir_all(&app_dir)
        .map_err(|error| format!("Failed to create {}: {error}", app_dir.display()))?;
    std::fs::write(&flag, "1")
        .map_err(|error| format!("Failed to write {}: {error}", flag.display()))
}

/// Drops the island's entries from the `hooks` object, keeping the user's own.
fn remove_claude_hooks(path: &Path) -> Result<(), String> {
    let Ok(text) = std::fs::read_to_string(path) else {
        // No Claude Code settings: nothing to clean.
        return Ok(());
    };
    let mut settings: Value = match serde_json::from_str(&text) {
        Ok(settings) => settings,
        Err(_) => return Ok(()),
    };
    let Some(events) = settings.get_mut("hooks").and_then(Value::as_object_mut) else {
        return Ok(());
    };

    let mut changed = false;
    let event_names: Vec<String> = events.keys().cloned().collect();
    for event in event_names {
        let Some(entries) = events.get_mut(&event).and_then(Value::as_array_mut) else {
            continue;
        };
        let before = entries.len();
        entries.retain(|entry| !mentions_signature(entry));
        if entries.len() != before {
            changed = true;
        }
        if entries.is_empty() {
            events.remove(&event);
        }
    }
    if !changed {
        return Ok(());
    }
    if events.is_empty() {
        if let Some(object) = settings.as_object_mut() {
            object.remove("hooks");
        }
    }
    write_json(path, &settings)
}

/// True when the hook entry runs one of the island's scripts.
fn mentions_signature(value: &Value) -> bool {
    match value {
        Value::String(text) => text.contains(HOOK_SIGNATURE),
        Value::Array(items) => items.iter().any(mentions_signature),
        Value::Object(object) => object.values().any(mentions_signature),
        _ => false,
    }
}

/// Drops the managed block from Codex's config.
fn remove_codex_hooks(path: &Path) -> Result<(), String> {
    let Ok(text) = std::fs::read_to_string(path) else {
        return Ok(());
    };
    let Some(start) = text.find(CODEX_BLOCK_BEGIN) else {
        return Ok(());
    };
    let Some(relative_end) = text[start..].find(CODEX_BLOCK_END) else {
        return Ok(());
    };
    let end = start + relative_end + CODEX_BLOCK_END.len();
    let mut cleaned = String::with_capacity(text.len());
    cleaned.push_str(&text[..start]);
    // Swallow the blank line the block left behind, so the file does not grow a gap.
    let rest = text[end..].trim_start_matches(['\r', '\n']);
    cleaned.push_str(rest);
    let cleaned = format!("{}\n", cleaned.trim_end());
    if cleaned.trim().is_empty() {
        std::fs::remove_file(path).map_err(|error| format!("Failed to remove {}: {error}", path.display()))?;
        return Ok(());
    }
    std::fs::write(path, cleaned).map_err(|error| format!("Failed to write {}: {error}", path.display()))
}

fn write_json(path: &Path, value: &Value) -> Result<(), String> {
    let text = serde_json::to_string_pretty(value)
        .map_err(|error| format!("Failed to serialize {}: {error}", path.display()))?;
    let temporary = path.with_extension("tmp");
    std::fs::write(&temporary, text).map_err(|error| format!("Failed to write {}: {error}", temporary.display()))?;
    std::fs::rename(&temporary, path).map_err(|error| format!("Failed to replace {}: {error}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn the_islands_own_entries_are_recognised() {
        assert!(mentions_signature(&json!({
            "hooks": [{ "type": "command", "command": "pwsh -File C:\\x\\focusd-agent-status.ps1" }]
        })));
        assert!(!mentions_signature(&json!({
            "hooks": [{ "type": "command", "command": "echo hi" }]
        })));
        assert!(!mentions_signature(&json!(42)));
    }
}
