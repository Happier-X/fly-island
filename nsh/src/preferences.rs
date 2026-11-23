//! The small settings store the Phase 2 island writes: theme and language.

use std::{fs, path::PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Preferences {
    pub dark: bool,
    pub locale: String,
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            dark: true,
            locale: String::new(),
        }
    }
}

fn data_file() -> Option<PathBuf> {
    let base = std::env::var_os("APPDATA")?;
    Some(
        PathBuf::from(base)
            .join("com.focusd.island.nsh")
            .join("settings.json"),
    )
}

pub fn load() -> Preferences {
    let Some(path) = data_file() else {
        return Preferences::default();
    };
    fs::read_to_string(path)
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

pub fn save(preferences: &Preferences) {
    let Some(path) = data_file() else {
        return;
    };
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(json) = serde_json::to_string_pretty(preferences) {
        let _ = fs::write(path, json);
    }
}
