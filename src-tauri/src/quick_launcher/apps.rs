//! Installed applications for the quick launcher: Start Menu shortcuts (`.lnk`, `.url`,
//! `.appref-ms`) and the executables registered under App Paths.

use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
};

use windows::Win32::System::Registry::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE};

use super::{LauncherEntry, LauncherEntryKind};
use crate::win32::{registry, shortcut};

/// Subfolders searched below each Start Menu folder. Deep enough for vendor folders
/// (`Microsoft Office\Office16`) without walking forever.
const START_MENU_DEPTH: usize = 4;

/// Keeps a runaway Start Menu (or a junction loop) from building an endless index.
const MAX_APPS: usize = 2_000;

/// Programs whose shortcut is an uninstaller rather than something to launch.
const SKIPPED_NAME_FRAGMENTS: [&str; 3] = ["uninstall", "卸载", "readme"];

/// The apps start menu shortcuts and App Paths entries describe, deduplicated by what
/// they open and sorted by name.
pub(super) fn enumerate() -> Vec<LauncherEntry> {
    let mut entries = Vec::new();
    let mut seen = HashSet::new();
    for directory in start_menu_directories() {
        collect_shortcut_directory(&directory, START_MENU_DEPTH, &mut entries, &mut seen);
    }
    collect_app_paths(&mut entries, &mut seen);
    entries.sort_by_key(|entry| entry.name.to_lowercase());
    entries.truncate(MAX_APPS);
    entries
}

fn start_menu_directories() -> Vec<PathBuf> {
    let mut directories = Vec::new();
    if let Some(app_data) = std::env::var_os("APPDATA") {
        directories.push(PathBuf::from(app_data).join(r"Microsoft\Windows\Start Menu\Programs"));
    }
    if let Some(program_data) = std::env::var_os("PROGRAMDATA") {
        directories
            .push(PathBuf::from(program_data).join(r"Microsoft\Windows\Start Menu\Programs"));
    }
    directories.retain(|directory| directory.is_dir());
    directories
}

/// Walks `directory` up to `depth` folders deep and adds every shortcut whose target is
/// usable. Missing folders are skipped: a profile without one is normal.
fn collect_shortcut_directory(
    directory: &Path,
    depth: usize,
    entries: &mut Vec<LauncherEntry>,
    seen: &mut HashSet<String>,
) {
    let Ok(read) = fs::read_dir(directory) else {
        return;
    };
    for entry in read.flatten() {
        let path = entry.path();
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if file_type.is_dir() {
            if depth > 0 {
                collect_shortcut_directory(&path, depth - 1, entries, seen);
            }
            continue;
        }
        let Some(entry) = shortcut_entry(&path) else {
            continue;
        };
        if seen.insert(super::target_key(&entry.subtitle)) {
            entries.push(entry);
        }
    }
}

fn shortcut_entry(path: &Path) -> Option<LauncherEntry> {
    let extension = path
        .extension()
        .map(|extension| extension.to_string_lossy().to_lowercase())?;
    let name = path
        .file_stem()
        .map(|stem| stem.to_string_lossy().trim().to_string())
        .filter(|name| !name.is_empty())?;
    if is_skipped_name(&name) {
        return None;
    }
    let target = match extension.as_str() {
        "lnk" => shortcut::shortcut_target(&path.to_string_lossy())
            .unwrap_or_else(|| path.to_string_lossy().into_owned()),
        "url" => url_shortcut_target(path)?,
        "appref-ms" => path.to_string_lossy().into_owned(),
        _ => return None,
    };
    // Opening the original link preserves its arguments and working directory.
    let launch = if extension == "lnk" {
        path.to_str()?.to_owned()
    } else {
        target.clone()
    };
    Some(LauncherEntry::new(
        LauncherEntryKind::App,
        name,
        target,
        launch,
    ))
}

/// The `URL=` line of an Internet Shortcut, the only part needed to launch it.
fn url_shortcut_target(path: &Path) -> Option<String> {
    let text = fs::read_to_string(path).ok()?;
    text.lines()
        .find_map(|line| {
            let (key, value) = line.split_once('=')?;
            key.trim()
                .eq_ignore_ascii_case("url")
                .then(|| value.trim().to_string())
        })
        .filter(|url| !url.is_empty())
}

fn is_skipped_name(name: &str) -> bool {
    let lower = name.to_lowercase();
    SKIPPED_NAME_FRAGMENTS
        .iter()
        .any(|fragment| lower.contains(fragment))
}

fn collect_app_paths(entries: &mut Vec<LauncherEntry>, seen: &mut HashSet<String>) {
    for (root, key) in [
        (
            HKEY_LOCAL_MACHINE,
            r"SOFTWARE\Microsoft\Windows\CurrentVersion\App Paths",
        ),
        (
            HKEY_LOCAL_MACHINE,
            r"SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\App Paths",
        ),
        (
            HKEY_CURRENT_USER,
            r"SOFTWARE\Microsoft\Windows\CurrentVersion\App Paths",
        ),
    ] {
        for subkey in registry::subkeys(root, key) {
            let full_key = format!(r"{key}\{subkey}");
            let Some(executable) = registry::read_string(root, &full_key, "") else {
                continue;
            };
            let Some(entry) = app_path_entry(&subkey, &executable) else {
                continue;
            };
            if seen.insert(super::target_key(&entry.target)) {
                entries.push(entry);
            }
        }
    }
}

/// An App Paths registration, whose key is the executable's file name and whose default
/// value is where it lives (sometimes quoted).
fn app_path_entry(subkey: &str, executable: &str) -> Option<LauncherEntry> {
    let target = executable.trim().trim_matches('"');
    if target.is_empty() {
        return None;
    }
    let path = Path::new(target);
    if !path
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("exe"))
        || !path.is_file()
    {
        return None;
    }
    let name = Path::new(subkey)
        .file_stem()
        .map(|stem| stem.to_string_lossy().trim().to_string())
        .filter(|name| !name.is_empty() && !is_skipped_name(name))?;
    Some(LauncherEntry::new(
        LauncherEntryKind::App,
        name,
        target.to_string(),
        target.to_string(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temporary_directory(tag: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let path = std::env::temp_dir().join(format!(
            "focusd-launcher-{tag}-{}-{nanos}",
            std::process::id()
        ));
        fs::create_dir_all(&path).expect("create the test folder");
        path
    }

    #[test]
    fn internet_shortcuts_become_apps_named_after_their_file() {
        let directory = temporary_directory("url");
        fs::write(
            directory.join("Example Site.url"),
            "[InternetShortcut]\r\nURL=https://example.com/a\r\n",
        )
        .expect("write the shortcut");
        let mut entries = Vec::new();
        let mut seen = HashSet::new();
        collect_shortcut_directory(&directory, 2, &mut entries, &mut seen);
        let _ = fs::remove_dir_all(&directory);

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "Example Site");
        assert_eq!(entries[0].target, "https://example.com/a");
        assert_eq!(entries[0].kind, LauncherEntryKind::App);
    }

    #[test]
    fn the_walk_stops_at_the_depth_limit_and_skips_other_files() {
        let directory = temporary_directory("depth");
        let deep = directory.join("a").join("b").join("c");
        fs::create_dir_all(&deep).expect("create the deep folder");
        fs::write(directory.join("Top.url"), "URL=https://top.example").expect("write top");
        fs::write(directory.join("notes.txt"), "not a shortcut").expect("write notes");
        fs::write(deep.join("Deep.url"), "URL=https://deep.example").expect("write deep");

        let mut entries = Vec::new();
        let mut seen = HashSet::new();
        collect_shortcut_directory(&directory, 1, &mut entries, &mut seen);
        let names: Vec<_> = entries.iter().map(|entry| entry.name.as_str()).collect();
        let _ = fs::remove_dir_all(&directory);

        assert_eq!(names, ["Top"]);
    }

    #[test]
    fn uninstallers_are_not_offered() {
        assert!(is_skipped_name("Uninstall Widget"));
        assert!(is_skipped_name("卸载工具"));
        assert!(!is_skipped_name("Widget"));
        assert!(app_path_entry("Widget.exe", r"C:\does\not\exist.exe").is_none());
    }
}
