//! QQ toast pipeline: a bounded in-memory buffer fed by a fixture source on
//! this machine, ready for the real UserNotificationListener later.

use std::{collections::HashMap, env};

pub const MAX_ITEMS: usize = 20;
const MERGE_WINDOW_MS: i64 = 5_000;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Notification {
    pub id: String,
    pub title: String,
    pub body: String,
    pub created_at: i64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    Disabled,
    Waiting,
    NotRunning,
    Listening,
    PermissionDenied,
    PermissionRequired,
    Error,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Snapshot {
    pub enabled: bool,
    pub status: Status,
    pub items: Vec<Notification>,
    pub error: Option<String>,
}

impl Default for Snapshot {
    fn default() -> Self {
        Self {
            enabled: false,
            status: Status::Disabled,
            items: Vec::new(),
            error: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum QqError {
    NotRunning,
    PermissionDenied,
    PermissionRequired,
    ReadFailed(String),
}

impl QqError {
    pub fn message(&self) -> String {
        match self {
            Self::NotRunning => "QQ 未运行，请先打开并登录桌面版 QQ。".to_string(),
            Self::PermissionDenied => "Windows 通知权限被拒绝，请在系统设置中允许。".to_string(),
            Self::PermissionRequired => "需要允许通知访问才能读取 QQ 消息。".to_string(),
            Self::ReadFailed(error) => format!("读取 QQ 通知失败：{error}"),
        }
    }

    fn status(&self) -> Status {
        match self {
            Self::NotRunning => Status::NotRunning,
            Self::PermissionDenied => Status::PermissionDenied,
            Self::PermissionRequired => Status::PermissionRequired,
            Self::ReadFailed(_) => Status::Error,
        }
    }
}

/// Port of the Tauri buffer: the first read is a baseline, later reads add
/// only new or updated toasts, a burst with the same title merges, and the
/// list never exceeds MAX_ITEMS.
#[derive(Default)]
pub struct Buffer {
    baseline: Option<HashMap<String, Notification>>,
    pub items: Vec<Notification>,
}

impl Buffer {
    pub fn sync(&mut self, mut current: Vec<Notification>) -> Option<String> {
        current.sort_by_key(|item| item.created_at);
        let additions: Vec<Notification> = self
            .baseline
            .as_ref()
            .map(|previous| {
                current
                    .iter()
                    .filter(|item| previous.get(&item.id) != Some(item))
                    .cloned()
                    .collect()
            })
            .unwrap_or_default();
        let mut newest = None;
        for item in additions {
            newest = Some(item.id.clone());
            self.insert(item);
        }
        self.baseline = Some(
            current
                .into_iter()
                .map(|item| (item.id.clone(), item))
                .collect(),
        );
        newest
    }

    fn insert(&mut self, item: Notification) {
        self.items.retain(|existing| existing.id != item.id);
        if let Some(existing) = self.items.iter_mut().find(|existing| {
            existing.title == item.title
                && item.created_at >= existing.created_at
                && item.created_at - existing.created_at <= MERGE_WINDOW_MS
        }) {
            if !item.body.is_empty() && !existing.body.contains(&item.body) {
                if !existing.body.is_empty() {
                    existing.body.push_str(" · ");
                }
                existing.body.push_str(&item.body);
            }
            existing.created_at = item.created_at;
            return;
        }
        self.items.insert(0, item);
        self.items.truncate(MAX_ITEMS);
    }
}

pub trait Source {
    fn read(&mut self) -> Result<Vec<Notification>, QqError>;
    fn open(&mut self) -> Result<(), String>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FixtureKind {
    Ok,
    Denied,
    Required,
    Error,
    NotRunning,
}

pub struct Fixture {
    kind: FixtureKind,
    reads: usize,
    opened: bool,
}

impl Fixture {
    pub fn from_environment() -> Self {
        let kind = match env::var("NSH_QQ_FIXTURE").unwrap_or_default().as_str() {
            "ok" => FixtureKind::Ok,
            "denied" => FixtureKind::Denied,
            "required" => FixtureKind::Required,
            "error" => FixtureKind::Error,
            _ => FixtureKind::NotRunning,
        };
        Self { kind, reads: 0, opened: false }
    }

    #[cfg(test)]
    pub fn ok() -> Self {
        Self { kind: FixtureKind::Ok, reads: 0, opened: false }
    }

    fn sample(id: &str, title: &str, body: &str, created_at: i64) -> Notification {
        Notification {
            id: id.to_string(),
            title: title.to_string(),
            body: body.to_string(),
            created_at,
        }
    }
}

impl Source for Fixture {
    fn read(&mut self) -> Result<Vec<Notification>, QqError> {
        self.reads += 1;
        match self.kind {
            FixtureKind::Denied => Err(QqError::PermissionDenied),
            FixtureKind::Required => Err(QqError::PermissionRequired),
            FixtureKind::Error => Err(QqError::ReadFailed("夹具读取失败".to_string())),
            FixtureKind::NotRunning => Err(QqError::NotRunning),
            FixtureKind::Ok => {
                let existing = Self::sample("1", "产品群", "existing", 1_700_000_000_000);
                let first = Self::sample("2", "设计群", "新消息", 1_700_000_001_000);
                let merged = Self::sample("3", "设计群", "后续消息", 1_700_000_002_000);
                let other = Self::sample("4", "朋友", "晚上好", 1_700_000_010_000);
                Ok(match self.reads {
                    1 => vec![existing],
                    2 => vec![existing, first, other],
                    _ => vec![existing, first, merged, other],
                })
            }
        }
    }

    fn open(&mut self) -> Result<(), String> {
        if self.opened {
            Ok(())
        } else {
            Err("QQ 未运行，请先打开并登录桌面版 QQ。".to_string())
        }
    }
}

pub struct Service {
    snapshot: Snapshot,
    buffer: Buffer,
    source: Box<dyn Source>,
}

impl Service {
    pub fn new(enabled: bool) -> Self {
        let mut service = Self {
            snapshot: Snapshot::default(),
            buffer: Buffer::default(),
            source: Box::new(Fixture::from_environment()),
        };
        service.set_enabled(enabled);
        service
    }

    pub fn snapshot(&self) -> Snapshot {
        self.snapshot.clone()
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.buffer = Buffer::default();
        self.snapshot = Snapshot {
            enabled,
            status: if enabled { Status::Waiting } else { Status::Disabled },
            ..Snapshot::default()
        };
    }

    pub fn clear(&mut self) {
        self.buffer.items.clear();
        self.snapshot.items.clear();
        if self.snapshot.enabled {
            self.snapshot.status = Status::Waiting;
            self.snapshot.error = None;
        }
    }

    pub fn poll(&mut self) -> Snapshot {
        if !self.snapshot.enabled {
            self.snapshot.status = Status::Disabled;
            self.snapshot.items.clear();
            self.snapshot.error = None;
            return self.snapshot();
        }
        match self.source.read() {
            Ok(current) => {
                self.buffer.sync(current);
                self.snapshot.status = Status::Listening;
                self.snapshot.error = None;
                self.snapshot.items = self.buffer.items.clone();
            }
            Err(error) => {
                self.snapshot.status = error.status();
                self.snapshot.error = Some(error.message());
                self.snapshot.items = self.buffer.items.clone();
            }
        }
        self.snapshot()
    }

    pub fn open(&mut self) -> Result<(), String> {
        self.source.open()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn notice(id: i64, title: &str) -> Notification {
        Notification {
            id: id.to_string(),
            title: title.to_string(),
            body: format!("body {id}"),
            created_at: id,
        }
    }

    #[test]
    fn startup_notifications_do_not_replay_and_new_messages_are_deduplicated() {
        let mut buffer = Buffer::default();
        assert_eq!(buffer.sync(vec![notice(1, "existing")]), None);
        assert!(buffer.items.is_empty());
        let current = vec![notice(1, "existing"), notice(2, "new")];
        assert_eq!(buffer.sync(current.clone()), Some("2".into()));
        assert_eq!(buffer.sync(current), None);
        assert_eq!(buffer.items, vec![notice(2, "new")]);
    }

    #[test]
    fn an_updated_toast_replaces_its_old_version() {
        let mut buffer = Buffer::default();
        buffer.sync(Vec::new());
        buffer.sync(vec![notice(1, "first")]);
        assert_eq!(buffer.sync(vec![notice(1, "updated")]), Some("1".into()));
        assert_eq!(buffer.items, vec![notice(1, "updated")]);
    }

    #[test]
    fn a_same_title_burst_merges_but_a_later_one_does_not() {
        let mut buffer = Buffer::default();
        buffer.sync(Vec::new());
        buffer.sync(vec![notice(1, "设计群")]);
        let mut second = notice(2, "设计群");
        second.created_at = 3_000;
        buffer.sync(vec![notice(1, "设计群"), second]);
        assert_eq!(buffer.items.len(), 1);
        assert!(buffer.items[0].body.contains("body 1"));
        assert!(buffer.items[0].body.contains("body 2"));
        let mut late = notice(3, "设计群");
        late.created_at = 20_000;
        buffer.sync(vec![notice(1, "设计群"), late]);
        assert_eq!(buffer.items.len(), 2);
    }

    #[test]
    fn the_buffer_stops_at_twenty_newest() {
        let mut buffer = Buffer::default();
        buffer.sync(Vec::new());
        buffer.sync((1..=25).map(|id| notice(id, &format!("title {id}"))).collect());
        assert_eq!(buffer.items.len(), MAX_ITEMS);
        assert_eq!(buffer.items.first().unwrap().id, "25");
        assert_eq!(buffer.items.last().unwrap().id, "6");
    }

    #[test]
    fn fixture_statuses_and_open_failure_are_explicit() {
        let mut fixture = Fixture::from_environment();
        fixture.kind = FixtureKind::Denied;
        assert_eq!(fixture.read(), Err(QqError::PermissionDenied));
        fixture.kind = FixtureKind::Required;
        assert_eq!(fixture.read(), Err(QqError::PermissionRequired));
        fixture.kind = FixtureKind::Error;
        assert_eq!(
            fixture.read(),
            Err(QqError::ReadFailed("夹具读取失败".to_string()))
        );
        let mut service = Service::new(true);
        service.source = Box::new(Fixture::from_environment());
        service.snapshot.status = Status::Error;
        assert!(service.open().is_err());
    }

    #[test]
    fn source_errors_reach_the_snapshot() {
        let mut service = Service::new(true);
        let mut fixture = Fixture::from_environment();
        fixture.kind = FixtureKind::NotRunning;
        service.source = Box::new(fixture);
        let snapshot = service.poll();
        assert_eq!(snapshot.status, Status::NotRunning);
        assert!(snapshot.error.unwrap().contains("QQ 未运行"));
        let mut fixture = Fixture::from_environment();
        fixture.kind = FixtureKind::Error;
        service.source = Box::new(fixture);
        let snapshot = service.poll();
        assert_eq!(snapshot.status, Status::Error);
        assert!(snapshot.error.unwrap().contains("读取失败"));
    }
    #[test]
    fn fixture_supplies_a_baseline_and_then_a_merged_burst() {
        let mut service = Service::new(true);
        service.source = Box::new(Fixture::ok());
        assert!(service.poll().items.is_empty());
        let second = service.poll();
        assert_eq!(second.status, Status::Listening);
        assert_eq!(second.items.len(), 2);
        let third = service.poll();
        assert_eq!(third.items.len(), 2);
        assert!(third.items.iter().any(|item| item.body.contains("后续消息")));
    }
}
