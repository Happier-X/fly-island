//! The agent status snapshot the island shows, derived from Cindy's sessions.

use serde::Serialize;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct AgentTaskStatus {
    /// "idle", "running", "completed" or "failed".
    pub(super) phase: String,
    /// The session the phase is about.
    pub(super) task_id: Option<String>,
    pub(super) updated_at: i64,
}

impl AgentTaskStatus {
    pub(super) fn idle() -> Self {
        Self {
            phase: "idle".to_string(),
            task_id: None,
            updated_at: current_unix_millis(),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentStatusSnapshot {
    pub(super) cindy: AgentTaskStatus,
    pub(super) updated_at: i64,
    /// Where the status comes from (Cindy's database), shown in Settings.
    pub(super) status_path: String,
}

pub(super) fn current_unix_millis() -> i64 {
    system_time_to_unix_millis(SystemTime::now()).unwrap_or_default()
}

pub(super) fn system_time_to_unix_millis(time: SystemTime) -> Option<i64> {
    time.duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .ok()
}
