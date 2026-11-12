//! Cindy 的会话数据源：只读 Cindy 自己的本地数据库。
//!
//! Cindy（Electron 应用）把会话状态存在 `%APPDATA%\Cindy\cindy-<id>.db` 的 SQLite 里，
//! 它的「灵动岛」显示的就是这批会话：有几个在跑、哪个等你回答、最近跑完了什么。
//! 这里只读连上去取 `sessions` 表（再补一条 `messages` 的最后角色），
//! 表结构是 Cindy 的私有实现，所以按「列存在才查」写：缺列降级，不让 Cindy 升级把岛弄崩。

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use rusqlite::{Connection, OpenFlags};

/// Cindy 里"等你回答"的两种消息角色（见 `messages.role`）。
const WAITING_ROLES: [&str; 2] = ["ask_user", "plan_review"];

/// 一个 Cindy 会话，字段是 `sessions` 表里岛需要的那部分。
#[derive(Debug, Clone, PartialEq)]
pub(super) struct CindySession {
    pub id: String,
    pub title: Option<String>,
    pub cwd: Option<String>,
    pub model: Option<String>,
    pub permission_mode: Option<String>,
    /// 当前这一轮还在跑：`active_turn_started_at` 晚于上一轮结束时间。
    pub running: bool,
    /// 等你回答（最后一条消息是提问或计划确认）。
    pub waiting: bool,
    /// 最后一次活动，Unix 毫秒。
    pub updated_at: i64,
    /// 当前这一轮开始的时间，Unix 毫秒。
    pub turn_started_at: Option<i64>,
    pub last_message: Option<String>,
    pub context_tokens: Option<u64>,
    pub context_window: Option<u64>,
    pub total_tokens: u64,
    pub cost_usd: f64,
}

/// 刚跑完的会话在这个时间内还显示成「完成」。
const COMPLETED_WINDOW_MS: i64 = 10 * 60 * 1000;

/// 岛上的状态快照：有会话在跑（或等你回答）就是 running；否则看最近有没有刚跑完的。
///
/// 这与 Cindy 自己灵动岛的 `priorityStatus` 语义一致：等你回答优先于在跑，
/// 在跑优先于刚完成。
pub(super) fn status_snapshot(
    sessions: &[CindySession],
    status_path: String,
) -> super::snapshot::AgentStatusSnapshot {
    use super::snapshot::{current_unix_millis, AgentStatusSnapshot, AgentTaskStatus};

    let now = current_unix_millis();
    let busy = sessions
        .iter()
        .find(|session| session.waiting)
        .or_else(|| sessions.iter().find(|session| session.running));
    let cindy = match busy {
        Some(session) => AgentTaskStatus {
            phase: "running".to_string(),
            task_id: Some(session.id.clone()),
            updated_at: now,
        },
        None => sessions
            .first()
            .filter(|session| now.saturating_sub(session.updated_at) < COMPLETED_WINDOW_MS)
            .map(|session| AgentTaskStatus {
                phase: "completed".to_string(),
                task_id: Some(session.id.clone()),
                updated_at: session.updated_at,
            })
            .unwrap_or_else(AgentTaskStatus::idle),
    };
    AgentStatusSnapshot {
        cindy,
        updated_at: now,
        status_path,
    }
}

/// Cindy 数据库的位置：`%APPDATA%\Cindy\` 下最新的 `cindy-*.db`。
///
/// 同级还有 `cindy-*.db.bak.<时间>` 备份和 `-wal` / `-shm`，靠后缀把它们排除掉。
pub(super) fn database_path() -> Option<PathBuf> {
    let app_data = std::env::var_os("APPDATA")?;
    let dir = Path::new(&app_data).join("Cindy");
    let mut newest: Option<(std::time::SystemTime, PathBuf)> = None;
    for entry in std::fs::read_dir(&dir).ok()?.flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if !name.starts_with("cindy-") || !name.ends_with(".db") {
            continue;
        }
        let modified = entry
            .metadata()
            .and_then(|metadata| metadata.modified())
            .unwrap_or(std::time::UNIX_EPOCH);
        if newest.as_ref().map(|(best, _)| modified > *best).unwrap_or(true) {
            newest = Some((modified, entry.path()));
        }
    }
    newest.map(|(_, path)| path)
}

/// 读最近 `window_ms` 内活动过的会话，运行中和等你回答的排在前面。
pub(super) fn list_sessions(window_ms: i64) -> Result<Vec<CindySession>, String> {
    let Some(path) = database_path() else {
        // Cindy 没装或还没建库：岛上不显示 AI 区，这不是错误。
        return Ok(Vec::new());
    };
    let connection = open_read_only(&path)?;
    let columns = table_columns(&connection, "sessions")?;
    if !columns.contains("id") {
        return Ok(Vec::new());
    }

    // 每一列先确认存在，缺的用常量顶上，这样 Cindy 改表也不会让整条查询失败。
    let pick = |name: &str, fallback: &str| if columns.contains(name) { name.to_string() } else { fallback.to_string() };
    let updated = ["last_activity_at", "updated_at"]
        .into_iter()
        .find(|name| columns.contains(*name))
        .unwrap_or("updated_at");
    let running = if columns.contains("active_turn_started_at") && columns.contains("last_turn_ended_at") {
        "(active_turn_started_at > COALESCE(last_turn_ended_at, 0))"
    } else {
        "0"
    };
    let sql = format!(
        "SELECT id, {title}, {cwd}, {model}, {permission_mode}, {running},
                {updated}, {preview}, {context_tokens}, {context_window}, {total_tokens}, {cost},
                {turn_started}
         FROM sessions
         WHERE {status} = 'active' AND {hidden} IS NULL AND {updated} >= ?1
         ORDER BY {running} DESC, {updated} DESC",
        title = pick("title", "NULL"),
        cwd = pick("working_dir", "NULL"),
        model = pick("model", "NULL"),
        permission_mode = pick("permission_mode", "NULL"),
        // `list_preview` is what Cindy shows in its own list; `summary` is the older fallback.
        preview = if columns.contains("list_preview") {
            "COALESCE(list_preview, summary)"
        } else if columns.contains("summary") {
            "summary"
        } else {
            "NULL"
        },
        context_tokens = pick("context_tokens", "NULL"),
        context_window = pick("context_window", "NULL"),
        total_tokens = pick("total_token_usage", "0"),
        cost = pick("total_cost_usd", "0"),
        status = pick("status", "'active'"),
        hidden = pick("hidden_at", "NULL"),
        turn_started = pick("active_turn_started_at", "NULL"),
    );

    let since = super::snapshot::current_unix_millis().saturating_sub(window_ms);
    let mut statement = connection
        .prepare(&sql)
        .map_err(|error| format!("Failed to read Cindy sessions: {error}"))?;
    let rows = statement
        .query_map([since], |row| {
            Ok(CindySession {
                id: row.get(0)?,
                title: row.get(1)?,
                cwd: row.get(2)?,
                model: row.get(3)?,
                permission_mode: row.get(4)?,
                running: row.get::<_, Option<i64>>(5)?.unwrap_or_default() != 0,
                waiting: false,
                updated_at: row.get::<_, Option<i64>>(6)?.unwrap_or_default(),
                last_message: row.get(7)?,
                context_tokens: row.get::<_, Option<i64>>(8)?.map(|value| value.max(0) as u64),
                context_window: row.get::<_, Option<i64>>(9)?.map(|value| value.max(0) as u64),
                total_tokens: row.get::<_, Option<i64>>(10)?.unwrap_or_default().max(0) as u64,
                cost_usd: row.get::<_, Option<f64>>(11)?.unwrap_or_default(),
                turn_started_at: row.get::<_, Option<i64>>(12)?,
            })
        })
        .map_err(|error| format!("Failed to read Cindy sessions: {error}"))?;

    let mut sessions: Vec<CindySession> = rows.flatten().collect();
    mark_waiting(&connection, &mut sessions);
    sessions.sort_by_key(|session| {
        let active = session.running || session.waiting;
        (!active, std::cmp::Reverse(session.updated_at))
    });
    Ok(sessions)
}

/// 打开只读连接：Cindy 正在写这个库，任何写操作都会抢锁。
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

/// 把「最后一条消息是提问」的会话标成等你回答。
///
/// Cindy 没有「待处理交互」字段，但回答一定会追加一条 user 消息，所以最后一条是
/// `ask_user` / `plan_review` 就意味着还没答。只对正在跑的会话查，走 `(session_id, created_at)`
/// 索引，不扫大表。
fn mark_waiting(connection: &Connection, sessions: &mut [CindySession]) {
    if !sessions.iter().any(|session| session.running) {
        return;
    }
    let columns = match table_columns(connection, "messages") {
        Ok(columns) => columns,
        Err(_) => return,
    };
    if !columns.contains("session_id") || !columns.contains("role") {
        return;
    }
    let rewind_filter = if columns.contains("rewind_at") {
        "AND rewind_at IS NULL"
    } else {
        ""
    };
    let sql = format!(
        "SELECT role FROM messages WHERE session_id = ?1 {rewind_filter}
         ORDER BY created_at DESC LIMIT 1"
    );
    let Ok(mut statement) = connection.prepare(&sql) else {
        return;
    };
    for session in sessions.iter_mut().filter(|session| session.running) {
        let role: Option<String> = statement.query_row([&session.id], |row| row.get(0)).ok();
        session.waiting = role
            .as_deref()
            .is_some_and(|role| WAITING_ROLES.contains(&role));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn memory_database() -> Connection {
        let connection = Connection::open_in_memory().expect("in-memory database");
        connection
            .execute_batch(
                "CREATE TABLE sessions (
                    id TEXT PRIMARY KEY, title TEXT, working_dir TEXT, model TEXT,
                    agent_kind TEXT, permission_mode TEXT, status TEXT,
                    active_turn_started_at INTEGER, last_turn_ended_at INTEGER,
                    last_activity_at INTEGER, list_preview TEXT, context_tokens INTEGER,
                    context_window INTEGER, total_token_usage INTEGER, total_cost_usd REAL,
                    hidden_at INTEGER
                );
                CREATE TABLE messages (
                    id TEXT PRIMARY KEY, session_id TEXT, role TEXT,
                    created_at INTEGER, rewind_at INTEGER
                );",
            )
            .expect("schema");
        connection
    }

    fn session(id: &str, running: bool) -> CindySession {
        CindySession {
            id: id.to_string(),
            title: Some("Task".to_string()),
            cwd: None,
            model: None,
            permission_mode: None,
            running,
            waiting: false,
            updated_at: 1000,
            turn_started_at: Some(900),
            last_message: None,
            context_tokens: None,
            context_window: None,
            total_tokens: 0,
            cost_usd: 0.0,
        }
    }

    #[test]
    fn columns_come_back_for_the_tables_we_query() {
        let connection = memory_database();
        let columns = table_columns(&connection, "sessions").expect("columns");
        assert!(columns.contains("list_preview"));
        assert!(columns.contains("active_turn_started_at"));
        assert!(!columns.contains("nope"));
    }

    #[test]
    fn a_running_turn_whose_last_message_is_a_question_waits_on_the_user() {
        let connection = memory_database();
        connection
            .execute_batch(
                "INSERT INTO messages (id, session_id, role, created_at) VALUES ('m1', 's1', 'assistant', 1500);
                 INSERT INTO messages (id, session_id, role, created_at) VALUES ('m2', 's1', 'ask_user', 2000);",
            )
            .expect("rows");
        let mut sessions = vec![session("s1", true)];
        mark_waiting(&connection, &mut sessions);
        assert!(sessions[0].waiting);
    }

    #[test]
    fn an_answered_question_does_not_wait() {
        let connection = memory_database();
        connection
            .execute_batch(
                "INSERT INTO messages (id, session_id, role, created_at) VALUES ('m1', 's1', 'ask_user', 1000);
                 INSERT INTO messages (id, session_id, role, created_at) VALUES ('m2', 's1', 'user', 2000);",
            )
            .expect("rows");
        let mut sessions = vec![session("s1", true)];
        mark_waiting(&connection, &mut sessions);
        assert!(!sessions[0].waiting);
    }

    #[test]
    fn a_session_that_is_not_running_is_left_alone() {
        let connection = memory_database();
        connection
            .execute_batch(
                "INSERT INTO messages (id, session_id, role, created_at) VALUES ('m1', 's1', 'ask_user', 1);",
            )
            .expect("rows");
        let mut sessions = vec![session("s1", false)];
        mark_waiting(&connection, &mut sessions);
        assert!(!sessions[0].waiting, "跑完的会话不该一直挂着等待");
    }
}
