//! Per-song lyrics offsets, ported from the frontend's songOffsets.ts.
//!
//! A positive offset shows the lyrics later (the music page's "later" button) and a negative one
//! shows them earlier. The global offset stays in Settings; this file only keeps what each song
//! needed. Unlike the Tauri app's localStorage entry, NSH stores the map in
//! APPDATA/com.focusd.island.nsh/lyrics-offsets.json.

use std::{
    fs,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

use super::app_data_dir;

/// Older entries are dropped past this many songs.
const MAX_SONG_OFFSETS: usize = 500;
/// One press of the page's earlier / later buttons.
pub const SONG_OFFSET_STEP_MS: i64 = 500;
const LIMIT_MS: i64 = 10_000;

/// Clamped to +-10 s and rounded to 0.1 s, like the frontend's clampSongOffset.
pub fn clamp_song_offset(ms: i64) -> i64 {
    let rounded = if ms >= 0 { (ms + 50) / 100 * 100 } else { -((-ms + 50) / 100 * 100) };
    rounded.clamp(-LIMIT_MS, LIMIT_MS)
}

/// Identifies the song for its own offset: title, artist and duration to 2 seconds.
pub fn track_key(title: &str, artist: &str, duration_ms: i64) -> String {
    format!("{title}{artist}{}", (duration_ms + 1000) / 2000)
}

/// Offsets in the order they were added or changed, so the oldest can be dropped.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SongOffsets {
    entries: Vec<(String, i64)>,
}

impl SongOffsets {
    pub fn get(&self, key: &str) -> i64 {
        self.entries.iter().find(|(other, _)| other == key).map_or(0, |(_, ms)| *ms)
    }

    /// Sets key to ms (clamped); 0 removes the entry, and the oldest go past the limit.
    pub fn set(&mut self, key: &str, ms: i64) {
        self.entries.retain(|(other, _)| other != key);
        let value = clamp_song_offset(ms);
        if value != 0 {
            self.entries.push((key.to_string(), value));
            if self.entries.len() > MAX_SONG_OFFSETS {
                self.entries.drain(..self.entries.len() - MAX_SONG_OFFSETS);
            }
        }
    }

    /// Drops zero, unrounded and non-finite values from a file, as the frontend normalizes.
    fn normalized(mut self) -> Self {
        self.entries = self
            .entries
            .into_iter()
            .filter(|(_, ms)| *ms != 0)
            .map(|(key, ms)| (key, clamp_song_offset(ms)))
            .collect();
        if self.entries.len() > MAX_SONG_OFFSETS {
            self.entries.drain(..self.entries.len() - MAX_SONG_OFFSETS);
        }
        self
    }
}

/// APPDATA/com.focusd.island.nsh/lyrics-offsets.json.
pub fn song_offsets_path() -> Option<PathBuf> {
    Some(app_data_dir()?.join("lyrics-offsets.json"))
}

pub fn load() -> SongOffsets {
    song_offsets_path().and_then(|path| load_from(&path)).unwrap_or_default()
}

fn load_from(path: &Path) -> Option<SongOffsets> {
    let text = fs::read_to_string(path).ok()?;
    Some(serde_json::from_str::<SongOffsets>(&text).ok()?.normalized())
}

pub fn save(offsets: &SongOffsets) {
    if let Some(path) = song_offsets_path() {
        save_to(&path, offsets);
    }
}

fn save_to(path: &Path, offsets: &SongOffsets) {
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(text) = serde_json::to_string_pretty(offsets) {
        let _ = fs::write(path, text);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn offsets_are_clamped_rounded_and_removed_at_zero() {
        assert_eq!(clamp_song_offset(540), 500);
        assert_eq!(clamp_song_offset(-540), -500);
        assert_eq!(clamp_song_offset(-10_250), -10_000);
        assert_eq!(clamp_song_offset(10_250), 10_000);
        let mut offsets = SongOffsets::default();
        offsets.set("a", 480);
        assert_eq!(offsets.get("a"), 500);
        offsets.set("a", 0);
        assert_eq!(offsets.get("a"), 0);
        offsets.set("a", -480);
        assert_eq!(offsets.get("a"), -500);
    }

    #[test]
    fn the_oldest_offset_is_dropped_past_the_limit() {
        let mut offsets = SongOffsets::default();
        for index in 0..MAX_SONG_OFFSETS + 2 {
            offsets.set(&format!("song-{index}"), 500);
        }
        assert_eq!(offsets.entries.len(), MAX_SONG_OFFSETS);
        assert_eq!(offsets.get("song-0"), 0);
        assert_eq!(offsets.get(&format!("song-{}", MAX_SONG_OFFSETS + 1)), 500);
    }

    #[test]
    fn the_file_keeps_only_usable_offsets() {
        let path = std::env::temp_dir().join(format!("nsh-lyrics-offsets-{}.json", std::process::id()));
        fs::write(&path, r#"[["a", 240], ["b", 0], ["c", -99999]]"#).unwrap();
        let loaded = load_from(&path).unwrap();
        assert_eq!(loaded.get("a"), 200);
        assert_eq!(loaded.get("b"), 0);
        assert_eq!(loaded.get("c"), -10_000);
        save_to(&path, &loaded);
        assert_eq!(load_from(&path), Some(loaded));
        fs::remove_file(&path).unwrap();
        assert_eq!(track_key("A", "B", 123_000), "AB62");
    }
}