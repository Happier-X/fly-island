use super::*;

fn memory_database() -> Connection {
    let connection = Connection::open_in_memory().expect("in-memory database");
    connection
        .execute_batch(
            "CREATE TABLE sessions (
                id TEXT PRIMARY KEY, title TEXT, source TEXT DEFAULT 'desktop',
                status TEXT DEFAULT 'active', active_turn_started_at INTEGER,
                last_turn_ended_at INTEGER, hidden_at INTEGER,
                notifications_muted INTEGER DEFAULT 0
            );
            CREATE TABLE messages (
                id TEXT PRIMARY KEY, session_id TEXT, role TEXT,
                created_at INTEGER, rewind_at INTEGER
            );
            CREATE TABLE schedules (
                id TEXT PRIMARY KEY, name TEXT, notify_desktop INTEGER DEFAULT 1
            );
            CREATE TABLE schedule_runs (
                id TEXT PRIMARY KEY, schedule_id TEXT, status TEXT,
                fired_at INTEGER, finished_at INTEGER, read_at INTEGER
            );
            CREATE TABLE subagent_runs (
                id TEXT PRIMARY KEY, session_id TEXT, status TEXT, ended_at INTEGER,
                deleted_at INTEGER, rewind_at INTEGER
            );",
        )
        .expect("schema");
    connection
}

fn add_session(connection: &Connection, id: &str, started: Option<i64>, ended: Option<i64>) {
    connection
        .execute(
            "INSERT INTO sessions (id, active_turn_started_at, last_turn_ended_at)
             VALUES (?1, ?2, ?3)",
            (id, started, ended),
        )
        .expect("session");
}

fn add_message(connection: &Connection, id: &str, session: &str, role: &str, at: i64) {
    connection
        .execute(
            "INSERT INTO messages (id, session_id, role, created_at) VALUES (?1, ?2, ?3, ?4)",
            (id, session, role, at),
        )
        .expect("message");
}

fn collect_at(connection: &Connection, now: i64) -> Vec<CindyNotification> {
    collect(connection, now).expect("notifications")
}

fn kinds(items: &[CindyNotification]) -> Vec<&str> {
    items.iter().map(|item| item.kind).collect()
}

#[test]
fn a_finished_turn_is_a_done_notification() {
    let connection = memory_database();
    add_session(&connection, "s1", Some(1_000), Some(2_000));
    let items = collect_at(&connection, 10_000);
    assert_eq!(kinds(&items), ["done"]);
    assert_eq!(items[0].key, "done:s1:2000");
    assert_eq!(items[0].at, 2_000);
}

#[test]
fn a_turn_still_under_way_is_not_done() {
    let connection = memory_database();
    add_session(&connection, "s1", Some(3_000), Some(2_000));
    assert!(collect_at(&connection, 10_000).is_empty());
}

#[test]
fn a_scheduled_session_reports_through_its_schedule_run() {
    let connection = memory_database();
    add_session(&connection, "s1", Some(1_000), Some(2_000));
    connection
        .execute("UPDATE sessions SET source = 'scheduler'", [])
        .expect("source");
    assert!(collect_at(&connection, 10_000).is_empty());
}

#[test]
fn finished_work_older_than_the_window_is_left_out() {
    let connection = memory_database();
    add_session(&connection, "s1", Some(1_000), Some(2_000));
    assert!(collect_at(&connection, WINDOW_MS + 10_000).is_empty());
}

#[test]
fn a_muted_session_is_left_out() {
    let connection = memory_database();
    add_session(&connection, "s1", Some(1_000), Some(2_000));
    connection
        .execute("UPDATE sessions SET notifications_muted = 1", [])
        .expect("mute");
    assert!(collect_at(&connection, 10_000).is_empty());
}

#[test]
fn an_unanswered_question_waits_for_the_user() {
    let connection = memory_database();
    add_session(&connection, "s1", Some(3_000), Some(2_000));
    add_message(&connection, "m1", "s1", "assistant", 3_500);
    add_message(&connection, "m2", "s1", "ask_user", 4_000);
    let items = collect_at(&connection, 10_000);
    assert_eq!(kinds(&items), ["question"]);
    assert_eq!(items[0].key, "question:s1:m2");
}

#[test]
fn an_answered_question_does_not_wait() {
    let connection = memory_database();
    add_session(&connection, "s1", Some(3_000), Some(2_000));
    add_message(&connection, "m1", "s1", "ask_user", 4_000);
    add_message(&connection, "m2", "s1", "user", 5_000);
    assert!(collect_at(&connection, 10_000).is_empty());
}

#[test]
fn a_rewound_question_is_ignored() {
    let connection = memory_database();
    add_session(&connection, "s1", Some(3_000), Some(2_000));
    add_message(&connection, "m1", "s1", "ask_user", 4_000);
    connection
        .execute("UPDATE messages SET rewind_at = 4_500", [])
        .expect("rewind");
    assert!(collect_at(&connection, 10_000).is_empty());
}

#[test]
fn a_question_survives_the_done_window() {
    let connection = memory_database();
    add_session(&connection, "s1", Some(3_000), Some(2_000));
    add_message(&connection, "m1", "s1", "ask_user", 4_000);
    assert_eq!(kinds(&collect_at(&connection, WINDOW_MS * 10)), ["question"]);
}

#[test]
fn an_error_message_is_an_error_notification() {
    let connection = memory_database();
    add_session(&connection, "s1", Some(3_000), Some(2_000));
    add_message(&connection, "m1", "s1", "error", 5_000);
    let items = collect_at(&connection, 10_000);
    assert_eq!(kinds(&items), ["error"]);
    assert_eq!(items[0].key, "error:m1");
}

#[test]
fn an_unread_successful_schedule_run_is_scheduled() {
    let connection = memory_database();
    connection
        .execute_batch(
            "INSERT INTO schedules (id, notify_desktop) VALUES ('job', 1);
             INSERT INTO schedule_runs (id, schedule_id, status, fired_at, finished_at, read_at)
             VALUES ('run', 'job', 'success', 4000, 5000, NULL);",
        )
        .expect("run");
    let items = collect_at(&connection, 10_000);
    assert_eq!(kinds(&items), ["scheduled"]);
    assert_eq!(items[0].at, 5_000, "finished_at 优先于 fired_at");
}

#[test]
fn a_failed_schedule_run_is_an_error() {
    let connection = memory_database();
    connection
        .execute_batch(
            "INSERT INTO schedules (id, notify_desktop) VALUES ('job', 1);
             INSERT INTO schedule_runs (id, schedule_id, status, fired_at, finished_at, read_at)
             VALUES ('run', 'job', 'failed', 4000, 5000, NULL);",
        )
        .expect("run");
    assert_eq!(kinds(&collect_at(&connection, 10_000)), ["error"]);
}

#[test]
fn read_and_muted_schedule_runs_are_left_out() {
    let connection = memory_database();
    connection
        .execute_batch(
            "INSERT INTO schedules (id, notify_desktop) VALUES ('job', 1), ('quiet', 0);
             INSERT INTO schedule_runs (id, schedule_id, status, fired_at, finished_at, read_at)
             VALUES ('read', 'job', 'success', 4000, 5000, 6000),
                    ('muted', 'quiet', 'success', 4000, 5000, NULL);",
        )
        .expect("runs");
    assert!(collect_at(&connection, 10_000).is_empty());
}

#[test]
fn background_runs_end_as_scheduled_or_error() {
    let connection = memory_database();
    add_session(&connection, "s1", Some(3_000), Some(2_000));
    connection
        .execute_batch(
            "INSERT INTO subagent_runs (id, session_id, status, ended_at)
             VALUES ('ok', 's1', 'completed', 4000), ('bad', 's1', 'failed', 5000);",
        )
        .expect("runs");
    let items = collect_at(&connection, 10_000);
    assert_eq!(kinds(&items), ["error", "scheduled"], "新的在前");
}

#[test]
fn background_runs_of_archived_sessions_are_left_out() {
    let connection = memory_database();
    add_session(&connection, "s1", Some(3_000), Some(2_000));
    connection
        .execute_batch(
            "UPDATE sessions SET status = 'archived' WHERE id = 's1';
             INSERT INTO subagent_runs (id, session_id, status, ended_at)
             VALUES ('ok', 's1', 'completed', 4000);",
        )
        .expect("run");
    assert!(collect_at(&connection, 10_000).is_empty());
}

#[test]
fn a_database_without_cindy_tables_has_no_notifications() {
    let connection = Connection::open_in_memory().expect("in-memory database");
    assert!(collect_at(&connection, 10_000).is_empty());
}

/// A file-based database, as Cindy keeps it, under the system temp folder.
fn temporary_database_path() -> std::path::PathBuf {
    use std::time::{SystemTime, UNIX_EPOCH};
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    std::env::temp_dir().join(format!("fly-island-cindy-{}-{stamp}.db", std::process::id()))
}

#[test]
fn a_temporary_database_file_answers_like_cindy_does() {
    let path = temporary_database_path();
    {
        let connection = Connection::open(&path).expect("temporary database");
        connection
            .execute_batch(
                "CREATE TABLE sessions (
                    id TEXT PRIMARY KEY, source TEXT DEFAULT 'desktop', status TEXT DEFAULT 'active',
                    active_turn_started_at INTEGER, last_turn_ended_at INTEGER,
                    hidden_at INTEGER, notifications_muted INTEGER DEFAULT 0
                );
                CREATE TABLE messages (
                    id TEXT PRIMARY KEY, session_id TEXT, role TEXT, created_at INTEGER, rewind_at INTEGER
                );
                CREATE TABLE schedules (id TEXT PRIMARY KEY, notify_desktop INTEGER DEFAULT 1);
                CREATE TABLE schedule_runs (
                    id TEXT PRIMARY KEY, schedule_id TEXT, status TEXT,
                    fired_at INTEGER, finished_at INTEGER, read_at INTEGER
                );
                CREATE TABLE subagent_runs (
                    id TEXT PRIMARY KEY, session_id TEXT, status TEXT, ended_at INTEGER,
                    deleted_at INTEGER, rewind_at INTEGER
                );
                INSERT INTO sessions (id, active_turn_started_at, last_turn_ended_at)
                VALUES ('done', 1000, 2000), ('asking', 3000, 2000), ('waiting', 3000, 2000);
                INSERT INTO messages (id, session_id, role, created_at)
                VALUES ('q', 'asking', 'ask_user', 4000), ('e', 'waiting', 'error', 5000);
                INSERT INTO schedules (id) VALUES ('job');
                INSERT INTO schedule_runs (id, schedule_id, status, fired_at, finished_at, read_at)
                VALUES ('run', 'job', 'success', 4000, 5000, NULL);
                INSERT INTO subagent_runs (id, session_id, status, ended_at)
                VALUES ('bg', 'done', 'failed', 6000);",
            )
            .expect("rows");
    }
    let connection = open_read_only(&path).expect("read-only connection");
    let items = collect(&connection, 10_000).expect("notifications");
    let mut kinds = kinds(&items);
    kinds.sort_unstable();
    assert_eq!(kinds, ["done", "error", "error", "question", "scheduled"]);
    drop(connection);
    let _ = std::fs::remove_file(&path);
}

#[test]
#[ignore = "reads Cindy's database on the machine running the tests"]
fn the_cindy_database_on_this_machine_answers() {
    let Some(path) = database_path() else {
        println!("no Cindy database here");
        return;
    };
    let connection = open_read_only(&path).expect("read-only connection");
    let items = collect(&connection, crate::cindy_notifications::now_unix_millis()).expect("notifications");
    for kind in ["done", "question", "scheduled", "error"] {
        let count = items.iter().filter(|item| item.kind == kind).count();
        println!("{kind}: {count}");
    }
}
