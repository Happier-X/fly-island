//! AI sessions and Cindy notifications.
//!
//! Cindy's own SQLite database is the only source, read-only: active Codex /
//! Claude Code sessions become running, done or needs-attention states, and the
//! four notification kinds mirror the Tauri app (done, question, scheduled,
//! error). A local watermark keeps events the user already saw from coming back;
//! clicking the AI strip brings Cindy's window forward.

use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    rc::Rc,
    time::Duration,
};

use nsh_core::prelude::*;
use rusqlite::{Connection, OpenFlags, OptionalExtension};
use serde::{Deserialize, Serialize};

const POLL: Duration = Duration::from_secs(5);
const EVENT_WINDOW_MS: i64 = 24 * 60 * 60 * 1000;
const SESSION_WINDOW_MS: i64 = 2 * 60 * 60 * 1000;
const DONE_WINDOW_MS: i64 = 10 * 60 * 1000;
const WAITING_ROLES: [&str; 2] = ["ask_user", "plan_review"];
const FINISHED_RUN_STATUSES: &str = "'success', 'failed', 'aborted', 'interrupted'";
const KIND_DONE: &str = "done";
const KIND_QUESTION: &str = "question";
const KIND_SCHEDULED: &str = "scheduled";
const KIND_ERROR: &str = "error";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentState {
    Running,
    Done,
    Attention,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentSession {
    pub id: String,
    pub title: String,
    pub agent: String,
    pub state: AgentState,
    pub updated_at: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct AiEvent {
    pub kind: &'static str,
    pub key: String,
    pub at: i64,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct AiSnapshot {
    pub sessions: Vec<AgentSession>,
    pub events: Vec<AiEvent>,
    pub checked_at: i64,
    pub database_path: String,
    pub error: Option<String>,
}

impl AiSnapshot {
    pub fn count(&self, state: AgentState) -> usize {
        self.sessions.iter().filter(|session| session.state == state).count()
    }

    pub fn state(&self) -> Option<AgentState> {
        if self.count(AgentState::Attention) > 0 {
            Some(AgentState::Attention)
        } else if self.count(AgentState::Running) > 0 {
            Some(AgentState::Running)
        } else if self.count(AgentState::Done) > 0 {
            Some(AgentState::Done)
        } else {
            None
        }
    }
}

pub struct AiStatus {
    pub snapshot: Signal<AiSnapshot>,
    pub watermark: Signal<i64>,
}

pub fn start(enabled: Signal<bool>) -> AiStatus {
    let snapshot = signal(AiSnapshot::default());
    let watermark = signal(load_watermark());
    effect(move || save_watermark(watermark.get()));
    let read = Rc::new(move || {
        if !enabled.get() {
            snapshot.set(AiSnapshot::default());
            return;
        }
        spawn(async move {
            let next = scan_once();
            let has_new = next.events.iter().any(|event| event.at > watermark.get_untracked());
            if has_new && cindy_in_front() {
                let current = watermark.get_untracked();
                watermark.set(current.max(next.checked_at));
            }
            snapshot.set(next);
        });
    });
    read();
    let poll = read.clone();
    set_interval(POLL, move || poll());
    AiStatus { snapshot, watermark }
}

pub fn unseen(snapshot: &AiSnapshot, watermark: i64, kinds: [bool; 4]) -> usize {
    snapshot
        .events
        .iter()
        .filter(|event| event.at > watermark && kind_enabled(event.kind, kinds))
        .count()
}

pub fn mark_seen(snapshot: Signal<AiSnapshot>, watermark: Signal<i64>) {
    let checked = snapshot.get_untracked().checked_at;
    if checked > watermark.get_untracked() {
        watermark.set(checked);
    }
}

pub fn focus_cindy() -> bool {
    let focused = focus_app("cindy");
    if focused {
        log::info!("ai: focused Cindy");
    }
    focused
}

fn kind_enabled(kind: &str, kinds: [bool; 4]) -> bool {
    match kind {
        KIND_DONE => kinds[0],
        KIND_QUESTION => kinds[1],
        KIND_SCHEDULED => kinds[2],
        KIND_ERROR => kinds[3],
        _ => false,
    }
}

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or_default()
}

fn scan_once() -> AiSnapshot {
    let checked_at = now_ms();
    let Some(path) = database_path() else {
        return AiSnapshot {
            checked_at,
            ..AiSnapshot::default()
        };
    };
    let database_path = path.to_string_lossy().to_string();
    match open_read_only(&path).and_then(|connection| collect(&connection, checked_at)) {
        Ok((sessions, events)) => AiSnapshot {
            sessions,
            events,
            checked_at,
            database_path,
            error: None,
        },
        Err(error) => AiSnapshot {
            checked_at,
            database_path,
            error: Some(error),
            ..AiSnapshot::default()
        },
    }
}

fn database_path() -> Option<PathBuf> {
    let dir = Path::new(&std::env::var_os("APPDATA")?).join("Cindy");
    let mut newest: Option<(std::time::SystemTime, PathBuf)> = None;
    for entry in std::fs::read_dir(dir).ok()?.flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if !name.starts_with("cindy-") || !name.ends_with(".db") {
            continue;
        }
        let modified = entry
            .metadata()
            .and_then(|metadata| metadata.modified())
            .unwrap_or(std::time::UNIX_EPOCH);
        if newest.as_ref().is_none_or(|(best, _)| modified > *best) {
            newest = Some((modified, entry.path()));
        }
    }
    newest.map(|(_, path)| path)
}

fn open_read_only(path: &Path) -> Result<Connection, String> {
    Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|error| format!("Failed to open the Cindy database: {error}"))
}

fn table_columns(connection: &Connection, table: &str) -> Result<HashSet<String>, String> {
    let mut statement = connection
        .prepare(&format!("PRAGMA table_info({table})"))
        .map_err(|error| format!("Failed to inspect the Cindy database: {error}"))?;
    let names = statement
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(|error| format!("Failed to inspect the Cindy database: {error}"))?;
    Ok(names.flatten().collect())
}

fn live_filter(columns: &HashSet<String>, alias: &str) -> String {
    let mut filter = String::new();
    if columns.contains("status") {
        filter.push_str(&format!(" AND {alias}.status = 'active'"));
    }
    if columns.contains("hidden_at") {
        filter.push_str(&format!(" AND {alias}.hidden_at IS NULL"));
    }
    if columns.contains("notifications_muted") {
        filter.push_str(&format!(" AND COALESCE({alias}.notifications_muted, 0) = 0"));
    }
    filter
}

fn not_scheduled_filter(columns: &HashSet<String>, alias: &str) -> String {
    if columns.contains("source") {
        format!(" AND {alias}.source <> 'scheduler'")
    } else {
        String::new()
    }
}

type Collected = (Vec<AgentSession>, Vec<AiEvent>);

fn collect(connection: &Connection, now: i64) -> Result<Collected, String> {
    let session_columns = table_columns(connection, "sessions")?;
    let message_columns = table_columns(connection, "messages")?;
    let since = now.saturating_sub(EVENT_WINDOW_MS);
    let sessions = list_sessions(connection, &session_columns, &message_columns, now)?;
    let mut events = Vec::new();
    if !session_columns.is_empty() {
        events.extend(finished_sessions(connection, &session_columns, since)?);
        events.extend(waiting_sessions(connection, &session_columns, &message_columns)?);
    }
    if !message_columns.is_empty() {
        events.extend(error_messages(
            connection,
            &session_columns,
            &message_columns,
            since,
        )?);
    }
    events.extend(scheduled_runs(connection, since)?);
    events.extend(background_runs(connection, &session_columns, since)?);
    events.sort_by_key(|event| std::cmp::Reverse(event.at));
    Ok((sessions, events))
}

fn pick<'a>(columns: &HashSet<String>, name: &'a str, fallback: &'a str) -> String {
    if columns.contains(name) {
        name.to_string()
    } else {
        fallback.to_string()
    }
}

struct RawSession {
    id: String,
    title: String,
    agent: String,
    running: bool,
    updated_at: i64,
    ended_at: i64,
}

fn list_sessions(
    connection: &Connection,
    session_columns: &HashSet<String>,
    message_columns: &HashSet<String>,
    now: i64,
) -> Result<Vec<AgentSession>, String> {
    if !session_columns.contains("id") {
        return Ok(Vec::new());
    }
    let updated = ["last_activity_at", "updated_at"]
        .into_iter()
        .find(|name| session_columns.contains(*name))
        .unwrap_or("updated_at");
    let running = if session_columns.contains("active_turn_started_at")
        && session_columns.contains("last_turn_ended_at")
    {
        "(COALESCE(active_turn_started_at, 0) > COALESCE(last_turn_ended_at, 0))"
    } else {
        "0"
    };
    let sql = format!(
        "SELECT id, {title}, {agent}, {updated}, {running}, {ended}
         FROM sessions
         WHERE {updated} >= ?1{}{}
         ORDER BY {running} DESC, {updated} DESC LIMIT 30",
        live_filter(session_columns, "sessions"),
        not_scheduled_filter(session_columns, "sessions"),
        title = pick(session_columns, "title", "''"),
        agent = pick(session_columns, "agent_kind", "'agent'"),
        ended = pick(session_columns, "last_turn_ended_at", "0"),
    );
    let since = now.saturating_sub(SESSION_WINDOW_MS);
    let mut statement = connection
        .prepare(&sql)
        .map_err(|error| format!("Failed to read Cindy sessions: {error}"))?;
    let rows = statement
        .query_map([since], |row| {
            Ok(RawSession {
                id: row.get(0)?,
                title: row.get(1)?,
                agent: row.get(2)?,
                updated_at: row.get(3)?,
                running: row.get::<_, i64>(4)? != 0,
                ended_at: row.get(5)?,
            })
        })
        .map_err(|error| format!("Failed to read Cindy sessions: {error}"))?;
    let mut sessions = Vec::new();
    for raw in rows.flatten() {
        let last_role = last_message_role(connection, message_columns, &raw.id);
        let waiting = raw.running
            && last_role
                .as_deref()
                .is_some_and(|role| WAITING_ROLES.contains(&role));
        let failed = last_role.as_deref() == Some("error");
        let state = if waiting || failed {
            AgentState::Attention
        } else if raw.running {
            AgentState::Running
        } else if now.saturating_sub(raw.ended_at) <= DONE_WINDOW_MS {
            AgentState::Done
        } else {
            continue;
        };
        let title = title_of(&raw);
        sessions.push(AgentSession {
            id: raw.id,
            title,
            agent: agent_label(&raw.agent),
            state,
            updated_at: raw.updated_at,
        });
    }
    sessions.sort_by_key(|session| {
        let rank = match session.state {
            AgentState::Attention => 0,
            AgentState::Running => 1,
            AgentState::Done => 2,
        };
        (rank, std::cmp::Reverse(session.updated_at))
    });
    Ok(sessions)
}

fn last_message_role(
    connection: &Connection,
    columns: &HashSet<String>,
    session: &str,
) -> Option<String> {
    if !columns.contains("session_id") || !columns.contains("role") {
        return None;
    }
    let rewind = if columns.contains("rewind_at") {
        " AND rewind_at IS NULL"
    } else {
        ""
    };
    let id = if columns.contains("id") { "id" } else { "rowid" };
    let sql = format!(
        "SELECT role FROM messages WHERE session_id = ?1{rewind}
         ORDER BY created_at DESC, {id} DESC LIMIT 1"
    );
    let mut statement = connection.prepare(&sql).ok()?;
    statement.query_row([session], |row| row.get(0)).optional().ok()?
}

fn title_of(session: &RawSession) -> String {
    let title = session.title.trim();
    if !title.is_empty() {
        return truncate(title, 40);
    }
    "Agent".to_string()
}

fn agent_label(kind: &str) -> String {
    match kind.trim().to_ascii_lowercase().as_str() {
        "cc" | "claude" | "claude-code" => "Claude Code".to_string(),
        "codex" => "Codex".to_string(),
        "" => "Agent".to_string(),
        other => other.to_string(),
    }
}

fn truncate(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        return text.to_string();
    }
    text.chars().take(max).collect::<String>() + "…"
}

fn finished_sessions(
    connection: &Connection,
    columns: &HashSet<String>,
    since: i64,
) -> Result<Vec<AiEvent>, String> {
    if !columns.contains("id") || !columns.contains("last_turn_ended_at") {
        return Ok(Vec::new());
    }
    let mut sql = String::from(
        "SELECT id, last_turn_ended_at FROM sessions WHERE last_turn_ended_at >= ?1",
    );
    sql.push_str(&live_filter(columns, "sessions"));
    if columns.contains("active_turn_started_at") {
        sql.push_str(
            " AND COALESCE(active_turn_started_at, 0) <= COALESCE(last_turn_ended_at, 0)",
        );
    }
    sql.push_str(&not_scheduled_filter(columns, "sessions"));
    let mut statement = connection
        .prepare(&sql)
        .map_err(|error| format!("Failed to read Cindy sessions: {error}"))?;
    let rows = statement
        .query_map([since], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })
        .map_err(|error| format!("Failed to read Cindy sessions: {error}"))?;
    Ok(rows
        .flatten()
        .map(|(id, at)| AiEvent {
            kind: KIND_DONE,
            key: format!("done:{id}:{at}"),
            at,
        })
        .collect())
}

fn waiting_sessions(
    connection: &Connection,
    session_columns: &HashSet<String>,
    message_columns: &HashSet<String>,
) -> Result<Vec<AiEvent>, String> {
    for required in ["id", "session_id", "role", "created_at"] {
        if !message_columns.contains(required) {
            return Ok(Vec::new());
        }
    }
    if !session_columns.contains("id")
        || !session_columns.contains("active_turn_started_at")
        || !session_columns.contains("last_turn_ended_at")
    {
        return Ok(Vec::new());
    }
    let sql = format!(
        "SELECT id FROM sessions
         WHERE COALESCE(active_turn_started_at, 0) > COALESCE(last_turn_ended_at, 0){}",
        live_filter(session_columns, "sessions"),
    );
    let sessions = {
        let mut statement = connection
            .prepare(&sql)
            .map_err(|error| format!("Failed to read Cindy sessions: {error}"))?;
        let rows = statement
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(|error| format!("Failed to read Cindy sessions: {error}"))?;
        rows.flatten().collect::<Vec<String>>()
    };
    if sessions.is_empty() {
        return Ok(Vec::new());
    }
    let rewind = if message_columns.contains("rewind_at") {
        "AND rewind_at IS NULL"
    } else {
        ""
    };
    let id_column = if message_columns.contains("id") {
        "id"
    } else {
        "''"
    };
    let mut statement = connection
        .prepare(&format!(
            "SELECT {id_column}, role, created_at FROM messages
             WHERE session_id = ?1 {rewind}
             ORDER BY created_at DESC, {id_column} DESC LIMIT 1"
        ))
        .map_err(|error| format!("Failed to read Cindy messages: {error}"))?;
    let mut events = Vec::new();
    for session in sessions {
        let row: Option<(String, String, i64)> = statement
            .query_row([&session], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
            .optional()
            .map_err(|error| format!("Failed to read Cindy messages: {error}"))?;
        if let Some((message, role, at)) = row {
            if WAITING_ROLES.contains(&role.as_str()) {
                events.push(AiEvent {
                    kind: KIND_QUESTION,
                    key: format!("question:{session}:{message}"),
                    at,
                });
            }
        }
    }
    Ok(events)
}

fn error_messages(
    connection: &Connection,
    session_columns: &HashSet<String>,
    message_columns: &HashSet<String>,
    since: i64,
) -> Result<Vec<AiEvent>, String> {
    if !message_columns.contains("id") || !message_columns.contains("created_at") {
        return Ok(Vec::new());
    }
    let rewind = if message_columns.contains("rewind_at") {
        "AND m.rewind_at IS NULL"
    } else {
        ""
    };
    let join = if session_columns.contains("id") && message_columns.contains("session_id") {
        " LEFT JOIN sessions s ON s.id = m.session_id"
    } else {
        ""
    };
    let live = if join.is_empty() {
        String::new()
    } else {
        format!(
            " AND (s.id IS NULL OR (1 = 1{}{}))",
            live_filter(session_columns, "s"),
            not_scheduled_filter(session_columns, "s")
        )
    };
    let sql = format!(
        "SELECT m.id, m.created_at FROM messages m{join}
         WHERE m.role = 'error' {rewind} AND m.created_at >= ?1{live}"
    );
    let mut statement = connection
        .prepare(&sql)
        .map_err(|error| format!("Failed to read Cindy messages: {error}"))?;
    let rows = statement
        .query_map([since], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })
        .map_err(|error| format!("Failed to read Cindy messages: {error}"))?;
    Ok(rows
        .flatten()
        .map(|(id, at)| AiEvent {
            kind: KIND_ERROR,
            key: format!("error:{id}"),
            at,
        })
        .collect())
}

fn scheduled_runs(connection: &Connection, since: i64) -> Result<Vec<AiEvent>, String> {
    let columns = table_columns(connection, "schedule_runs")?;
    for required in ["id", "status", "read_at"] {
        if !columns.contains(required) {
            return Ok(Vec::new());
        }
    }
    let time = if columns.contains("finished_at") && columns.contains("fired_at") {
        "COALESCE(r.finished_at, r.fired_at)"
    } else if columns.contains("finished_at") {
        "r.finished_at"
    } else if columns.contains("fired_at") {
        "r.fired_at"
    } else {
        return Ok(Vec::new());
    };
    let schedule_columns = table_columns(connection, "schedules")?;
    let joined = columns.contains("schedule_id")
        && schedule_columns.contains("id")
        && schedule_columns.contains("notify_desktop");
    let join = if joined {
        " JOIN schedules sc ON sc.id = r.schedule_id"
    } else {
        ""
    };
    let notify = if joined {
        " AND COALESCE(sc.notify_desktop, 1) <> 0"
    } else {
        ""
    };
    let sql = format!(
        "SELECT r.id, r.status, {time} FROM schedule_runs r{join}
         WHERE r.read_at IS NULL AND r.status IN ({FINISHED_RUN_STATUSES})
           AND {time} >= ?1{notify}"
    );
    let mut statement = connection
        .prepare(&sql)
        .map_err(|error| format!("Failed to read Cindy schedules: {error}"))?;
    let rows = statement
        .query_map([since], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i64>(2)?,
            ))
        })
        .map_err(|error| format!("Failed to read Cindy schedules: {error}"))?;
    Ok(rows
        .flatten()
        .map(|(id, status, at)| AiEvent {
            kind: if status == "success" {
                KIND_SCHEDULED
            } else {
                KIND_ERROR
            },
            key: format!("schedule:{id}"),
            at,
        })
        .collect())
}

fn background_runs(
    connection: &Connection,
    session_columns: &HashSet<String>,
    since: i64,
) -> Result<Vec<AiEvent>, String> {
    let columns = table_columns(connection, "subagent_runs")?;
    for required in ["id", "status", "ended_at"] {
        if !columns.contains(required) {
            return Ok(Vec::new());
        }
    }
    let joined = session_columns.contains("id") && columns.contains("session_id");
    let mut sql = String::from("SELECT r.id, r.status, r.ended_at FROM subagent_runs r");
    if joined {
        sql.push_str(" JOIN sessions s ON s.id = r.session_id");
    }
    sql.push_str(" WHERE r.status IN ('completed', 'failed', 'stopped') AND r.ended_at >= ?1");
    if columns.contains("deleted_at") {
        sql.push_str(" AND r.deleted_at IS NULL");
    }
    if columns.contains("rewind_at") {
        sql.push_str(" AND r.rewind_at IS NULL");
    }
    if joined {
        sql.push_str(&live_filter(session_columns, "s"));
    }
    let mut statement = connection
        .prepare(&sql)
        .map_err(|error| format!("Failed to read Cindy background tasks: {error}"))?;
    let rows = statement
        .query_map([since], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i64>(2)?,
            ))
        })
        .map_err(|error| format!("Failed to read Cindy background tasks: {error}"))?;
    Ok(rows
        .flatten()
        .map(|(id, status, at)| AiEvent {
            kind: if status == "completed" {
                KIND_SCHEDULED
            } else {
                KIND_ERROR
            },
            key: format!("background:{id}:{status}"),
            at,
        })
        .collect())
}

#[derive(Serialize, Deserialize)]
struct Watermark {
    watermark: i64,
}

fn data_file() -> Option<PathBuf> {
    let base = std::env::var_os("APPDATA")?;
    Some(PathBuf::from(base).join("com.focusd.island.nsh").join("ai-watermark.json"))
}

fn load_watermark() -> i64 {
    let saved = data_file()
        .and_then(|path| std::fs::read_to_string(path).ok())
        .and_then(|text| serde_json::from_str::<Watermark>(&text).ok())
        .map(|watermark| watermark.watermark)
        .filter(|watermark| *watermark > 0);
    saved.unwrap_or_else(now_ms)
}

fn save_watermark(watermark: i64) {
    let Some(path) = data_file() else {
        return;
    };
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(text) = serde_json::to_string_pretty(&Watermark { watermark }) {
        let _ = std::fs::write(path, text);
    }
}

#[cfg(windows)]
fn focus_app(_app: &str) -> bool {
    use windows::{
        core::BOOL,
        Win32::{
            Foundation::{HWND, LPARAM},
            System::Threading::{AttachThreadInput, GetCurrentThreadId},
            UI::WindowsAndMessaging::{
                AllowSetForegroundWindow, EnumWindows, GetForegroundWindow, GetWindow,
                GetWindowThreadProcessId, IsGUIThread, IsIconic, IsWindowVisible,
                SetForegroundWindow, ShowWindow, GW_OWNER, SW_RESTORE,
            },
        },
    };

    unsafe extern "system" fn visit(window: HWND, context: LPARAM) -> BOOL {
        // SAFETY: the context points at the Option<HWND> owned by the call
        // below; EnumWindows calls this synchronously before it returns.
        let found = unsafe { &mut *(context.0 as *mut Option<HWND>) };
        if found.is_some() {
            return false.into();
        }
        // SAFETY: every call takes the window by value; invalid windows fail.
        unsafe {
            if !IsWindowVisible(window).as_bool() {
                return true.into();
            }
            if GetWindow(window, GW_OWNER).is_ok_and(|owner| !owner.is_invalid()) {
                return true.into();
            }
        }
        let process_id = process_id_of(window);
        if image_stem(process_id).is_some_and(|name| name.eq_ignore_ascii_case("cindy")) {
            *found = Some(window);
            return false.into();
        }
        true.into()
    }

    let mut found: Option<HWND> = None;
    // SAFETY: visit matches WNDENUMPROC and reads the live context pointer.
    unsafe {
        let _ = EnumWindows(
            Some(visit),
            LPARAM(&mut found as *mut Option<HWND> as isize),
        );
    }
    let Some(window) = found else {
        return false;
    };
    let process_id = process_id_of(window);
    // SAFETY: plain values; a failed call is harmless.
    unsafe {
        let _ = AllowSetForegroundWindow(process_id);
        if IsIconic(window).as_bool() {
            let _ = ShowWindow(window, SW_RESTORE);
        }
        if GetForegroundWindow() == window {
            return true;
        }
        if SetForegroundWindow(window).as_bool() && GetForegroundWindow() == window {
            return true;
        }
        let foreground = GetForegroundWindow();
        let (foreground_thread, own_thread) =
            (GetWindowThreadProcessId(foreground, None), GetCurrentThreadId());
        let attach = foreground_thread != 0 && foreground_thread != own_thread;
        let _ = IsGUIThread(true);
        let attached = attach && AttachThreadInput(own_thread, foreground_thread, true).as_bool();
        let _ = SetForegroundWindow(window);
        if attached {
            let _ = AttachThreadInput(own_thread, foreground_thread, false);
        }
        GetForegroundWindow() == window
    }
}

#[cfg(windows)]
fn cindy_in_front() -> bool {
    use windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow;
    // SAFETY: GetForegroundWindow takes no arguments.
    let window = unsafe { GetForegroundWindow() };
    if window.is_invalid() {
        return false;
    }
    image_stem(process_id_of(window)).is_some_and(|name| name.eq_ignore_ascii_case("cindy"))
}

#[cfg(windows)]
fn process_id_of(window: windows::Win32::Foundation::HWND) -> u32 {
    use windows::Win32::UI::WindowsAndMessaging::GetWindowThreadProcessId;
    let mut process_id = 0;
    // SAFETY: the call writes into the valid local; invalid windows yield 0.
    unsafe { GetWindowThreadProcessId(window, Some(&mut process_id)) };
    process_id
}

#[cfg(windows)]
fn image_stem(process_id: u32) -> Option<String> {
    use windows::{
        core::PWSTR,
        Win32::{
            Foundation::CloseHandle,
            System::Threading::{
                OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32,
                PROCESS_QUERY_LIMITED_INFORMATION,
            },
        },
    };
    // SAFETY: a handle returned by OpenProcess is owned here and closed below.
    let handle = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, process_id) }.ok()?;
    let mut path = [0u16; 32768];
    let mut length = path.len() as u32;
    // SAFETY: the handle is live and the buffer is writable for length units.
    let result = unsafe {
        QueryFullProcessImageNameW(
            handle,
            PROCESS_NAME_WIN32,
            PWSTR(path.as_mut_ptr()),
            &mut length,
        )
    };
    // SAFETY: the handle came from OpenProcess and is closed exactly once.
    let _ = unsafe { CloseHandle(handle) };
    result.ok()?;
    Path::new(&String::from_utf16_lossy(&path[..length as usize]))
        .file_stem()
        .map(|name| name.to_string_lossy().to_string())
}

#[cfg(not(windows))]
fn focus_app(_app: &str) -> bool {
    false
}

#[cfg(not(windows))]
fn cindy_in_front() -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn memory_database() -> Connection {
        let connection = Connection::open_in_memory().expect("in-memory database");
        connection
            .execute_batch(
                "CREATE TABLE sessions (
                    id TEXT PRIMARY KEY, title TEXT, agent_kind TEXT, status TEXT,
                    hidden_at INTEGER, notifications_muted INTEGER, source TEXT,
                    active_turn_started_at INTEGER, last_turn_ended_at INTEGER,
                    last_activity_at INTEGER
                );
                CREATE TABLE messages (
                    id TEXT PRIMARY KEY, session_id TEXT, role TEXT,
                    created_at INTEGER, rewind_at INTEGER
                );",
            )
            .expect("schema");
        connection
    }

    fn add_session(connection: &Connection, id: &str, running: bool, ended: i64, updated: i64) {
        connection
            .execute(
                "INSERT INTO sessions (id, title, agent_kind, status, source,
                    active_turn_started_at, last_turn_ended_at, last_activity_at)
                 VALUES (?1, ?2, 'codex', 'active', 'desktop', ?3, ?4, ?5)",
                rusqlite::params![id, format!("Session {id}"), if running { updated } else { ended - 1 }, ended, updated],
            )
            .expect("session");
    }

    fn add_message(connection: &Connection, id: &str, session: &str, role: &str, at: i64) {
        connection
            .execute(
                "INSERT INTO messages (id, session_id, role, created_at) VALUES (?1, ?2, ?3, ?4)",
                rusqlite::params![id, session, role, at],
            )
            .expect("message");
    }

    #[test]
    fn three_states_follow_the_turn_and_the_last_message() {
        let connection = memory_database();
        let now = 10_000_000;
        add_session(&connection, "run", true, now - 2_000, now - 1_000);
        add_message(&connection, "m1", "run", "tool_use", now - 500);
        add_session(&connection, "ask", true, now - 2_000, now - 900);
        add_message(&connection, "m2", "ask", "ask_user", now - 400);
        add_session(&connection, "done", false, now - 1_000, now - 1_000);
        add_session(&connection, "old", false, now - 3_600_000, now - 3_600_000);
        add_session(&connection, "failed", false, now - 1_000, now - 800);
        add_message(&connection, "m3", "failed", "error", now - 700);

        let (sessions, events) = collect(&connection, now).expect("collect");
        let states: Vec<(&str, AgentState)> = sessions
            .iter()
            .map(|session| (session.id.as_str(), session.state))
            .collect();

        assert_eq!(states.len(), 4);
        assert!(states.contains(&("ask", AgentState::Attention)));
        assert!(states.contains(&("failed", AgentState::Attention)));
        assert!(states.contains(&("run", AgentState::Running)));
        assert!(states.contains(&("done", AgentState::Done)));
        assert!(!states.iter().any(|(id, _)| *id == "old"));
        assert_eq!(sessions[0].state, AgentState::Attention);
        assert!(events.iter().any(|event| event.kind == KIND_QUESTION));
        assert!(events.iter().any(|event| event.kind == KIND_DONE));
        assert!(events.iter().any(|event| event.kind == KIND_ERROR));
    }

    #[test]
    fn an_answered_question_is_not_attention() {
        let connection = memory_database();
        let now = 10_000_000;
        add_session(&connection, "run", true, now - 2_000, now - 1_000);
        add_message(&connection, "m1", "run", "ask_user", now - 500);
        add_message(&connection, "m2", "run", "user", now - 400);

        let (sessions, _) = collect(&connection, now).expect("collect");
        assert_eq!(sessions[0].state, AgentState::Running);
    }
}

