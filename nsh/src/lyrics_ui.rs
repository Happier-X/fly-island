//! The island's lyrics: a lookup that follows the playing track, the line shown in the collapsed
//! capsule, and the three-line view in the expanded page.
//!
//! The lookup runs once per track (and again when the lyrics settings change), not on every media
//! poll. The clock only moves the highlighted line, so a failed or missing source is a readable
//! state rather than a blank row.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use nsh_core::prelude::*;

use crate::lyrics::{
    line_index_at, load_song_offsets, parse_lrc, playback_position, save_song_offsets, track_key,
    with_translations, CacheState, LyricLine, LyricsQuery, LyricsResponse, SongOffsets,
    SONG_OFFSET_STEP_MS,
};
use crate::media::MediaState;
use crate::palette;

#[derive(Clone, Copy)]
pub struct LyricsState {
    pub response: Signal<Option<LyricsResponse>>,
    pub lines: Signal<Vec<LyricLine>>,
    pub loading: Signal<bool>,
    clock: Signal<i64>,
    offsets: Signal<SongOffsets>,
    signature: Signal<String>,
    reload: Signal<u64>,
    folder: Signal<String>,
    online: Signal<bool>,
    chinese: Signal<bool>,
}

fn now_ms() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |time| time.as_millis() as i64)
}

fn title_of(media: &MediaState) -> Option<&str> {
    media.title.as_deref().filter(|title| !title.trim().is_empty()).filter(|_| media.available)
}

pub fn start(
    media: Signal<Option<MediaState>>,
    folder: Signal<String>,
    online: Signal<bool>,
    chinese: Signal<bool>,
) -> LyricsState {
    let response = signal(None::<LyricsResponse>);
    let lines = signal(Vec::<LyricLine>::new());
    let loading = signal(false);
    let clock = signal(now_ms());
    let offsets = signal(load_song_offsets());
    let signature = signal(String::new());
    let reload = signal(0u64);

    set_interval(Duration::from_millis(250), move || clock.set(now_ms()));

    effect(move || {
        let parsed = response
            .get()
            .and_then(|answer| answer.lyrics)
            .map(|lyrics| {
                let main = parse_lrc(&lyrics.lrc);
                let translation = lyrics.translation.as_deref().map(parse_lrc).unwrap_or_default();
                with_translations(main, &translation)
            })
            .unwrap_or_default();
        lines.set(parsed);
    });

    effect(move || {
        let state = media.get();
        let folder_value = folder.get();
        let online_value = online.get();
        let chinese_value = chinese.get();
        let attempt = reload.get();
        let key = match state.as_ref() {
            Some(media) if title_of(media).is_some() => format!(
                "{}\u{1f}{}\u{1f}{}\u{1f}{}\u{1f}{}\u{1f}{}\u{1f}{}",
                media.title.as_deref().unwrap_or_default(),
                media.artist.as_deref().unwrap_or_default(),
                media.duration_ms / 2000,
                media.track_id.as_deref().unwrap_or_default(),
                folder_value,
                online_value,
                chinese_value
            ),
            _ => String::new(),
        };
        let next_signature = format!("{key}\u{1f}{attempt}");
        if next_signature == signature.get_untracked() {
            return;
        }
        signature.set(next_signature.clone());
        let Some(media) = state.filter(|media| title_of(media).is_some()) else {
            loading.set(false);
            response.set(None);
            return;
        };
        loading.set(true);
        response.set(None);
        spawn(async move {
            let answer = crate::lyrics::get_lyrics(LyricsQuery {
                title: media.title.clone().unwrap_or_default(),
                artist: media.artist.clone().unwrap_or_default(),
                album: media.album.clone().unwrap_or_default(),
                duration_ms: media.duration_ms,
                folder: folder_value.clone(),
                online: online_value,
                chinese: chinese_value,
                track_id: media.track_id.clone(),
            })
            .await;
            if signature.get_untracked() == next_signature {
                loading.set(false);
                response.set(Some(answer));
            }
        });
    });

    LyricsState { response, lines, loading, clock, offsets, signature, reload, folder, online, chinese }
}

fn current_offset(lyrics: &LyricsState, media: &MediaState) -> i64 {
    let Some(title) = title_of(media) else {
        return 0;
    };
    let key = track_key(title, media.artist.as_deref().unwrap_or_default(), media.duration_ms);
    lyrics.offsets.get().get(&key)
}

fn position_of(lyrics: &LyricsState, media: &MediaState) -> i64 {
    playback_position(media.position_ms, media.duration_ms, media.updated_at, media.playback_status, lyrics.clock.get())
        + current_offset(lyrics, media)
}

fn line_at(lyrics: &LyricsState, media: &MediaState) -> (Vec<LyricLine>, i64) {
    let lines = lyrics.lines.get();
    let index = line_index_at(&lines, position_of(lyrics, media));
    (lines, index)
}

fn song_title(media: Option<&MediaState>) -> String {
    let Some(media) = media else {
        return t!("island.title");
    };
    let title = media.title.clone().unwrap_or_else(|| t!("island.media_unknown"));
    let artist = media.artist.clone().unwrap_or_default();
    if artist.is_empty() {
        title
    } else {
        format!("{title} — {artist}")
    }
}

/// The collapsed capsule's one line: the lyric being sung, else the song, else the island title.
pub fn collapsed(lyrics: LyricsState, media: Signal<Option<MediaState>>, dark: Signal<bool>) -> impl IntoElement {
    text(move || {
        let Some(state) = media.get() else {
            return t!("island.title");
        };
        if title_of(&state).is_none() {
            return t!("island.title");
        }
        let (lines, index) = line_at(&lyrics, &state);
        lines
            .get(index as usize)
            .map(|line| line.text.trim().to_string())
            .filter(|text| !text.is_empty())
            .unwrap_or_else(|| song_title(Some(&state)))
    })
    .truncate()
    .flex_1()
    .text_size(px(13.0))
    .font_weight(FontWeight::MEDIUM)
    .text_color(move || palette(dark.get()).fg)
}

fn status_hint(lyrics: LyricsState, media: Signal<Option<MediaState>>) -> String {
    let state = media.get();
    let Some(media) = state.as_ref() else {
        return t!("lyrics_none");
    };
    if lyrics.loading.get() {
        return t!("lyrics_loading");
    }
    if let Some(answer) = lyrics.response.get() {
        if let Some(error) = answer.error.as_deref() {
            if error.contains("歌词目录") {
                return t!("lyrics_folder_missing");
            }
            return t!("lyrics_failed");
        }
        if answer.lyrics.is_some() {
            return t!("lyrics_no_lines");
        }
    }
    if !lyrics.online.get() && lyrics.folder.get().trim().is_empty() {
        return t!("lyrics_off");
    }
    let chinese_named = format!("{}{}", media.title.as_deref().unwrap_or_default(), media.artist.as_deref().unwrap_or_default())
        .chars()
        .any(|c| ('一'..='鿿').contains(&c));
    if chinese_named && !lyrics.chinese.get() {
        return t!("lyrics_chinese_off");
    }
    t!("lyrics_none")
}

fn detail_hint(lyrics: LyricsState, media: Signal<Option<MediaState>>) -> String {
    let state = media.get();
    let Some(media) = state.as_ref() else {
        return String::new();
    };
    if let Some(answer) = lyrics.response.get() {
        if let Some(error) = answer.error.as_deref() {
            if error.contains("歌词目录") {
                return t!("lyrics_folder_missing_hint");
            }
            return error.to_string();
        }
    }
    if !lyrics.online.get() && lyrics.folder.get().trim().is_empty() {
        return t!("lyrics_off_hint");
    }
    let chinese_named = format!("{}{}", media.title.as_deref().unwrap_or_default(), media.artist.as_deref().unwrap_or_default())
        .chars()
        .any(|c| ('一'..='鿿').contains(&c));
    if chinese_named && !lyrics.chinese.get() {
        return t!("lyrics_chinese_off_hint");
    }
    t!("lyrics_none_hint")
}

fn cache_note(answer: &LyricsResponse) -> Option<String> {
    match answer.cache {
        CacheState::Rejected => Some(t!("lyrics_cache_stale")),
        CacheState::Expired => Some(t!("lyrics_cache_expired")),
        CacheState::Failed => Some(t!("lyrics_cache_failed")),
        CacheState::NotUsed | CacheState::Miss | CacheState::Hit => None,
    }
}

fn set_offset(lyrics: LyricsState, media: Signal<Option<MediaState>>, delta: i64) {
    let Some(state) = media.get_untracked() else {
        return;
    };
    let Some(title) = title_of(&state) else {
        return;
    };
    let key = track_key(title, state.artist.as_deref().unwrap_or_default(), state.duration_ms);
    let mut offsets = lyrics.offsets.get_untracked();
    let next = if delta == i64::MIN { 0 } else { offsets.get(&key) + delta };
    offsets.set(&key, next);
    save_song_offsets(&offsets);
    lyrics.offsets.set(offsets);
}

fn retry(lyrics: LyricsState) {
    lyrics.signature.set(String::new());
    lyrics.reload.set(lyrics.reload.get_untracked() + 1);
}

fn small_button(label: impl Into<String> + 'static, dark: Signal<bool>, press: impl Fn() + 'static) -> impl IntoElement {
    let label = label.into();
    text(move || label.clone())
        .px(px(7.0))
        .py(px(2.0))
        .rounded(px(5.0))
        .bg(move || palette(dark.get()).input)
        .text_size(px(10.0))
        .text_color(move || palette(dark.get()).muted)
        .on_press(press)
}

/// The expanded page's lyrics: source and cache state, the line before, the line being sung,
/// its translation (else the next line), and this song's offset buttons.
pub fn panel(lyrics: LyricsState, media: Signal<Option<MediaState>>, dark: Signal<bool>) -> impl IntoElement {
    view()
        .flex_col()
        .gap(px(2.0))
        .px(px(14.0))
        .pb(px(4.0))
        .child(
            view()
                .flex_row()
                .items_center()
                .gap(px(6.0))
                .child(
                    text(move || t!("island.lyrics"))
                        .text_size(px(12.0))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(move || palette(dark.get()).fg),
                )
                .child(
                    text(move || lyrics.response.get().and_then(|answer| answer.lyrics).map(|lyrics| lyrics.source).unwrap_or(""))
                        .text_size(px(9.0))
                        .text_color(move || palette(dark.get()).faint),
                )
                .child(
                    text(move || lyrics.response.get().and_then(|answer| cache_note(&answer)).unwrap_or_default())
                        .text_size(px(9.0))
                        .text_color(move || palette(dark.get()).warning),
                )
                .child(view().flex_1())
                .child(when(move || !lyrics.lines.get().is_empty(), move || {
                    view()
                        .flex_row()
                        .items_center()
                        .gap(px(4.0))
                        .child(small_button("−0.5s", dark, move || set_offset(lyrics, media, -SONG_OFFSET_STEP_MS)))
                        .child(
                            text(move || {
                                let offset = media
                                    .get()
                                    .as_ref()
                                    .map_or(0, |state| current_offset(&lyrics, state));
                                format!("{:+.1}s", offset as f32 / 1000.0)
                            })
                            .text_size(px(9.0))
                            .text_color(move || palette(dark.get()).muted),
                        )
                        .child(small_button("+0.5s", dark, move || set_offset(lyrics, media, SONG_OFFSET_STEP_MS)))
                        .child(small_button("↺", dark, move || set_offset(lyrics, media, i64::MIN)))
                }))
                .child(when(move || lyrics.loading.get(), move || {
                    text("…").text_size(px(10.0)).text_color(move || palette(dark.get()).faint)
                })),
        )
        .child(when(move || !lyrics.lines.get().is_empty(), move || lyric_rows(lyrics, media, dark)))
        .child(when(
            move || lyrics.lines.get().is_empty(),
            move || {
                view()
                    .flex_col()
                    .gap(px(2.0))
                    .py(px(6.0))
                    .child(
                        text(move || status_hint(lyrics, media))
                            .text_size(px(12.0))
                            .text_color(move || palette(dark.get()).muted),
                    )
                    .child(
                        text(move || detail_hint(lyrics, media))
                            .truncate()
                            .text_size(px(9.5))
                            .text_color(move || palette(dark.get()).faint),
                    )
                    .child(when(move || !lyrics.loading.get(), move || {
                        small_button(t!("lyrics_retry"), dark, move || retry(lyrics))
                    }))
            },
        ))
}

fn lyric_rows(lyrics: LyricsState, media: Signal<Option<MediaState>>, dark: Signal<bool>) -> impl IntoElement {
    view()
        .flex_col()
        .gap(px(1.0))
        .child(
            text(move || {
                let Some(state) = media.get() else { return String::new() };
                let (lines, index) = line_at(&lyrics, &state);
                if index > 0 {
                    lines.get(index as usize - 1).map(|line| line.text.clone()).unwrap_or_default()
                } else {
                    String::new()
                }
            })
            .truncate()
            .h(px(16.0))
            .text_size(px(10.5))
            .text_color(move || palette(dark.get()).faint),
        )
        .child(
            text(move || {
                let Some(state) = media.get() else { return String::new() };
                let (lines, index) = line_at(&lyrics, &state);
                lines
                    .get(index as usize)
                    .map(|line| line.text.clone())
                    .filter(|text| !text.is_empty())
                    .unwrap_or_else(|| "♪".to_string())
            })
            .truncate()
            .h(px(22.0))
            .text_size(px(13.5))
            .font_weight(FontWeight::MEDIUM)
            .text_color(move || palette(dark.get()).fg),
        )
        .child(
            text(move || {
                let Some(state) = media.get() else { return String::new() };
                let (lines, index) = line_at(&lyrics, &state);
                let Some(line) = lines.get(index as usize) else { return String::new() };
                if let Some(translation) = line.translation.as_deref().filter(|text| !text.is_empty()) {
                    return translation.to_string();
                }
                lines.get(index as usize + 1).map(|next| next.text.clone()).unwrap_or_default()
            })
            .truncate()
            .h(px(16.0))
            .text_size(px(10.5))
            .text_color(move || palette(dark.get()).muted),
        )
}