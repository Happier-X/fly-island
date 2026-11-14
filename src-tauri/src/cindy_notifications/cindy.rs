//! Cindy 的本地数据库：只读判断四类通知。
//!
//! Cindy（Electron 应用）把会话、消息、定时任务和后台子任务都存在 `%APPDATA%\Cindy\cindy-<id>.db`
//! 的 SQLite 里。这里只读连上去，按「列存在才查」拼查询：表结构是 Cindy 的私有实现，缺列时那一类
//! 事件降级为空，不让 Cindy 升级把岛弄崩。
//!
//! 判断口径：
//! - `done`：一轮已经结束（`last_turn_ended_at`，且没有未结束的轮次）的活跃会话；
//! - `question`：最后一条消息是 `ask_user` / `plan_review`，还在等你回答的会话；
//! - `scheduled`：未读且成功的定时任务结果，或刚结束的后台子任务（`subagent_runs`）；
//! - `error`：新的 `role='error'` 消息，或失败的定时 / 后台任务。
//!
//! 已读与否不在这里判断：Cindy 只对 `schedule_runs` 记 `read_at`，其余类型没有已读字段，
//! 由岛上前端按本地水位过滤（见 `features/cindy-notifications/storage.ts`）。

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use rusqlite::{Connection, OpenFlags, OptionalExtension};
use serde::Serialize;

use super::now_unix_millis;

/// 更早的完成事件不再提醒：角标说的是「你离开时发生了什么」，不是历史记录。
/// 还在等你回答的会话不受这个窗口限制。
const WINDOW_MS: i64 = 24 * 60 * 60 * 1000;

const KIND_DONE: &str = "done";
const KIND_QUESTION: &str = "question";
const KIND_SCHEDULED: &str = "scheduled";
const KIND_ERROR: &str = "error";

/// Cindy 里「等你回答」的消息角色（见 `messages.role`）。
const WAITING_ROLES: [&str; 2] = ["ask_user", "plan_review"];

/// 定时任务结束的状态。`running` 及以后可能新增的状态不算结束。
const FINISHED_RUN_STATUSES: &str = "'success', 'failed', 'aborted', 'interrupted'";

/// 一条待处理通知，不含正文：岛上只显示数量，点开就是 Cindy 本身。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CindyNotification {
    /// `done` / `question` / `scheduled` / `error`。
    pub kind: &'static str,
    /// 稳定指纹，同一事件每次读取都相同；前端只按时间水位去重，这份指纹留给排查与测试。
    pub key: String,
    /// 事件时间，Unix 毫秒。
    pub at: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CindyNotificationSnapshot {
    /// 尚未处理的事件，新的在前。是否「看过」由前端本地水位决定。
    pub items: Vec<CindyNotification>,
    /// Cindy 数据库路径；空字符串表示 Cindy 没装或还没建库。
    pub database_path: String,
    /// 这次读取的时间，Unix 毫秒。
    pub checked_at: i64,
    /// 数据库读不出来时的原因，给设置页排障；能读时为 None。
    pub error: Option<String>,
}

/// 读一次 Cindy 的数据库。读不出来不报错，而是把原因放进快照让设置页显示。
pub(super) fn scan() -> CindyNotificationSnapshot {
    let checked_at = now_unix_millis();
    let path = database_path();
    let database_path = path
        .as_ref()
        .map(|path| path.to_string_lossy().to_string())
        .unwrap_or_default();
    let read = match &path {
        Some(path) => open_read_only(path).and_then(|connection| collect(&connection, checked_at)),
        None => Ok(Vec::new()),
    };
    match read {
        Ok(items) => CindyNotificationSnapshot {
            items,
            database_path,
            checked_at,
            error: None,
        },
        Err(error) => CindyNotificationSnapshot {
            items: Vec::new(),
            database_path,
            checked_at,
            error: Some(error),
        },
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

/// 打开只读连接：Cindy 正在写这个库，任何写操作都会抢锁。
fn open_read_only(path: &Path) -> Result<Connection, String> {
    Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|error| format!("Failed to open the Cindy database: {error}"))
}

fn collect(connection: &Connection, now: i64) -> Result<Vec<CindyNotification>, String> {
    let since = now.saturating_sub(WINDOW_MS);
    let session_columns = table_columns(connection, "sessions")?;
    let message_columns = table_columns(connection, "messages")?;
    let mut items = Vec::new();
    if !session_columns.is_empty() {
        items.extend(finished_sessions(connection, &session_columns, since)?);
        items.extend(waiting_sessions(connection, &session_columns, &message_columns)?);
    }
    if !message_columns.is_empty() {
        items.extend(error_messages(connection, &session_columns, &message_columns, since)?);
    }
    items.extend(scheduled_runs(connection, since)?);
    items.extend(background_runs(connection, &session_columns, since)?);
    items.sort_by(|left, right| right.at.cmp(&left.at).then_with(|| left.key.cmp(&right.key)));
    items.dedup_by(|left, right| left.key == right.key);
    Ok(items)
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

/// 值得提醒的会话：Cindy 里仍是活跃、未隐藏、未静音的。
fn live_session_filters(columns: &HashSet<String>, alias: &str) -> String {
    let mut filters = String::new();
    if columns.contains("status") {
        filters.push_str(&format!(" AND {alias}.status = 'active'"));
    }
    if columns.contains("hidden_at") {
        filters.push_str(&format!(" AND {alias}.hidden_at IS NULL"));
    }
    if columns.contains("notifications_muted") {
        filters.push_str(&format!(" AND COALESCE({alias}.notifications_muted, 0) = 0"));
    }
    filters
}

/// 定时任务跑出来的会话由 `schedule_runs` 统一算，避免一次运行报两遍。
fn not_scheduled_filter(columns: &HashSet<String>, alias: &str) -> String {
    if columns.contains("source") {
        format!(" AND {alias}.source <> 'scheduler'")
    } else {
        String::new()
    }
}

/// 一轮已经结束的会话。跑完的轮次在 `last_turn_ended_at`，未结束的轮次让开始时间更大。
fn finished_sessions(
    connection: &Connection,
    columns: &HashSet<String>,
    since: i64,
) -> Result<Vec<CindyNotification>, String> {
    if !columns.contains("id") || !columns.contains("last_turn_ended_at") {
        return Ok(Vec::new());
    }
    let mut sql = String::from("SELECT s.id, s.last_turn_ended_at FROM sessions s WHERE s.last_turn_ended_at >= ?1");
    sql.push_str(&live_session_filters(columns, "s"));
    if columns.contains("active_turn_started_at") {
        sql.push_str(" AND COALESCE(s.active_turn_started_at, 0) <= COALESCE(s.last_turn_ended_at, 0)");
    }
    sql.push_str(&not_scheduled_filter(columns, "s"));
    let mut statement = connection
        .prepare(&sql)
        .map_err(|error| format!("Failed to read Cindy sessions: {error}"))?;
    let rows = statement
        .query_map([since], |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?)))
        .map_err(|error| format!("Failed to read Cindy sessions: {error}"))?;
    Ok(rows
        .flatten()
        .map(|(id, at)| CindyNotification {
            kind: KIND_DONE,
            key: format!("done:{id}:{at}"),
            at,
        })
        .collect())
}

/// 最后一条消息还是提问的会话。回答一定会追加一条 `user` 消息，所以判定不会一直挂着。
fn waiting_sessions(
    connection: &Connection,
    session_columns: &HashSet<String>,
    message_columns: &HashSet<String>,
) -> Result<Vec<CindyNotification>, String> {
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
        "SELECT s.id FROM sessions s
         WHERE COALESCE(s.active_turn_started_at, 0) > COALESCE(s.last_turn_ended_at, 0){}",
        live_session_filters(session_columns, "s"),
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
    // 走 `(session_id, created_at)` 索引，只对还在跑的会话查。
    let rewind_filter = if message_columns.contains("rewind_at") {
        "AND rewind_at IS NULL"
    } else {
        ""
    };
    let id_column = if message_columns.contains("id") { "id" } else { "''" };
    let mut statement = connection
        .prepare(&format!(
            "SELECT {id_column}, role, created_at FROM messages
             WHERE session_id = ?1 {rewind_filter}
             ORDER BY created_at DESC, {id_column} DESC LIMIT 1"
        ))
        .map_err(|error| format!("Failed to read Cindy messages: {error}"))?;
    let mut items = Vec::new();
    for session in sessions {
        let row: Option<(String, String, i64)> = statement
            .query_row([&session], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
            .optional()
            .map_err(|error| format!("Failed to read Cindy messages: {error}"))?;
        if let Some((message, role, at)) = row {
            if WAITING_ROLES.contains(&role.as_str()) {
                items.push(CindyNotification {
                    kind: KIND_QUESTION,
                    key: format!("question:{session}:{message}"),
                    at,
                });
            }
        }
    }
    Ok(items)
}

/// 新的出错消息。过滤掉已回滚、已删除会话和静音会话里的错误。
fn error_messages(
    connection: &Connection,
    session_columns: &HashSet<String>,
    message_columns: &HashSet<String>,
    since: i64,
) -> Result<Vec<CindyNotification>, String> {
    if !message_columns.contains("id") || !message_columns.contains("created_at") {
        return Ok(Vec::new());
    }
    let rewind_filter = if message_columns.contains("rewind_at") {
        "AND m.rewind_at IS NULL"
    } else {
        ""
    };
    let join = if session_columns.contains("id") && message_columns.contains("session_id") {
        " LEFT JOIN sessions s ON s.id = m.session_id"
    } else {
        ""
    };
    // 没有会话行的错误消息（会话已被删掉）照常提醒；有会话行的按会话状态过滤。
    let live = if join.is_empty() {
        String::new()
    } else {
        format!(
            " AND (s.id IS NULL OR (1 = 1{}{}))",
            live_session_filters(session_columns, "s"),
            not_scheduled_filter(session_columns, "s")
        )
    };
    let sql = format!(
        "SELECT m.id, m.created_at FROM messages m{join}
         WHERE m.role = 'error' {rewind_filter} AND m.created_at >= ?1{live}"
    );
    let mut statement = connection
        .prepare(&sql)
        .map_err(|error| format!("Failed to read Cindy messages: {error}"))?;
    let rows = statement
        .query_map([since], |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?)))
        .map_err(|error| format!("Failed to read Cindy messages: {error}"))?;
    Ok(rows
        .flatten()
        .map(|(id, at)| CindyNotification {
            kind: KIND_ERROR,
            key: format!("error:{id}"),
            at,
        })
        .collect())
}

/// 定时任务的结果。`read_at` 是 Cindy 自己记的已读，只有它为空的运行才算未处理。
fn scheduled_runs(connection: &Connection, since: i64) -> Result<Vec<CindyNotification>, String> {
    let columns = table_columns(connection, "schedule_runs")?;
    for required in ["id", "status", "read_at"] {
        if !columns.contains(required) {
            return Ok(Vec::new());
        }
    }
    let time = finished_at_expression(&columns);
    let Some(time) = time else {
        return Ok(Vec::new());
    };
    let schedule_columns = table_columns(connection, "schedules")?;
    let joined = columns.contains("schedule_id")
        && schedule_columns.contains("id")
        && schedule_columns.contains("notify_desktop");
    let join = if joined { " JOIN schedules sc ON sc.id = r.schedule_id" } else { "" };
    // 用户在 Cindy 里关掉某个任务的桌面通知时，岛上也不提醒它。
    let notify = if joined { " AND COALESCE(sc.notify_desktop, 1) <> 0" } else { "" };
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
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, i64>(2)?))
        })
        .map_err(|error| format!("Failed to read Cindy schedules: {error}"))?;
    Ok(rows
        .flatten()
        .map(|(id, status, at)| CindyNotification {
            kind: if status == "success" { KIND_SCHEDULED } else { KIND_ERROR },
            key: format!("schedule:{id}"),
            at,
        })
        .collect())
}

fn finished_at_expression(columns: &HashSet<String>) -> Option<&'static str> {
    if columns.contains("finished_at") && columns.contains("fired_at") {
        Some("COALESCE(r.finished_at, r.fired_at)")
    } else if columns.contains("finished_at") {
        Some("r.finished_at")
    } else if columns.contains("fired_at") {
        Some("r.fired_at")
    } else {
        None
    }
}

/// 后台子任务（Cindy 的 `subagent_runs`）：跑完或失败的，算在「定时与后台任务」里。
/// 父会话已归档、子任务已回滚或删除的不提醒。
fn background_runs(
    connection: &Connection,
    session_columns: &HashSet<String>,
    since: i64,
) -> Result<Vec<CindyNotification>, String> {
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
        sql.push_str(&live_session_filters(session_columns, "s"));
    }
    let mut statement = connection
        .prepare(&sql)
        .map_err(|error| format!("Failed to read Cindy background tasks: {error}"))?;
    let rows = statement
        .query_map([since], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, i64>(2)?))
        })
        .map_err(|error| format!("Failed to read Cindy background tasks: {error}"))?;
    Ok(rows
        .flatten()
        .map(|(id, status, at)| CindyNotification {
            kind: if status == "completed" { KIND_SCHEDULED } else { KIND_ERROR },
            key: format!("background:{id}:{status}"),
            at,
        })
        .collect())
}

#[cfg(test)]
mod tests;
