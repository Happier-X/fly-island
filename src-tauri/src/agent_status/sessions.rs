//! Agent sessions for the island's Agent page, read from Cindy's local database (cindy/mod.rs).
//!
//! Cindy is what actually runs the agent sessions on this machine, and its database already
//! records what each one is doing, so nothing here reads engine session files.

use serde::Serialize;

use super::cindy::CindySession;

/// The latest reply shown under each session: two lines on the Agent page, up to three on an
/// alert's card (about 36 CJK or 70 Latin characters a line).
pub(super) const LAST_MESSAGE_MAX_CHARS: usize = 200;

/// The provider id the frontend keys everything off.
pub(super) const CINDY_PROVIDER: &str = "cindy";

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentSessionSummary {
    pub(super) provider: &'static str,
    pub(super) session_id: String,
    pub(super) cwd: Option<String>,
    pub(super) title: Option<String>,
    /// The model Cindy runs this session with, as Cindy names it.
    pub(super) model: Option<String>,
    /// "running", "waiting" (asked the user mid-turn), "completed" or "failed".
    pub(super) phase: &'static str,
    pub(super) last_message: Option<String>,
    /// Last change of the session, Unix milliseconds.
    pub(super) updated_at: i64,
    /// When the current turn started, Unix milliseconds; `None` when it is not running.
    pub(super) turn_started_at: Option<i64>,
    /// "default", "auto", "bypass", "acceptEdits" or "plan"; `None` when unknown.
    pub(super) permission_mode: Option<&'static str>,
    /// Tokens in the context now; shown on the Agent page's cards.
    pub(super) context_tokens: Option<u64>,
    /// The model's context window, for the occupancy a card shows.
    pub(super) context_window: Option<u64>,
    /// Tokens this session has spent in total, and what they cost.
    pub(super) total_tokens: u64,
    pub(super) cost_usd: f64,
}

/// A Cindy session as the island's Agent page shows it.
pub(super) fn from_cindy(session: CindySession) -> AgentSessionSummary {
    let phase = phase_of(&session);
    let turn_started_at = if session.running {
        session.turn_started_at
    } else {
        None
    };
    AgentSessionSummary {
        provider: CINDY_PROVIDER,
        session_id: session.id,
        cwd: session.cwd.filter(|cwd| !cwd.trim().is_empty()),
        title: session.title.map(|title| clamp(&title, LAST_MESSAGE_MAX_CHARS)),
        model: session.model.filter(|model| !model.trim().is_empty()),
        phase,
        last_message: session
            .last_message
            .map(|message| clamp(&message, LAST_MESSAGE_MAX_CHARS))
            .filter(|message| !message.is_empty()),
        updated_at: session.updated_at,
        turn_started_at,
        permission_mode: permission_mode_of(session.permission_mode.as_deref()),
        context_tokens: session.context_tokens,
        context_window: session.context_window,
        total_tokens: session.total_tokens,
        cost_usd: session.cost_usd,
    }
}

fn phase_of(session: &CindySession) -> &'static str {
    if session.waiting {
        "waiting"
    } else if session.running {
        "running"
    } else {
        "completed"
    }
}

/// Cindy stores the permission mode as free text; the page only knows these.
fn permission_mode_of(mode: Option<&str>) -> Option<&'static str> {
    match mode?.trim() {
        "default" => Some("default"),
        "auto" => Some("auto"),
        "bypass" | "bypassPermissions" => Some("bypass"),
        "acceptEdits" => Some("acceptEdits"),
        "plan" => Some("plan"),
        _ => None,
    }
}

fn clamp(text: &str, max_chars: usize) -> String {
    let trimmed = text.trim();
    if trimmed.chars().count() <= max_chars {
        return trimmed.to_string();
    }
    let mut shortened: String = trimmed.chars().take(max_chars).collect();
    shortened.push('…');
    shortened
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cindy(running: bool, waiting: bool) -> CindySession {
        CindySession {
            id: "abc123".to_string(),
            title: Some("  修一下登录页  ".to_string()),
            cwd: Some(r"C:\code\app".to_string()),
            model: Some("claude-sonnet-4-6".to_string()),
            permission_mode: Some("plan".to_string()),
            running,
            waiting,
            updated_at: 1_700_000_000_000,
            turn_started_at: Some(1_699_999_999_000),
            last_message: Some("改好了，跑一下测试".to_string()),
            context_tokens: Some(12_000),
            context_window: Some(200_000),
            total_tokens: 34_000,
            cost_usd: 0.12,
        }
    }

    #[test]
    fn a_running_session_is_running() {
        let summary = from_cindy(cindy(true, false));
        assert_eq!(summary.phase, "running");
        assert_eq!(summary.provider, CINDY_PROVIDER);
    }

    #[test]
    fn a_session_waiting_on_an_answer_says_so() {
        assert_eq!(from_cindy(cindy(true, true)).phase, "waiting");
    }

    #[test]
    fn a_finished_session_is_completed() {
        assert_eq!(from_cindy(cindy(false, false)).phase, "completed");
    }

    #[test]
    fn a_finished_session_has_no_turn_under_way() {
        assert_eq!(from_cindy(cindy(true, false)).turn_started_at, Some(1_699_999_999_000));
        assert_eq!(from_cindy(cindy(false, false)).turn_started_at, None);
    }

    #[test]
    fn titles_and_messages_are_trimmed_and_clamped() {
        let summary = from_cindy(cindy(true, false));
        assert_eq!(summary.title.as_deref(), Some("修一下登录页"));
        let long = "字".repeat(LAST_MESSAGE_MAX_CHARS + 20);
        let mut session = cindy(true, false);
        session.last_message = Some(long);
        let summary_long = from_cindy(session);
        let message = summary_long.last_message.unwrap_or_default();
        assert_eq!(message.chars().count(), LAST_MESSAGE_MAX_CHARS + 1, "截断后带省略号");
        assert!(message.ends_with('…'));
    }

    #[test]
    fn empty_fields_become_none_instead_of_blank_text() {
        let mut session = cindy(true, false);
        session.cwd = Some("   ".to_string());
        session.model = Some(String::new());
        session.last_message = Some("  ".to_string());
        let summary = from_cindy(session);
        assert_eq!(summary.cwd, None);
        assert_eq!(summary.model, None);
        assert_eq!(summary.last_message, None);
    }

    #[test]
    fn unknown_permission_modes_are_dropped() {
        assert_eq!(permission_mode_of(Some("plan")), Some("plan"));
        assert_eq!(permission_mode_of(Some("bypassPermissions")), Some("bypass"));
        assert_eq!(permission_mode_of(Some("something-new")), None);
        assert_eq!(permission_mode_of(None), None);
    }
}
