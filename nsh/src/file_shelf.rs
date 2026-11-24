//! File shelf: file paths dropped on the island, kept as references (never copied).
//!
//! The list lives in the NSH data directory. Tauri's rules are kept: absolute paths
//! only, case-insensitive duplicate paths, pinned entries first, and at most 100
//! entries (20..200 when configured).

use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

#[cfg(feature = "embedded")]
use std::sync::atomic::{AtomicU64, Ordering};
use serde::{Deserialize, Serialize};

#[allow(dead_code)]
pub const MAX_ITEMS: usize = 100;
pub const MIN_MAX_ITEMS: usize = 20;
pub const MAX_MAX_ITEMS: usize = 200;
const FILE_NAME: &str = "file-shelf.json";
const DAY_MILLIS: i64 = 24 * 60 * 60 * 1_000;

#[cfg(feature = "embedded")]
static NEXT_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Item {
    pub id: String,
    pub path: String,
    pub name: String,
    #[serde(default)]
    pub is_dir: bool,
    #[serde(default)]
    pub size: u64,
    #[serde(default)]
    pub added_at: i64,
    #[serde(default)]
    pub pinned: bool,
    #[serde(skip)]
    pub missing: bool,
    #[serde(skip)]
    pub unreachable: bool,
}

#[derive(Deserialize)]
struct Stored {
    #[serde(default)]
    items: Vec<serde_json::Value>,
}

#[derive(Serialize)]
struct StoredRef<'a> {
    items: &'a [Item],
}

fn data_file() -> Option<PathBuf> {
    let base = std::env::var_os("APPDATA")?;
    Some(PathBuf::from(base).join("com.focusd.island.nsh").join(FILE_NAME))
}

fn now_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or_default()
}

/// Loads the list and applies the cleanup rules. Errors give an empty shelf.
pub fn load(max_items: usize, max_age_days: u32, remove_missing: bool) -> Vec<Item> {
    let Some(path) = data_file() else {
        return Vec::new();
    };
    let Ok(text) = fs::read_to_string(path) else {
        return Vec::new();
    };
    let Ok(stored) = serde_json::from_str::<Stored>(&text) else {
        return Vec::new();
    };
    let mut items: Vec<Item> = stored
        .items
        .into_iter()
        .filter_map(|value| serde_json::from_value(value).ok())
        .filter(|item: &Item| !item.id.is_empty() && !item.path.is_empty())
        .collect();
    refresh_presence(&mut items);
    prune(items, max_items, max_age_days, remove_missing, now_millis())
}

/// Saves atomically: a crash never leaves half a JSON file.
pub fn save(items: &[Item]) -> Result<(), String> {
    let path = data_file().ok_or_else(|| "找不到 NSH 数据目录。".to_string())?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| format!("创建文件中转目录失败：{error}"))?;
    }
    let text = serde_json::to_string_pretty(&StoredRef { items })
        .map_err(|error| format!("编码文件中转失败：{error}"))?;
    let temporary = path.with_extension("tmp");
    fs::write(&temporary, text).map_err(|error| format!("写入文件中转失败：{error}"))?;
    fs::rename(&temporary, &path).or_else(|_| {
        fs::remove_file(&path).ok();
        fs::rename(&temporary, &path)
    }).map_err(|error| format!("替换文件中转失败：{error}"))
}

/// Adds existing absolute paths, moving duplicates to the front instead of repeating them.
#[cfg(feature = "embedded")]
pub fn add_paths(existing: Vec<Item>, paths: &[String], max_items: usize) -> Vec<Item> {
    let now = now_millis();
    let mut items = existing;
    for path in paths {
        let Some(next) = describe(path, now) else {
            continue;
        };
        let key = path_key(&next.path);
        if let Some(index) = items.iter().position(|item| path_key(&item.path) == key) {
            let mut old = items.remove(index);
            old.name = next.name;
            old.is_dir = next.is_dir;
            old.size = next.size;
            old.missing = false;
            old.unreachable = false;
            items.insert(0, old);
        } else {
            items.insert(0, next);
        }
    }
    normalize(items, max_items)
}

pub fn remove(items: &mut Vec<Item>, id: &str) {
    items.retain(|item| item.id != id);
}

pub fn clear(items: &mut Vec<Item>) {
    items.clear();
}

pub fn toggle_pin(items: &mut [Item], id: &str) {
    if let Some(item) = items.iter_mut().find(|item| item.id == id) {
        item.pinned = !item.pinned;
    }
}

pub fn refresh_presence(items: &mut [Item]) {
    for item in items {
        let exists = Path::new(&item.path).exists();
        item.missing = !exists && !item.path.starts_with(r"\\");
        item.unreachable = !exists && item.path.starts_with(r"\\");
    }
}

#[cfg(windows)]
pub fn copy_text(text: &str) -> Result<(), String> {
    let mut last = String::new();
    for attempt in 0..5 {
        match clipboard_win::set_clipboard_string(text) {
            Ok(()) => return Ok(()),
            Err(error) => {
                last = format!("复制路径失败：{error}");
                if attempt < 4 {
                    std::thread::sleep(std::time::Duration::from_millis(80));
                }
            }
        }
    }
    Err(last)
}

#[cfg(not(windows))]
pub fn copy_text(_text: &str) -> Result<(), String> {
    Err("复制路径仅支持 Windows。".to_string())
}
pub fn format_size(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    if bytes == 0 {
        return "0 B".to_string();
    }
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    let digits = if unit == 0 || value >= 10.0 { 0 } else { 1 };
    format!("{value:.digits$} {}", UNITS[unit])
}

/// A short type badge used in place of Explorer's icon: PDF, PNG, ZIP, DIR, FILE.
pub fn badge(item: &Item) -> String {
    if item.is_dir {
        return "DIR".to_string();
    }
    let extension = Path::new(&item.name)
        .extension()
        .map(|extension| extension.to_string_lossy().to_uppercase())
        .unwrap_or_default();
    if extension.is_empty() || extension.chars().count() > 4 {
        "FILE".to_string()
    } else {
        extension
    }
}

pub fn path_key(path: &str) -> String {
    path.replace('/', "\\").to_lowercase()
}

#[cfg(feature = "embedded")]
fn describe(path: &str, now: i64) -> Option<Item> {
    let path = path.trim();
    if path.is_empty() || !Path::new(path).is_absolute() {
        return None;
    }
    let metadata = fs::metadata(path).ok()?;
    let name = Path::new(path)
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.to_string());
    let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
    Some(Item {
        id: format!("{now:x}-{id:x}"),
        path: path.to_string(),
        name,
        is_dir: metadata.is_dir(),
        size: if metadata.is_dir() { 0 } else { metadata.len() },
        added_at: now,
        pinned: false,
        missing: false,
        unreachable: false,
    })
}

fn prune(
    items: Vec<Item>,
    max_items: usize,
    max_age_days: u32,
    remove_missing: bool,
    now: i64,
) -> Vec<Item> {
    let max_age = i64::from(max_age_days) * DAY_MILLIS;
    let kept = items
        .into_iter()
        .filter(|item| item.pinned || max_age == 0 || now - item.added_at <= max_age)
        .filter(|item| !remove_missing || !item.missing)
        .collect();
    normalize(kept, max_items)
}

fn normalize(items: Vec<Item>, max_items: usize) -> Vec<Item> {
    let limit = max_items.clamp(MIN_MAX_ITEMS, MAX_MAX_ITEMS);
    let mut seen = HashSet::new();
    let (pinned, rest): (Vec<_>, Vec<_>) = items
        .into_iter()
        .filter(|item| seen.insert(path_key(&item.path)))
        .partition(|item| item.pinned);
    pinned.into_iter().chain(rest.into_iter().take(limit)).collect()
}

#[cfg(windows)]
fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(Some(0)).collect()
}

#[cfg(windows)]
fn shell_execute(verb: &str, file: &str, parameters: Option<&str>) -> Result<(), String> {
    use windows::{
        core::PCWSTR,
        Win32::UI::{Shell::ShellExecuteW, WindowsAndMessaging::SW_SHOWNORMAL},
    };
    let verb = wide(verb);
    let file = wide(file);
    let parameters = parameters.map(wide);
    let parameter_ptr = parameters
        .as_ref()
        .map_or(PCWSTR::null(), |value| PCWSTR(value.as_ptr()));
    // SAFETY: all strings are NUL-terminated and live through the call.
    let result = unsafe {
        ShellExecuteW(
            None,
            PCWSTR(verb.as_ptr()),
            PCWSTR(file.as_ptr()),
            parameter_ptr,
            PCWSTR::null(),
            SW_SHOWNORMAL,
        )
    };
    if result.0 as isize > 32 {
        Ok(())
    } else {
        Err(format!("无法打开：ShellExecute 返回 {}", result.0 as isize))
    }
}

#[cfg(windows)]
pub fn open(item: &Item) -> Result<(), String> {
    shell_execute("open", &item.path, None)
}

#[cfg(windows)]
pub fn reveal(item: &Item) -> Result<(), String> {
    let parameters = format!("/select,\"{}\"", item.path);
    shell_execute("open", "explorer.exe", Some(&parameters))
}

#[cfg(not(windows))]
pub fn open(_item: &Item) -> Result<(), String> {
    Err("文件打开仅支持 Windows。".to_string())
}

#[cfg(not(windows))]
pub fn reveal(_item: &Item) -> Result<(), String> {
    Err("文件定位仅支持 Windows。".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(id: &str, path: &str) -> Item {
        Item {
            id: id.to_string(),
            path: path.to_string(),
            name: path.rsplit('\\').next().unwrap_or(path).to_string(),
            is_dir: false,
            size: 1,
            added_at: 0,
            pinned: false,
            missing: false,
            unreachable: false,
        }
    }

    #[test]
    fn paths_dedupe_case_and_slashes_and_keep_pinned_first() {
        let mut first = item("a", r"C:\Temp\A.txt");
        first.pinned = true;
        let items = normalize(vec![item("b", r"C:\Temp\B.txt"), first, item("c", r"C:/temp/a.TXT")], 100);
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].id, "a");
    }

    #[test]
    fn the_cap_keeps_the_newest_unpinned_entries() {
        let items: Vec<Item> = (0..25).map(|index| item(&index.to_string(), &format!(r"C:\Temp\{index}.txt"))).collect();
        let kept = normalize(items, MIN_MAX_ITEMS);
        assert_eq!(kept.len(), MIN_MAX_ITEMS);
        assert_eq!(kept.last().unwrap().id, "19");
    }

    #[test]
    fn size_and_badge_labels_match_the_shelf_rules() {
        assert_eq!(format_size(0), "0 B");
        assert_eq!(format_size(1_500_000), "1.4 MB");
        let mut file = item("a", r"C:\Temp\report.pdf");
        assert_eq!(badge(&file), "PDF");
        file.is_dir = true;
        assert_eq!(badge(&file), "DIR");
    }
}
