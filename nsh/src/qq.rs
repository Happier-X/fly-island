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

pub trait Source: Send {
    fn read(&mut self) -> Result<Vec<Notification>, QqError>;
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
        Self { kind, reads: 0 }
    }

    #[cfg(test)]
    pub fn ok() -> Self {
        Self { kind: FixtureKind::Ok, reads: 0 }
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

}


use windows::{core::PCWSTR,
    Win32::{
        Foundation::{HWND, LPARAM, RECT},
        System::{
            Com::{CoInitializeEx, CoUninitialize, COINIT_MULTITHREADED},
            Diagnostics::ToolHelp::{
                CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W,
                TH32CS_SNAPPROCESS,
            },
        },
        Foundation::CloseHandle,
        UI::{
            Shell::ShellExecuteW,
            WindowsAndMessaging::{
                EnumWindows, GetClassNameW, GetWindowRect, GetWindowTextW, IsWindowVisible,
                SetForegroundWindow, ShowWindow, SW_RESTORE,
            },
        },
    },
    UI::Notifications::{
        KnownNotificationBindings, NotificationKinds,
        Management::{UserNotificationListener, UserNotificationListenerAccessStatus},
    },
};

const QQ_APP_ID: &str = "QQ";

pub fn is_qq_app_id(app_id: &str) -> bool {
    app_id.eq_ignore_ascii_case(QQ_APP_ID)
}

#[derive(Default)]
pub struct RealSource;

impl Source for RealSource {
    fn read(&mut self) -> Result<Vec<Notification>, QqError> {
        read_real()
    }
}

fn read_real() -> Result<Vec<Notification>, QqError> {
    let initialize = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
    let result = (|| {
        let listener = UserNotificationListener::Current()
            .map_err(|error| QqError::ReadFailed(format!("Windows 通知监听不可用：{error}")))?;
        let access = listener
            .GetAccessStatus()
            .map_err(|error| QqError::ReadFailed(format!("无法检查通知权限：{error}")))?;
        if !qq_running() {
            return Err(QqError::NotRunning);
        }
        if access != UserNotificationListenerAccessStatus::Allowed {
            return Err(if access == UserNotificationListenerAccessStatus::Denied {
                QqError::PermissionDenied
            } else {
                QqError::PermissionRequired
            });
        }
        let operation = listener
            .GetNotificationsAsync(NotificationKinds::Toast)
            .map_err(|error| QqError::ReadFailed(format!("无法读取 Windows 通知：{error}")))?;
        let notifications = pollster::block_on(operation)
            .map_err(|error| QqError::ReadFailed(format!("无法读取 Windows 通知：{error}")))?;
        let mut items = Vec::new();
        for notification in notifications {
            if let Ok(Some(item)) = from_notification(&notification) {
                items.push(item);
            }
        }
        Ok(items)
    })();
    if initialize.is_ok() {
        unsafe { CoUninitialize() };
    }
    result
}

fn from_notification(
    notification: &windows::UI::Notifications::UserNotification,
) -> windows::core::Result<Option<Notification>> {
    if !is_qq_app_id(&notification.AppInfo()?.AppUserModelId()?.to_string_lossy()) {
        return Ok(None);
    }
    let id = notification.Id()?.to_string();
    let created_at = notification.CreationTime()?.UniversalTime;
    let mut parts = Vec::new();
    if let Ok(elements) = notification
        .Notification()?
        .Visual()?
        .GetBinding(&KnownNotificationBindings::ToastGeneric()?)
        .and_then(|binding| binding.GetTextElements())
    {
        for element in elements {
            if let Ok(text) = element.Text() {
                parts.push(text.to_string_lossy());
            }
        }
    }
    let title = parts.first().cloned().unwrap_or_else(|| "QQ".to_string());
    let body = parts.into_iter().skip(1).collect::<Vec<_>>().join("\n");
    Ok(Some(Notification { id, title, body, created_at }))
}

/// Requests notification access once, or opens Windows settings when access was denied.
pub fn request_access() {
    std::thread::spawn(|| {
        let initialize = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
        let denied = (|| {
            let listener = UserNotificationListener::Current().ok()?;
            let access = listener.GetAccessStatus().ok()?;
            if access == UserNotificationListenerAccessStatus::Unspecified {
                let operation = listener.RequestAccessAsync().ok()?;
                let _ = pollster::block_on(operation);
            }
            Some(access == UserNotificationListenerAccessStatus::Denied)
        })()
        .unwrap_or(true);
        if initialize.is_ok() {
            unsafe { CoUninitialize() };
        }
        if denied {
            let _ = open_notification_settings();
        }
    });
}

fn qq_running() -> bool {
    // SAFETY: the snapshot handle is closed below; PROCESSENTRY32W declares its size.
    let Ok(snapshot) = (unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) }) else {
        return false;
    };
    let mut entry = PROCESSENTRY32W {
        dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
        ..Default::default()
    };
    let mut found = false;
    // SAFETY: entry is writable and sized; the snapshot is live.
    let mut next = unsafe { Process32FirstW(snapshot, &mut entry) }.is_ok();
    while next {
        let end = entry
            .szExeFile
            .iter()
            .position(|unit| *unit == 0)
            .unwrap_or(entry.szExeFile.len());
        if String::from_utf16_lossy(&entry.szExeFile[..end]).eq_ignore_ascii_case("QQ.exe") {
            found = true;
            break;
        }
        // SAFETY: same live snapshot and writable entry.
        next = unsafe { Process32NextW(snapshot, &mut entry) }.is_ok();
    }
    // SAFETY: the snapshot came from CreateToolhelp32Snapshot and is no longer used.
    let _ = unsafe { CloseHandle(snapshot) };
    found
}
pub fn open_notification_settings() -> Result<(), String> {
    open_url("ms-settings:privacy-notifications")
}

fn open_url(url: &str) -> Result<(), String> {
    let operation = wide("open");
    let target = wide(url);
    let result = unsafe {
        ShellExecuteW(
            None,
            PCWSTR(operation.as_ptr()),
            PCWSTR(target.as_ptr()),
            None,
            None,
            windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL,
        )
    };
    if result.0 as usize <= 32 {
        Err(format!("无法打开 {url}：{}", result.0 as usize))
    } else {
        Ok(())
    }
}

pub fn open_qq() -> Result<(), String> {
    let mut windows: Vec<(HWND, i64)> = Vec::new();
    unsafe {
        let _ = EnumWindows(Some(collect_qq_window), LPARAM(&mut windows as *mut _ as isize));
    }
    if let Some((window, _)) = windows.into_iter().max_by_key(|(_, area)| *area) {
        unsafe {
            let _ = ShowWindow(window, SW_RESTORE);
            let _ = SetForegroundWindow(window);
        }
        return Ok(());
    }
    let operation = wide("open");
    let target = wide("QQ.exe");
    let result = unsafe {
        ShellExecuteW(
            None,
            PCWSTR(operation.as_ptr()),
            PCWSTR(target.as_ptr()),
            None,
            None,
            windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL,
        )
    };
    if result.0 as usize > 32 {
        Ok(())
    } else {
        Err("QQ 未运行，请先打开并登录桌面版 QQ。".to_string())
    }
}

unsafe extern "system" fn collect_qq_window(window: HWND, lparam: LPARAM) -> windows::core::BOOL {
    if !unsafe { IsWindowVisible(window) }.as_bool() {
        return true.into();
    }
    let mut title = [0u16; 64];
    // SAFETY: the buffer is writable and sized by the API.
    let length = unsafe { GetWindowTextW(window, &mut title) };
    if length <= 0 || String::from_utf16_lossy(&title[..length as usize]) != "QQ" {
        return true.into();
    }
    let mut class = [0u16; 128];
    // SAFETY: the buffer is writable and sized by the API.
    let length = unsafe { GetClassNameW(window, &mut class) };
    let class = String::from_utf16_lossy(&class[..length.max(0) as usize]);
    if class != "Chrome_WidgetWin_1" && class != "TXGuiFoundation" {
        return true.into();
    }
    let mut rect = RECT::default();
    // SAFETY: rect points at a writable RECT.
    if unsafe { GetWindowRect(window, &mut rect) }.is_ok() {
        let width = i64::from(rect.right - rect.left);
        let height = i64::from(rect.bottom - rect.top);
        if width >= 500 && height >= 300 {
            // SAFETY: lparam is the Vec passed by open_qq and lives for the call.
            let windows = unsafe { &mut *(lparam.0 as *mut Vec<(HWND, i64)>) };
            windows.push((window, width * height));
        }
    }
    true.into()
}

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

#[cfg(test)]
pub fn live_probe() -> Result<(usize, usize), String> {
    let initialize = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
    let result = (|| {
        let listener = UserNotificationListener::Current().map_err(|error| error.to_string())?;
        let access = listener.GetAccessStatus().map_err(|error| error.to_string())?;
        if access != UserNotificationListenerAccessStatus::Allowed {
            return Ok((0, 0));
        }
        let operation = listener
            .GetNotificationsAsync(NotificationKinds::Toast)
            .map_err(|error| error.to_string())?;
        let notifications = pollster::block_on(operation).map_err(|error| error.to_string())?;
        let mut total = 0;
        let mut qq = 0;
        for notification in notifications {
            total += 1;
            if notification
                .AppInfo()
                .and_then(|info| info.AppUserModelId())
                .is_ok_and(|id| is_qq_app_id(&id.to_string_lossy()))
            {
                qq += 1;
            }
        }
        Ok((total, qq))
    })();
    if initialize.is_ok() {
        unsafe { CoUninitialize() };
    }
    result
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
            source: if env::var_os("NSH_QQ_FIXTURE").is_some() {
                Box::new(Fixture::from_environment())
            } else {
                Box::new(RealSource)
            },
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
    fn only_the_qq_app_id_is_accepted() {
        assert!(is_qq_app_id("QQ"));
        assert!(is_qq_app_id("qq"));
        assert!(!is_qq_app_id("QQMail"));
        assert!(!is_qq_app_id("WeChat"));
        assert!(!is_qq_app_id(""));
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
    }

    #[test]
    #[ignore = "reads the real Windows notification center"]
    fn live_notifications_only_count_qq() {
        match super::live_probe() {
            Ok((total, qq)) => {
                println!("notifications: {total}, QQ: {qq}");
                assert!(qq <= total);
            }
            Err(error) => println!("notification probe skipped: {error}"),
        }
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
