//! Installed applications, files and web results for the quick launcher.

mod apps;
mod files;
pub mod hotkey;
mod matching;

use std::{
    path::Path,
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Mutex,
    },
    thread,
    time::{Duration, Instant},
};

use windows::Win32::UI::Shell::ShellExecuteW;

const APP_TTL: Duration = Duration::from_secs(300);
const FILE_TTL: Duration = Duration::from_secs(30);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LauncherEntry {
    pub id: String,
    pub name: String,
    pub subtitle: String,
    pub target: String,
    pub kind: EntryKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EntryKind { App, File }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResultKind { App, File, Web }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MatchKind { Exact, Prefix, Word, Contains, Path, Web }
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LauncherResult {
    pub id: String,
    pub name: String,
    pub subtitle: String,
    pub target: String,
    pub kind: ResultKind,
    pub match_kind: MatchKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SearchEngine { Bing, Google, Baidu, DuckDuckGo }

impl SearchEngine {
    pub fn parse(value: &str) -> Self {
        match value {
            "google" => Self::Google,
            "baidu" => Self::Baidu,
            "duckduckgo" => Self::DuckDuckGo,
            _ => Self::Bing,
        }
    }

    fn host(self) -> &'static str {
        match self {
            Self::Bing => "www.bing.com",
            Self::Google => "www.google.com",
            Self::Baidu => "www.baidu.com",
            Self::DuckDuckGo => "duckduckgo.com",
        }
    }

    fn url(self, query: &str) -> String {
        let (base, key) = match self {
            Self::Baidu => ("https://www.baidu.com/s", "wd"),
            Self::Bing => ("https://www.bing.com/search", "q"),
            Self::Google => ("https://www.google.com/search", "q"),
            Self::DuckDuckGo => ("https://duckduckgo.com/", "q"),
        };
        reqwest::Url::parse_with_params(base, &[(key, query)])
            .map_or_else(|_| base.to_string(), |url| url.to_string())
    }
}

pub struct Snapshot {
    pub apps: Vec<LauncherEntry>,
    pub files: Vec<LauncherEntry>,
    pub apps_building: bool,
    pub files_building: bool,
}
struct AppCache {
    entries: Vec<LauncherEntry>,
    built_at: Instant,
}

struct FileCache {
    key: String,
    entries: Vec<LauncherEntry>,
    built_at: Instant,
}

static APPS: Mutex<Option<AppCache>> = Mutex::new(None);
static FILES: Mutex<Option<FileCache>> = Mutex::new(None);
static APPS_BUILDING: AtomicBool = AtomicBool::new(false);
static FILES_BUILDING: AtomicBool = AtomicBool::new(false);
static REVISION: AtomicU64 = AtomicU64::new(0);

pub fn revision() -> u64 {
    REVISION.load(Ordering::SeqCst)
}

/// Starts background builds for the parts that are missing or stale.
pub fn ensure_index(include_apps: bool, include_files: bool, folders: &[String]) {
    if include_apps {
        let stale = APPS
            .lock()
            .ok()
            .and_then(|cache| cache.as_ref().map(|cache| cache.built_at.elapsed() >= APP_TTL))
            .unwrap_or(true);
        if stale && !APPS_BUILDING.swap(true, Ordering::SeqCst) {
            thread::spawn(|| {
                let entries = apps::enumerate();
                if let Ok(mut cache) = APPS.lock() {
                    *cache = Some(AppCache { entries, built_at: Instant::now() });
                }
                APPS_BUILDING.store(false, Ordering::SeqCst);
                REVISION.fetch_add(1, Ordering::SeqCst);
            });
        }
    }
    if include_files {
        let key = files_key(folders);
        let stale = FILES
            .lock()
            .ok()
            .and_then(|cache| cache.as_ref().map(|cache| cache.key != key || cache.built_at.elapsed() >= FILE_TTL))
            .unwrap_or(true);
        if stale && !FILES_BUILDING.swap(true, Ordering::SeqCst) {
            let folders = folders.to_vec();
            thread::spawn(move || {
                let entries = files::indexed(&folders);
                if let Ok(mut cache) = FILES.lock() {
                    *cache = Some(FileCache { key: files_key(&folders), entries, built_at: Instant::now() });
                }
                FILES_BUILDING.store(false, Ordering::SeqCst);
                REVISION.fetch_add(1, Ordering::SeqCst);
            });
        }
    }
}
pub fn snapshot(include_apps: bool, include_files: bool) -> Snapshot {
    let apps = if include_apps {
        APPS.lock().ok().and_then(|cache| cache.as_ref().map(|cache| cache.entries.clone())).unwrap_or_default()
    } else {
        Vec::new()
    };
    let files = if include_files {
        FILES.lock().ok().and_then(|cache| cache.as_ref().map(|cache| cache.entries.clone())).unwrap_or_default()
    } else {
        Vec::new()
    };
    Snapshot {
        apps,
        files,
        apps_building: include_apps && APPS_BUILDING.load(Ordering::SeqCst),
        files_building: include_files && FILES_BUILDING.load(Ordering::SeqCst),
    }
}

fn files_key(folders: &[String]) -> String {
    folders.iter().map(|folder| folder.trim().to_lowercase()).collect::<Vec<_>>().join("\n")
}

pub fn display_under_cursor() -> Option<nsh_core::desktop::DisplayId> {
    use windows::Win32::Foundation::POINT;
    use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;

    let mut point = POINT::default();
    // SAFETY: GetCursorPos writes a POINT into the live value.
    unsafe { GetCursorPos(&mut point) }.ok().and_then(|()| display_at_point(point))
}

fn display_at_point(point: windows::Win32::Foundation::POINT) -> Option<nsh_core::desktop::DisplayId> {
    use windows::core::PCWSTR;
    use windows::Win32::{
        Graphics::Gdi::{
            EnumDisplayDevicesW, GetMonitorInfoW, MonitorFromPoint, DISPLAY_DEVICEW,
            MONITORINFO, MONITORINFOEXW, MONITOR_DEFAULTTONEAREST,
        },
        UI::WindowsAndMessaging::EDD_GET_DEVICE_INTERFACE_NAME,
    };
    let mut info = MONITORINFOEXW::default();
    info.monitorInfo.cbSize = std::mem::size_of::<MONITORINFOEXW>() as u32;
    // SAFETY: the monitor comes from the live point; info declares its size.
    let monitor = unsafe { MonitorFromPoint(point, MONITOR_DEFAULTTONEAREST) };
    if !unsafe { GetMonitorInfoW(monitor, &mut info as *mut _ as *mut MONITORINFO) }.as_bool() {
        return primary_display();
    }
    let mut device = DISPLAY_DEVICEW {
        cb: std::mem::size_of::<DISPLAY_DEVICEW>() as u32,
        ..Default::default()
    };
    // SAFETY: szDevice is NUL-terminated; device declares its size.
    if unsafe {
        EnumDisplayDevicesW(
            PCWSTR(info.szDevice.as_ptr()),
            0,
            &mut device,
            EDD_GET_DEVICE_INTERFACE_NAME,
        )
    }
    .as_bool()
        && device.DeviceID[0] != 0
    {
        let end = device.DeviceID.iter().position(|unit| *unit == 0).unwrap_or(device.DeviceID.len());
        let name = String::from_utf16_lossy(&device.DeviceID[..end]);
        return Some(nsh_core::desktop::DisplayId::from_name(name.as_bytes()));
    }
    primary_display()
}

fn primary_display() -> Option<nsh_core::desktop::DisplayId> {
    nsh_core::element::desktop::displays()
        .iter()
        .find(|display| display.primary)
        .map(|display| display.id)
}
pub fn open_target(target: &str) -> Result<(), String> {
    let lower = target.to_ascii_lowercase();
    let allowed = lower.starts_with("http://")
        || lower.starts_with("https://")
        || lower.starts_with("shell:appsfolder\\")
        || Path::new(target).is_absolute();
    if !allowed {
        return Err(format!("无法打开「{target}」：目标不是绝对路径或网址。"));
    }
    let operation: Vec<u16> = "open".encode_utf16().chain(std::iter::once(0)).collect();
    let file: Vec<u16> = target.encode_utf16().chain(std::iter::once(0)).collect();
    // SAFETY: both strings are NUL-terminated and live for the call.
    let result = unsafe {
        ShellExecuteW(
            None,
            windows::core::PCWSTR(operation.as_ptr()),
            windows::core::PCWSTR(file.as_ptr()),
            None,
            None,
            windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL,
        )
    };
    if result.0 as usize <= 32 {
        Err(format!("无法打开「{target}」：{}", result.0 as usize))
    } else {
        Ok(())
    }
}
pub fn effective_folders(mode: &str, custom: &[String]) -> Vec<String> {
    if mode == "custom" { custom.to_vec() } else { default_folders() }
}

pub fn default_folders() -> Vec<String> {
    let Some(home) = std::env::var_os("USERPROFILE") else {
        return Vec::new();
    };
    let home = std::path::PathBuf::from(home);
    ["Desktop", "Documents", "Downloads"]
        .into_iter()
        .map(|name| home.join(name))
        .filter(|path| path.is_dir())
        .map(|path| path.to_string_lossy().into_owned())
        .collect()
}

pub fn search(
    entries: &[LauncherEntry],
    query: &str,
    limit: usize,
    search_web: bool,
    engine: SearchEngine,
) -> Vec<LauncherResult> {
    matching::search(entries, query, limit, search_web, engine)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(name: &str, target: &str, kind: EntryKind) -> LauncherEntry {
        LauncherEntry { id: format!("{:?}:{target}", kind), name: name.into(), subtitle: target.into(), target: target.into(), kind }
    }

    #[test]
    #[ignore = "reads the real monitor arrangement"]
    fn the_secondary_monitor_has_a_different_display_id() {
        use windows::Win32::Foundation::POINT;
        let primary = display_at_point(POINT { x: 100, y: 100 });
        let secondary = display_at_point(POINT { x: 3000, y: 700 });
        println!("primary={primary:?} secondary={secondary:?}");
        assert!(primary.is_some());
        assert!(secondary.is_some());
        assert_ne!(primary, secondary);
    }
    #[test]
    #[ignore = "reads the real Start Menu, AppX list and PATH"]
    fn the_real_app_index_has_unique_targets() {
        let first = apps::enumerate();
        let second = apps::enumerate();
        let unique: std::collections::HashSet<_> = first.iter().map(|entry| entry.target.to_lowercase()).collect();
        assert_eq!(unique.len(), first.len());
        assert!(!first.is_empty());
        println!("apps: {} first, {} second", first.len(), second.len());
    }
    #[test]
    fn exact_then_prefix_then_apps_before_files() {
        let entries = [
            entry("Visual Studio Code", "C:\\Code.exe", EntryKind::App),
            entry("Code", "C:\\Code.txt", EntryKind::File),
            entry("Code Helper", "C:\\Helper.exe", EntryKind::App),
        ];
        let results = search(&entries, "code", 30, false, SearchEngine::Bing);
        assert_eq!(results[0].name, "Code");
        assert_eq!(results[1].name, "Code Helper");
    }

    #[test]
    fn only_absolute_paths_and_urls_are_opened() {
        assert!(open_target("relative\\file.txt").is_err());
        assert!(open_target("ftp://example.com/file").is_err());
    }
    #[test]
    fn urls_open_directly_and_empty_results_fall_back_to_web() {
        let results = search(&[], "https://example.com/a", 30, true, SearchEngine::Bing);
        assert_eq!(results[0].kind, ResultKind::Web);
        assert_eq!(results[0].target, "https://example.com/a");
        let web = search(&[], "no such app", 30, true, SearchEngine::Baidu);
        assert_eq!(web[0].target, "https://www.baidu.com/s?wd=no+such+app");
    }
}
