//! Start Menu shortcuts, AppX applications and executables on PATH.

use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
};

use super::{EntryKind, LauncherEntry};

const START_MENU_DEPTH: usize = 4;
const MAX_APPS: usize = 4_000;

pub fn enumerate() -> Vec<LauncherEntry> {
    let mut entries = Vec::new();
    let mut seen = HashSet::new();
    for directory in start_menu_directories() {
        walk(&directory, START_MENU_DEPTH, &mut entries, &mut seen);
    }
    collect_uwp(&mut entries, &mut seen);
    collect_path(&mut entries, &mut seen);
    entries.sort_by_key(|entry| entry.name.to_lowercase());
    entries.truncate(MAX_APPS);
    entries
}

fn new_entry(name: String, subtitle: String, target: String) -> LauncherEntry {
    LauncherEntry {
        id: format!("app:{}", target.to_lowercase()),
        name,
        subtitle,
        target,
        kind: EntryKind::App,
    }
}

fn start_menu_directories() -> Vec<PathBuf> {
    let mut directories = Vec::new();
    if let Some(app_data) = std::env::var_os("APPDATA") {
        directories.push(PathBuf::from(app_data).join(r"Microsoft\Windows\Start Menu\Programs"));
    }
    if let Some(program_data) = std::env::var_os("PROGRAMDATA") {
        directories.push(PathBuf::from(program_data).join(r"Microsoft\Windows\Start Menu\Programs"));
    }
    directories.retain(|directory| directory.is_dir());
    directories
}

fn walk(directory: &Path, depth: usize, entries: &mut Vec<LauncherEntry>, seen: &mut HashSet<String>) {
    let Ok(read) = fs::read_dir(&directory) else {
        return;
    };
    for item in read.flatten() {
        let path = item.path();
        let Ok(kind) = item.file_type() else {
            continue;
        };
        if kind.is_dir() {
            if depth > 0 {
                walk(&path, depth - 1, entries, seen);
            }
            continue;
        }
        let Some(entry) = shortcut_entry(&path) else {
            continue;
        };
        if seen.insert(entry.target.to_lowercase()) {
            entries.push(entry);
        }
    }
}
fn shortcut_entry(path: &Path) -> Option<LauncherEntry> {
    let extension = path.extension()?.to_string_lossy().to_lowercase();
    let name = path.file_stem()?.to_string_lossy().trim().to_string();
    if name.is_empty() || skipped(&name) {
        return None;
    }
    let full = path.to_string_lossy().into_owned();
    match extension.as_str() {
        "lnk" => Some(new_entry(name, full.clone(), full)),
        "appref-ms" => Some(new_entry(name, full.clone(), full)),
        "url" => {
            let url = url_target(path)?;
            Some(new_entry(name, url.clone(), url))
        }
        _ => None,
    }
}

fn url_target(path: &Path) -> Option<String> {
    let text = fs::read_to_string(path).ok()?;
    text.lines().find_map(|line| {
        let (key, value) = line.split_once('=')?;
        key.trim()
            .eq_ignore_ascii_case("url")
            .then(|| value.trim().to_string())
            .filter(|url| !url.is_empty())
    })
}

fn skipped(name: &str) -> bool {
    let lower = name.to_lowercase();
    ["uninstall", "readme", "help", "documentation", "website", "release notes"]
        .iter()
        .any(|word| lower.contains(word))
}

fn collect_uwp(entries: &mut Vec<LauncherEntry>, seen: &mut HashSet<String>) {
    let output = std::process::Command::new("powershell.exe")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            "Get-StartApps | Select-Object Name,AppID | ConvertTo-Json -Compress",
        ])
        .output();
    let Ok(output) = output else {
        return;
    };
    if !output.status.success() {
        return;
    }
    let Ok(value) = serde_json::from_slice::<serde_json::Value>(&output.stdout) else {
        return;
    };
    let list = value.as_array().cloned().unwrap_or_else(|| vec![value]);
    let known: HashSet<String> = entries.iter().map(|entry| entry.name.to_lowercase()).collect();
    for item in list {
        let Some(name) = item["Name"].as_str().or_else(|| item["name"].as_str()) else {
            continue;
        };
        let Some(app_id) = item["AppID"].as_str().or_else(|| item["appID"].as_str()) else {
            continue;
        };
        let name = name.trim();
        if name.is_empty() || known.contains(&name.to_lowercase()) {
            continue;
        }
        let target = format!("shell:AppsFolder\\{app_id}");
        if seen.insert(target.to_lowercase()) {
            entries.push(new_entry(name.to_string(), "UWP".to_string(), target));
        }
    }
}
fn collect_path(entries: &mut Vec<LauncherEntry>, seen: &mut HashSet<String>) {
    let Some(path) = std::env::var_os("PATH") else {
        return;
    };
    // Start Menu and AppX names win over executables on PATH with the same name:
    // Notepad, for instance, is listed by both and should appear once.
    let mut known: HashSet<String> = entries.iter().map(|entry| entry.name.to_lowercase()).collect();
    let mut added = 0usize;
    for directory in std::env::split_paths(&path) {
        if added >= 1_500 || !directory.is_dir() {
            continue;
        }
        let Ok(read) = fs::read_dir(&directory) else {
            continue;
        };
        for item in read.flatten() {
            if added >= 1_500 {
                break;
            }
            let file = item.path();
            if !file.extension().is_some_and(|extension| extension.eq_ignore_ascii_case("exe")) {
                continue;
            }
            let Some(name) = file.file_stem().map(|stem| stem.to_string_lossy().trim().to_string()) else {
                continue;
            };
            if name.is_empty() || skipped(&name) {
                continue;
            }
            let target = file.to_string_lossy().into_owned();
            if seen.insert(target.to_lowercase()) && known.insert(name.to_lowercase()) {
                entries.push(new_entry(name, directory.to_string_lossy().into_owned(), target));
                added += 1;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entries_have_stable_ids_and_skip_help_shortcuts() {
        let entry = new_entry("Code".into(), "C:\\Code.exe".into(), "C:\\Code.exe".into());
        assert_eq!(entry.id, "app:c:\\code.exe");
        assert!(skipped("Uninstall Foo"));
        assert!(!skipped("Visual Studio Code"));
    }
}