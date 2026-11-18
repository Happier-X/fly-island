//! Files under the folders the user picked for the quick launcher. The scan is bounded
//! in depth and count, and its result is cached briefly, so opening the panel never
//! walks a whole drive.

use std::{
    fs,
    os::windows::fs::MetadataExt,
    path::Path,
    sync::{Mutex, MutexGuard, PoisonError},
    time::{Duration, Instant},
};

use super::{LauncherEntry, LauncherEntryKind};

/// Folders below each configured one that are searched. Three levels reach
/// `Documents\Project\src\main.rs` without turning into a drive scan.
const MAX_DEPTH: usize = 3;

/// A panel shows a few dozen matches at most; this keeps the first scan responsive.
const MAX_FILES: usize = 3_000;

/// Windows' hidden and system attributes; these files are not what a launcher is for.
const FILE_ATTRIBUTE_HIDDEN: u32 = 0x2;
const FILE_ATTRIBUTE_SYSTEM: u32 = 0x4;
const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;

/// Folders that are large and rarely meant by "my files", or that point back up.
const SKIPPED_DIRECTORIES: [&str; 7] = [
    "node_modules",
    "target",
    ".git",
    "$recycle.bin",
    "system volume information",
    "appdata",
    "windows",
];

/// How long a scan is reused. Opening the panel again within this window shows the same
/// files; past it, a fresh scan picks up changes.
const CACHE_TTL: Duration = Duration::from_secs(30);

struct CachedScan {
    key: String,
    scanned_at: Instant,
    entries: Vec<LauncherEntry>,
}

static CACHE: Mutex<Option<CachedScan>> = Mutex::new(None);

fn lock() -> MutexGuard<'static, Option<CachedScan>> {
    CACHE.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Files for `folders`, from the cache while it is fresh and for the same folders.
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
    *lock() = Some(CachedScan {
        key,
        scanned_at: Instant::now(),
        entries: entries.clone(),
    });
    entries
}

fn normalized_folders(folders: &[String]) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    folders
        .iter()
        .map(|folder| folder.trim().to_string())
        .filter(|folder| {
            Path::new(folder).is_absolute()
                && Path::new(folder).parent().is_some()
                && Path::new(folder).is_dir()
        })
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
    for entry in read.flatten() {
        if entries.len() >= MAX_FILES {
            return;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') {
            continue;
        }
        let path = entry.path();
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if file_type.is_symlink() {
            continue;
        }
        if entry
            .metadata()
            .map(|metadata| metadata.file_attributes())
            .is_ok_and(|attributes| {
                attributes
                    & (FILE_ATTRIBUTE_HIDDEN | FILE_ATTRIBUTE_SYSTEM | FILE_ATTRIBUTE_REPARSE_POINT)
                    != 0
            })
        {
            continue;
        }
        if file_type.is_dir() {
            if depth > 0 && !SKIPPED_DIRECTORIES.contains(&name.to_lowercase().as_str()) {
                walk(&path, depth - 1, entries);
            }
            continue;
        }
        if name.starts_with('~') {
            continue;
        }
        let Some(target) = path.to_str() else {
            continue;
        };
        entries.push(LauncherEntry::new(
            LauncherEntryKind::File,
            name,
            directory.to_string_lossy().into_owned(),
            target.to_string(),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temporary_directory(tag: &str) -> std::path::PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let path = std::env::temp_dir().join(format!(
            "focusd-launcher-files-{tag}-{}-{nanos}",
            std::process::id()
        ));
        fs::create_dir_all(&path).expect("create the test folder");
        path
    }

    #[test]
    fn files_are_found_up_to_the_depth_limit() {
        let root = temporary_directory("depth");
        fs::create_dir_all(root.join("a").join("b")).expect("create folders");
        fs::write(root.join("top.txt"), "x").expect("write file");
        fs::write(root.join("a").join("middle.txt"), "x").expect("write file");
        fs::write(root.join("a").join("b").join("deep.txt"), "x").expect("write file");

        let mut entries = Vec::new();
        walk(&root, 1, &mut entries);
        let mut names: Vec<_> = entries.iter().map(|entry| entry.name.as_str()).collect();
        names.sort_unstable();
        let _ = fs::remove_dir_all(&root);

        assert_eq!(names, ["middle.txt", "top.txt"]);
    }

    #[test]
    fn heavy_and_hidden_folders_are_skipped() {
        let root = temporary_directory("skip");
        fs::create_dir_all(root.join("node_modules")).expect("create folders");
        fs::create_dir_all(root.join(".git")).expect("create folders");
        fs::write(root.join("node_modules").join("hidden.js"), "x").expect("write file");
        fs::write(root.join(".git").join("config"), "x").expect("write file");
        fs::write(root.join("kept.txt"), "x").expect("write file");

        let mut entries = Vec::new();
        walk(&root, 3, &mut entries);
        let names: Vec<_> = entries.iter().map(|entry| entry.name.as_str()).collect();
        let _ = fs::remove_dir_all(&root);

        assert_eq!(names, ["kept.txt"]);
    }

    #[test]
    fn a_cached_scan_is_reused_until_the_folders_change() {
        let root = temporary_directory("cache");
        fs::write(root.join("before.txt"), "x").expect("write file");
        let folders = vec![root.to_string_lossy().into_owned()];
        let first = indexed(&folders);
        fs::write(root.join("after.txt"), "x").expect("write file");
        let second = indexed(&folders);
        let _ = fs::remove_dir_all(&root);

        assert_eq!(first.len(), 1);
        assert_eq!(second.len(), 1, "the cached scan is reused");
    }
}
