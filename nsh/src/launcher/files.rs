//! Files under the folders picked for the quick launcher.

use std::{
    fs,
    os::windows::fs::MetadataExt,
    path::Path,
    sync::{Mutex, MutexGuard, PoisonError},
    time::{Duration, Instant},
};

use super::{EntryKind, LauncherEntry};

const MAX_DEPTH: usize = 3;
const MAX_FILES: usize = 3_000;
const CACHE_TTL: Duration = Duration::from_secs(30);
const FILE_ATTRIBUTE_HIDDEN: u32 = 0x2;
const FILE_ATTRIBUTE_SYSTEM: u32 = 0x4;
const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
const SKIPPED: [&str; 7] = [
    "node_modules", "target", ".git", "$recycle.bin",
    "system volume information", "appdata", "windows",
];

struct CachedScan {
    key: String,
    scanned_at: Instant,
    entries: Vec<LauncherEntry>,
}

static CACHE: Mutex<Option<CachedScan>> = Mutex::new(None);

fn lock() -> MutexGuard<'static, Option<CachedScan>> {
    CACHE.lock().unwrap_or_else(PoisonError::into_inner)
}

pub(super) fn indexed(folders: &[String]) -> Vec<LauncherEntry> {
    let folders = normalized_folders(folders);
    let key = folders.join("\n");
    if let Some(cached) = lock()
        .as_ref()
        .filter(|cached| cached.key == key && cached.scanned_at.elapsed() < CACHE_TTL)
    {
        return cached.entries.clone();
    }
    let entries = scan(&folders);
    *lock() = Some(CachedScan { key, scanned_at: Instant::now(), entries: entries.clone() });
    entries
}

fn normalized_folders(folders: &[String]) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    folders
        .iter()
        .map(|folder| folder.trim().to_string())
        .filter(|folder| Path::new(folder).is_absolute() && Path::new(folder).is_dir())
        .filter(|folder| seen.insert(folder.to_lowercase()))
        .collect()
}

fn scan(folders: &[String]) -> Vec<LauncherEntry> {
    let mut entries = Vec::new();
    for folder in folders {
        walk(Path::new(folder), MAX_DEPTH, &mut entries);
        if entries.len() >= MAX_FILES {
            break;
        }
    }
    entries.truncate(MAX_FILES);
    entries
}
fn walk(directory: &Path, depth: usize, entries: &mut Vec<LauncherEntry>) {
    if entries.len() >= MAX_FILES {
        return;
    }
    let Ok(read) = fs::read_dir(directory) else {
        return;
    };
    for item in read.flatten() {
        if entries.len() >= MAX_FILES {
            return;
        }
        let name = item.file_name().to_string_lossy().to_string();
        if name.starts_with('.') || name.starts_with('~') {
            continue;
        }
        let path = item.path();
        let Ok(kind) = item.file_type() else {
            continue;
        };
        if kind.is_symlink() {
            continue;
        }
        if item.metadata().is_ok_and(|metadata| {
            metadata.file_attributes() & (FILE_ATTRIBUTE_HIDDEN | FILE_ATTRIBUTE_SYSTEM | FILE_ATTRIBUTE_REPARSE_POINT) != 0
        }) {
            continue;
        }
        if kind.is_dir() {
            if depth > 0 && !SKIPPED.contains(&name.to_lowercase().as_str()) {
                walk(&path, depth - 1, entries);
            }
            continue;
        }
        let Some(target) = path.to_str() else {
            continue;
        };
        entries.push(LauncherEntry {
            id: format!("file:{}", target.to_lowercase()),
            name,
            subtitle: directory.to_string_lossy().into_owned(),
            target: target.to_string(),
            kind: EntryKind::File,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp(tag: &str) -> std::path::PathBuf {
        let nanos = SystemTime::now().duration_since(UNIX_EPOCH).map(|time| time.as_nanos()).unwrap_or(0);
        let path = std::env::temp_dir().join(format!("nsh-launcher-{tag}-{}-{nanos}", std::process::id()));
        fs::create_dir_all(&path).unwrap();
        path
    }

    #[test]
    fn files_are_found_up_to_the_depth_limit() {
        let root = temp("depth");
        fs::create_dir_all(root.join("a").join("b")).unwrap();
        fs::write(root.join("top.txt"), "x").unwrap();
        fs::write(root.join("a").join("middle.txt"), "x").unwrap();
        fs::write(root.join("a").join("b").join("deep.txt"), "x").unwrap();
        let mut entries = Vec::new();
        walk(&root, 1, &mut entries);
        let mut names: Vec<_> = entries.iter().map(|entry| entry.name.as_str()).collect();
        names.sort_unstable();
        let _ = fs::remove_dir_all(&root);
        assert_eq!(names, ["middle.txt", "top.txt"]);
    }

    #[test]
    fn a_cached_scan_is_reused_until_the_folders_change() {
        let root = temp("cache");
        fs::write(root.join("before.txt"), "x").unwrap();
        let folders = vec![root.to_string_lossy().into_owned()];
        let first = indexed(&folders);
        fs::write(root.join("after.txt"), "x").unwrap();
        let second = indexed(&folders);
        let _ = fs::remove_dir_all(&root);
        assert_eq!(first.len(), 1);
        assert_eq!(second.len(), 1);
    }
}