//! Synced lyrics for the music page. Windows' media session gives the playing track's title,
//! artist, album and duration but no lyrics, so they are looked up: first in a folder of .lrc
//! files the user keeps (local.rs), then, only when the user turned online lyrics on, on LRCLIB
//! (lrclib.rs) and, when the user also turned Chinese sources on, on Kugou (kugou.rs) and
//! NetEase (netease.rs), which are unofficial. Online answers, and "not found", are cached in
//! lyrics-cache/ in the NSH data folder, so a track is asked for once. The text is returned as
//! LRC, with the translation as a second LRC when the source has one; the island parses both.

#![allow(dead_code)]

mod chinese;
mod found;
mod krc;
mod kugou;
mod local;
mod lrclib;
mod matching;
mod netease;
mod offset;
mod parse;

use std::{
    fs,
    path::{Path, PathBuf},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use found::FoundLyrics;
use matching::{cache_key, named_in_simplified, stable_hash, traditional_chars};

pub use offset::{load as load_song_offsets, save as save_song_offsets, track_key, SongOffsets, SONG_OFFSET_STEP_MS};
pub use parse::{line_index_at, parse_lrc, playback_position, with_translations, LyricLine};



const CACHE_DIR_NAME: &str = "lyrics-cache";
/// A track no source had lyrics for is asked about again after a week.
const MISSING_TTL: Duration = Duration::from_secs(7 * 24 * 60 * 60);
/// Past this many cached tracks, the oldest are removed.
const MAX_CACHED: usize = 2000;

/// The NSH data folder: APPDATA/com.focusd.island.nsh.
pub(super) fn app_data_dir() -> Option<PathBuf> {
    let base = std::env::var_os("APPDATA")?;
    Some(PathBuf::from(base).join("com.focusd.island.nsh"))
}

#[derive(Debug, Clone)]
pub struct LyricsQuery {
    pub title: String,
    pub artist: String,
    pub album: String,
    pub duration_ms: i64,
    /// The user's folder of .lrc files; empty when none is set.
    pub folder: String,
    /// Whether LRCLIB may be asked.
    pub online: bool,
    /// Whether Kugou and NetEase may be asked too (only with online).
    pub chinese: bool,
    /// The song id a player plugin gives, such as NCM-487885426 (MediaState::track_id).
    pub track_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lyrics {
    /// "local", "lrclib", "kugou" or "netease".
    pub source: &'static str,
    /// Synced lyrics in LRC format.
    pub lrc: String,
    /// The translation of the same lines, in LRC, when the source has one.
    pub translation: Option<String>,
    /// The song's length as the source lists it, when known.
    pub duration_ms: Option<i64>,
}

/// What the disk cache did for this lookup, so the page can say when it was stale or unwritable
/// instead of silently looking empty.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheState {
    /// No online lookup was made (local lyrics, online off, or an empty title).
    NotUsed,
    /// No cache file for this track yet.
    Miss,
    /// The answer came from the cache.
    Hit,
    /// The cached "not found" was older than a week, so the sources were asked again.
    Expired,
    /// A cache file was unusable (bad JSON, another track, or an older format) and was replaced.
    Rejected,
    /// The cache folder could not be written; lyrics still came from the network.
    Failed,
}

/// Lyrics for a track, plus why the answer is what it is. error is set only when no online
/// source could be reached (or the local folder could not be read); a successful answer wins.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LyricsResponse {
    pub lyrics: Option<Lyrics>,
    pub cache: CacheState,
    pub error: Option<String>,
}

impl LyricsResponse {
    fn none(cache: CacheState) -> Self {
        Self { lyrics: None, cache, error: None }
    }
}

/// One cached online answer: lyrics, or no lrc when no source had any. Entries written before
/// Chinese sources existed have neither source nor translation and came from LRCLIB.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct CacheEntry {
    key: String,
    lrc: Option<String>,
    #[serde(default)]
    source: Option<String>,
    #[serde(default)]
    translation: Option<String>,
    /// Written by a version that keeps Kugou's word times; older Kugou entries are fetched again.
    #[serde(default)]
    word_times: bool,
    /// The source's length for the song, when it listed one.
    #[serde(default)]
    duration_ms: Option<i64>,
    /// Written by a version that keeps the length; older entries with lyrics are fetched again.
    #[serde(default)]
    durations: bool,
    fetched_at: u64,
}

/// An online answer: the source and its lyrics.
type Answer = (&'static str, FoundLyrics);

#[derive(Debug, PartialEq, Eq)]
enum CacheLookup {
    Hit(Option<Answer>),
    Miss,
    Expired,
    Rejected,
}

fn source_name(name: &str) -> &'static str {
    match name {
        "kugou" => "kugou",
        "netease" => "netease",
        _ => "lrclib",
    }
}

fn now_secs() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |time| time.as_secs())
}

fn cache_path(dir: &Path, key: &str) -> PathBuf {
    dir.join(format!("{:016x}.json", stable_hash(key)))
}

/// The cached answer for key, whether it was usable, and why not.
fn read_cache(dir: &Path, key: &str, now: u64) -> CacheLookup {
    let Ok(text) = fs::read_to_string(cache_path(dir, key)) else {
        return CacheLookup::Miss;
    };
    let Ok(entry) = serde_json::from_str::<CacheEntry>(&text) else {
        return CacheLookup::Rejected;
    };
    if entry.key != key {
        return CacheLookup::Rejected;
    }
    match entry.lrc {
        Some(_) if entry.source.as_deref() == Some("kugou") && !entry.word_times => CacheLookup::Rejected,
        Some(_) if !entry.durations => CacheLookup::Rejected,
        Some(lrc) => {
            let source = source_name(entry.source.as_deref().unwrap_or_default());
            CacheLookup::Hit(Some((source, FoundLyrics { lrc, translation: entry.translation, duration_ms: entry.duration_ms })))
        }
        None if now.saturating_sub(entry.fetched_at) < MISSING_TTL.as_secs() => CacheLookup::Hit(None),
        None => CacheLookup::Expired,
    }
}

/// Writes one answer; false when the cache folder could not be used.
fn write_cache(dir: &Path, key: &str, answer: Option<&Answer>, now: u64) -> bool {
    if fs::create_dir_all(dir).is_err() {
        return false;
    }
    let entry = CacheEntry {
        key: key.to_string(),
        lrc: answer.map(|(_, found)| found.lrc.clone()),
        source: answer.map(|(source, _)| (*source).to_string()),
        translation: answer.and_then(|(_, found)| found.translation.clone()),
        word_times: true,
        duration_ms: answer.and_then(|(_, found)| found.duration_ms),
        durations: true,
        fetched_at: now,
    };
    let Ok(text) = serde_json::to_string(&entry) else {
        return false;
    };
    if fs::write(cache_path(dir, key), text).is_err() {
        return false;
    }
    prune_cache(dir, MAX_CACHED);
    true
}

/// Removes the oldest cache files until at most keep are left.
fn prune_cache(dir: &Path, keep: usize) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    let mut files: Vec<(SystemTime, PathBuf)> = entries
        .flatten()
        .filter_map(|entry| Some((entry.metadata().ok()?.modified().ok()?, entry.path())))
        .collect();
    if files.len() <= keep {
        return;
    }
    files.sort();
    for (_, path) in &files[..files.len() - keep] {
        let _ = fs::remove_file(path);
    }
}

/// Lyrics as shown: a track named in simplified Chinese gets simplified lyrics and translation,
/// since many Chinese lyrics on LRCLIB were uploaded in traditional characters.
fn for_display(source: &'static str, found: FoundLyrics, title: &str, artist: &str) -> Lyrics {
    let simplify = |text: String| {
        if named_in_simplified(title, artist) && traditional_chars(&text) > 2 {
            chinese::to_simplified(&text).unwrap_or(text)
        } else {
            text
        }
    };
    Lyrics { source, lrc: simplify(found.lrc), translation: found.translation.map(simplify), duration_ms: found.duration_ms }
}

/// The online sources to ask, in order. A song named in Chinese asks Kugou and NetEase first,
/// as LRCLIB has few Chinese songs; a NetEase song the player named asks NetEase first.
fn source_order(query: &LyricsQuery) -> Vec<&'static str> {
    if !query.chinese {
        return vec!["lrclib"];
    }
    let chinese_named = format!("{}{}", query.title, query.artist).chars().any(|c| ('一'..='鿿').contains(&c));
    match (netease::track_id(query.track_id.as_deref()).is_some(), chinese_named) {
        (true, _) => vec!["netease", "kugou", "lrclib"],
        (false, true) => vec!["kugou", "netease", "lrclib"],
        (false, false) => vec!["lrclib", "kugou", "netease"],
    }
}

/// The first source that has lyrics. Err only when every source asked failed to answer.
async fn fetch_online(query: &LyricsQuery) -> Result<Option<Answer>, String> {
    let (title, artist) = (query.title.as_str(), query.artist.as_str());
    let mut errors = Vec::new();
    let mut answered = false;
    for source in source_order(query) {
        let result = tokio::time::timeout(Duration::from_secs(12), async {
            match source {
                "kugou" => kugou::fetch(title, artist, query.duration_ms).await,
                "netease" => netease::fetch(title, artist, query.duration_ms, query.track_id.as_deref()).await,
                _ => lrclib::fetch(title, artist, &query.album, query.duration_ms).await,
            }
        })
        .await
        .unwrap_or_else(|_| Err(format!("{source} did not answer in time")));
        match result {
            Ok(Some(found)) => return Ok(Some((source, found))),
            Ok(None) => answered = true,
            Err(error) => errors.push(error),
        }
    }
    if answered {
        Ok(None)
    } else {
        Err(errors.join("; "))
    }
}

/// Synced lyrics for the playing track. Errors only when no source could be reached; that answer
/// is not cached, so the next track change asks again.
pub async fn get_lyrics(query: LyricsQuery) -> LyricsResponse {
    let cache_dir = app_data_dir().map(|dir| dir.join(CACHE_DIR_NAME));
    get_lyrics_with_cache(query, cache_dir.as_deref()).await
}

async fn get_lyrics_with_cache(query: LyricsQuery, cache_dir: Option<&Path>) -> LyricsResponse {
    if query.title.trim().is_empty() {
        return LyricsResponse::none(CacheState::NotUsed);
    }
    let mut local_error = None;
    let folder = query.folder.trim();
    if !folder.is_empty() {
        let path = Path::new(folder);
        if !path.is_dir() {
            local_error = Some("本地歌词目录不存在或无法读取".to_string());
        } else if let Some(lrc) = local::find(path, &query.title, &query.artist) {
            return LyricsResponse {
                lyrics: Some(for_display("local", FoundLyrics { lrc, translation: None, duration_ms: None }, &query.title, &query.artist)),
                cache: CacheState::NotUsed,
                error: None,
            };
        }
    }
    if !query.online {
        return LyricsResponse { lyrics: None, cache: CacheState::NotUsed, error: local_error };
    }
    // A miss with LRCLIB alone says nothing about the Chinese sources.
    let mut key = cache_key(&query.title, &query.artist, query.duration_ms);
    if query.chinese {
        key.push_str("\u{1f}cn");
    }
    let lookup = cache_dir.map_or(CacheLookup::Miss, |dir| read_cache(dir, &key, now_secs()));
    if let CacheLookup::Hit(answer) = lookup {
        return LyricsResponse {
            lyrics: answer.map(|(source, found)| for_display(source, found, &query.title, &query.artist)),
            cache: CacheState::Hit,
            error: local_error,
        };
    }
    let fetched = fetch_online(&query).await;
    match fetched {
        Err(error) => LyricsResponse { lyrics: None, cache: cache_state(&lookup), error: Some(error) },
        Ok(answer) => {
            let cache = match cache_dir {
                Some(dir) if write_cache(dir, &key, answer.as_ref(), now_secs()) => cache_state(&lookup),
                _ => CacheState::Failed,
            };
            LyricsResponse {
                lyrics: answer.map(|(source, found)| for_display(source, found, &query.title, &query.artist)),
                cache,
                error: local_error,
            }
        }
    }
}

fn cache_state(lookup: &CacheLookup) -> CacheState {
    match lookup {
        CacheLookup::Hit(_) => CacheState::Hit,
        CacheLookup::Miss => CacheState::Miss,
        CacheLookup::Expired => CacheState::Expired,
        CacheLookup::Rejected => CacheState::Rejected,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn found(lrc: &str, translation: Option<&str>) -> FoundLyrics {
        FoundLyrics { lrc: lrc.into(), translation: translation.map(Into::into), duration_ms: None }
    }

    #[test]
    fn the_cache_keeps_lyrics_with_their_source_and_recent_misses() {
        let dir = std::env::temp_dir().join(format!("nsh-lyrics-cache-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let now = 1_800_000_000;
        assert_eq!(read_cache(&dir, "a", now), CacheLookup::Miss);
        assert!(write_cache(&dir, "a", Some(&("kugou", found("[00:01.00]x", Some("[00:01.00]译")))), now));
        assert!(write_cache(&dir, "b", None, now));
        assert_eq!(
            read_cache(&dir, "a", now + 999_999_999),
            CacheLookup::Hit(Some(("kugou", found("[00:01.00]x", Some("[00:01.00]译")))))
        );
        assert_eq!(read_cache(&dir, "b", now + 60), CacheLookup::Hit(None));
        // A miss is asked about again after a week.
        assert_eq!(read_cache(&dir, "b", now + MISSING_TTL.as_secs()), CacheLookup::Expired);
        // Entries without a source came from LRCLIB.
        fs::write(cache_path(&dir, "old"), r#"{"key":"old","lrc":"[00:01.00]y","durations":true,"fetchedAt":1}"#).unwrap();
        assert_eq!(read_cache(&dir, "old", now), CacheLookup::Hit(Some(("lrclib", found("[00:01.00]y", None)))));
        // The song's length is kept; lyrics cached before lengths were are fetched again.
        let timed = FoundLyrics { duration_ms: Some(246_000), ..found("[00:01.00]t", None) };
        assert!(write_cache(&dir, "t", Some(&("netease", timed.clone())), now));
        assert_eq!(read_cache(&dir, "t", now), CacheLookup::Hit(Some(("netease", timed))));
        fs::write(cache_path(&dir, "untimed"), r#"{"key":"untimed","lrc":"[00:01.00]u","source":"netease","fetchedAt":1}"#).unwrap();
        assert_eq!(read_cache(&dir, "untimed", now), CacheLookup::Rejected);
        // Kugou lyrics cached before word times were kept are fetched again.
        fs::write(cache_path(&dir, "k"), r#"{"key":"k","lrc":"[00:01.00]z","source":"kugou","fetchedAt":1}"#).unwrap();
        assert_eq!(read_cache(&dir, "k", now), CacheLookup::Rejected);
        // A damaged file is replaced rather than shown.
        fs::write(cache_path(&dir, "bad"), "not json").unwrap();
        assert_eq!(read_cache(&dir, "bad", now), CacheLookup::Rejected);
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn traditional_lyrics_are_shown_simplified_for_a_track_named_in_simplified() {
        let lrc = "[00:31.50] 劃一根火柴 將慵倦的夜點亮";
        let shown = for_display("lrclib", found(lrc, Some("[00:31.50] 劃一根火柴 將慵倦的夜點亮")), "余香", "张小九");
        assert_eq!(shown.lrc, "[00:31.50] 划一根火柴 将慵倦的夜点亮");
        assert_eq!(shown.translation.as_deref(), Some("[00:31.50] 划一根火柴 将慵倦的夜点亮"));
        assert_eq!(for_display("lrclib", found(lrc, None), "餘香", "張小九").lrc, lrc);
    }

    #[test]
    fn chinese_songs_ask_the_chinese_sources_first() {
        let query = |title: &str, chinese: bool, track_id: Option<&str>| LyricsQuery {
            title: title.into(),
            artist: String::new(),
            album: String::new(),
            duration_ms: 0,
            folder: String::new(),
            online: true,
            chinese,
            track_id: track_id.map(Into::into),
        };
        assert_eq!(source_order(&query("余香", false, None)), ["lrclib"]);
        assert_eq!(source_order(&query("余香", true, None)), ["kugou", "netease", "lrclib"]);
        assert_eq!(source_order(&query("Yesterday", true, None)), ["lrclib", "kugou", "netease"]);
        assert_eq!(source_order(&query("Yesterday", true, Some("NCM-1"))), ["netease", "kugou", "lrclib"]);
    }

    #[test]
    fn pruning_keeps_the_newest_files() {
        let dir = std::env::temp_dir().join(format!("nsh-lyrics-prune-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        for name in ["1", "2", "3"] {
            fs::write(dir.join(name), name).unwrap();
            std::thread::sleep(Duration::from_millis(20));
        }
        prune_cache(&dir, 2);
        let mut left: Vec<String> = fs::read_dir(&dir).unwrap().flatten().map(|e| e.file_name().to_string_lossy().into_owned()).collect();
        left.sort();
        assert_eq!(left, ["2", "3"]);
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_local_file_wins_and_a_missing_folder_is_reported() {
        let root = std::env::temp_dir().join(format!("nsh-lyrics-local-{}", std::process::id()));
        let folder = root.join("lrc");
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&folder).unwrap();
        fs::write(folder.join("周杰伦 - 晴天.lrc"), "[00:01.00]故事的小黄花").unwrap();
        let query = |path: &Path| LyricsQuery {
            title: "晴天".into(),
            artist: "周杰伦".into(),
            album: String::new(),
            duration_ms: 0,
            folder: path.to_string_lossy().into_owned(),
            online: false,
            chinese: true,
            track_id: None,
        };
        let answer = pollster::block_on(get_lyrics_with_cache(query(&folder), None));
        assert_eq!(answer.lyrics.as_ref().map(|lyrics| lyrics.source), Some("local"));
        assert_eq!(answer.lyrics.as_ref().map(|lyrics| lyrics.lrc.as_str()), Some("[00:01.00]故事的小黄花"));
        assert_eq!(answer.cache, CacheState::NotUsed);
        let missing = pollster::block_on(get_lyrics_with_cache(query(&root.join("missing")), None));
        assert_eq!(missing.lyrics, None);
        assert!(missing.error.as_deref().is_some_and(|error| error.contains("歌词目录")));
        fs::remove_dir_all(&root).unwrap();
    }
    fn network_query(title: &str, artist: &str, duration_ms: i64, online: bool, chinese: bool) -> LyricsQuery {
        LyricsQuery {
            title: title.into(),
            artist: artist.into(),
            album: String::new(),
            duration_ms,
            folder: String::new(),
            online,
            chinese,
            track_id: None,
        }
    }

    #[tokio::test]
    #[ignore = "network: asks the live LRCLIB"]
    async fn live_lrclib_finds_yesterday() {
        let found = lrclib::fetch("Yesterday", "The Beatles", "Help!", 125_000).await.unwrap().unwrap();
        assert!(found.lrc.to_lowercase().contains("yesterday"));
    }

    #[tokio::test]
    #[ignore = "network: asks live Kugou and NetEase"]
    async fn live_chinese_sources_find_yu_xiang() {
        let kugou = kugou::fetch("余香", "张小九", 246_000).await.unwrap().unwrap();
        assert!(kugou.lrc.contains("划") && kugou.lrc.contains("<00:"));
        let netease = netease::fetch("余香", "张小九", 246_000, None).await.unwrap().unwrap();
        assert!(!netease.lrc.trim().is_empty());
    }

    #[tokio::test]
    #[ignore = "network: asks the live Kugou for KRC with a translation"]
    async fn live_kugou_returns_krc_translation() {
        let found = kugou::fetch("Yesterday", "The Beatles", 125_000).await.unwrap().unwrap();
        assert!(found.translation.as_deref().is_some_and(|text| !text.trim().is_empty()));
    }

    #[tokio::test]
    #[ignore = "network: checks that an unreachable proxy reports an error"]
    async fn live_an_unreachable_proxy_is_an_error() {
        // SAFETY: this ignored test runs on its own and restores both variables before it ends.
        unsafe {
            std::env::set_var("HTTPS_PROXY", "http://127.0.0.1:9");
            std::env::set_var("HTTP_PROXY", "http://127.0.0.1:9");
        }
        let result = fetch_online(&network_query("Yesterday", "The Beatles", 125_000, true, true)).await;
        // SAFETY: as above; no other thread reads these variables in this test.
        unsafe {
            std::env::remove_var("HTTPS_PROXY");
            std::env::remove_var("HTTP_PROXY");
        }
        assert!(result.is_err(), "a refused proxy should be reported as a failure");
    }

}