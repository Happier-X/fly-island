//! LRC text to timed lines for the island, ported from the frontend's lrc.ts.
//!
//! A line may carry several time tags ("[00:12.00][01:30.00]chorus"), enhanced-LRC word times
//! ("<00:17.51>Oh <00:17.80>I") and a "[offset:+500]" header that shifts every line earlier.
//! Translations are a second LRC whose lines start at the same time (within 50 ms).

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LyricWord {
    pub start_ms: i64,
    pub end_ms: i64,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LyricLine {
    pub time_ms: i64,
    pub text: String,
    pub translation: Option<String>,
    pub words: Vec<LyricWord>,
}

/// Milliseconds of a time tag; fraction digits are tenths, hundredths or milliseconds.
fn tag_ms(minutes: &str, seconds: &str, fraction: &str) -> Option<i64> {
    let minutes: i64 = minutes.parse().ok()?;
    let seconds: i64 = seconds.parse().ok()?;
    let fraction = if fraction.is_empty() { 0 } else {
        let digits: i64 = fraction.parse().ok()?;
        match fraction.len() {
            1 => digits * 100,
            2 => digits * 10,
            _ => digits,
        }
    };
    Some(minutes * 60_000 + seconds * 1000 + fraction)
}

/// The time a tag body such as "00:17.51" names.
fn time_of(body: &str) -> Option<i64> {
    let (minutes, rest) = body.split_once(':')?;
    let (seconds, fraction) = rest.split_once(['.', ':']).unwrap_or((rest, ""));
    if minutes.is_empty() || seconds.is_empty() || minutes.len() > 3 || seconds.len() > 2 || fraction.len() > 3 {
        return None;
    }
    if !minutes.bytes().all(|byte| byte.is_ascii_digit())
        || !seconds.bytes().all(|byte| byte.is_ascii_digit())
        || !fraction.bytes().all(|byte| byte.is_ascii_digit())
    {
        return None;
    }
    tag_ms(minutes, seconds, fraction)
}

/// Parses LRC text into lines sorted by time. Untimed lines are dropped, and so are lines that
/// are only tags; blank sung lines stay as breaks between verses.
pub fn parse_lrc(text: &str) -> Vec<LyricLine> {
    let mut offset_ms = 0i64;
    let mut lines = Vec::new();
    for raw in text.lines() {
        let trimmed = raw.trim();
        if let Some(rest) = trimmed.strip_prefix('[') {
            if let Some((body, _)) = rest.split_once(']') {
                if body.len() > 7 && body[..7].eq_ignore_ascii_case("offset:") {
                    offset_ms = body[7..].trim().parse().unwrap_or(0);
                    continue;
                }
            }
        }
        let (times, rest) = split_time_tags(raw);
        if times.is_empty() {
            continue;
        }
        let (line_text, words) = words_of(&rest, times[0]);
        for time in &times {
            let shift = time - times[0];
            let moved = words.iter().map(|word| LyricWord { start_ms: word.start_ms + shift, end_ms: word.end_ms + shift, text: word.text.clone() }).collect();
            lines.push(LyricLine { time_ms: *time, text: line_text.clone(), translation: None, words: if words.is_empty() { Vec::new() } else { moved } });
        }
    }
    let early = |ms: i64| (ms - offset_ms).max(0);
    for line in &mut lines {
        line.time_ms = early(line.time_ms);
        for word in &mut line.words {
            word.start_ms = early(word.start_ms);
            word.end_ms = early(word.end_ms);
        }
    }
    lines.sort_by_key(|line| line.time_ms);
    lines
}

/// The time tags in a line, and the line without them.
fn split_time_tags(line: &str) -> (Vec<i64>, String) {
    let mut times = Vec::new();
    let mut rest = String::new();
    let mut chars = line.char_indices().peekable();
    while let Some((start, ch)) = chars.next() {
        if ch == '[' {
            if let Some(end) = line[start + 1..].find(']').map(|end| start + 1 + end) {
                let body = &line[start + 1..end];
                if let Some(ms) = time_of(body) {
                    times.push(ms);
                    while let Some((next, _)) = chars.peek() {
                        if *next <= end {
                            chars.next();
                        } else {
                            break;
                        }
                    }
                    continue;
                }
            }
        }
        rest.push(ch);
    }
    (times, rest)
}

/// A line's text with enhanced-LRC word times, and the words when it has any.
fn words_of(text: &str, line_ms: i64) -> (String, Vec<LyricWord>) {
    let mut marks = Vec::new();
    let mut rest = text;
    let mut offset = 0usize;
    while let Some(open) = rest.find('<') {
        let Some(close) = rest[open..].find('>').map(|end| open + end) else {
            break;
        };
        let body = &rest[open + 1..close];
        if let Some(ms) = time_of(body) {
            marks.push((offset + open, ms, offset + close + 1));
        }
        offset += close + 1;
        rest = &rest[close + 1..];
    }
    if marks.is_empty() {
        return (text.trim().to_string(), Vec::new());
    }
    let mut words = Vec::new();
    let lead = text[..marks[0].0].trim();
    if !lead.is_empty() {
        words.push(LyricWord { start_ms: line_ms, end_ms: marks[0].1, text: lead.to_string() });
    }
    for (index, (_, ms, end)) in marks.iter().enumerate() {
        let next = marks.get(index + 1).map(|mark| mark.1);
        let next_at = marks.get(index + 1).map(|mark| mark.0).unwrap_or(text.len());
        let word = &text[*end..next_at];
        if !word.is_empty() {
            words.push(LyricWord { start_ms: *ms, end_ms: next.unwrap_or(*ms), text: word.to_string() });
        }
    }
    if let Some(first) = words.first_mut() {
        first.text = first.text.trim_start().to_string();
    }
    (words.iter().map(|word| word.text.as_str()).collect::<String>().trim().to_string(), words)
}

/// Index of the line being sung at position_ms, or -1 before the first line.
pub fn line_index_at(lines: &[LyricLine], position_ms: i64) -> i64 {
    let mut low = 0usize;
    let mut high = lines.len() as i64 - 1;
    let mut found = -1i64;
    while (low as i64) <= high {
        let middle = ((low as i64 + high) / 2) as usize;
        if lines[middle].time_ms <= position_ms {
            found = middle as i64;
            low = middle + 1;
        } else {
            high = middle as i64 - 1;
        }
    }
    found
}

/// Lines with the translation line that starts at the same time (within 50 ms) attached.
pub fn with_translations(mut lines: Vec<LyricLine>, translation: &[LyricLine]) -> Vec<LyricLine> {
    if translation.is_empty() {
        return lines;
    }
    for line in &mut lines {
        line.translation = translation
            .iter()
            .find(|other| (other.time_ms - line.time_ms).abs() < 50 && !other.text.is_empty())
            .map(|other| other.text.clone())
            .filter(|_| !line.text.is_empty());
    }
    lines
}

/// Where playback is now: the position Windows gave at updated_at, moved on while playing.
pub fn playback_position(position_ms: i64, duration_ms: i64, updated_at: i64, playback_status: &str, now: i64) -> i64 {
    let moved = if playback_status == "playing" { (now - updated_at).max(0) } else { 0 };
    let position = position_ms + moved;
    if duration_ms > 0 { position.min(duration_ms) } else { position }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn several_time_tags_repeat_a_line_and_word_times_move_with_it() {
        let lines = parse_lrc("[00:12.00][01:30.00]<00:12.00>chorus
[00:31.50] 劃一根火柴
no tag");
        assert_eq!(lines.len(), 3);
        assert_eq!((lines[0].time_ms, lines[0].text.as_str()), (12_000, "chorus"));
        assert_eq!((lines[1].time_ms, lines[1].text.as_str()), (31_500, "劃一根火柴"));
        assert_eq!((lines[2].time_ms, lines[2].text.as_str()), (90_000, "chorus"));
        assert_eq!(lines[2].words[0].start_ms, 90_000);
    }

    #[test]
    fn an_offset_header_moves_every_line_earlier_but_not_before_zero() {
        let lines = parse_lrc("[offset:+500]
[00:00.20]a
[00:10.00]b");
        assert_eq!(lines.iter().map(|line| line.time_ms).collect::<Vec<_>>(), [0, 9_500]);
    }

    #[test]
    fn word_times_split_a_line_and_keep_their_text() {
        let lines = parse_lrc("[00:17.51]<00:17.51>Oh <00:17.80>I <00:18.21>believe");
        assert_eq!(lines[0].text, "Oh I believe");
        assert_eq!(lines[0].words.len(), 3);
        assert_eq!(lines[0].words[1], LyricWord { start_ms: 17_800, end_ms: 18_210, text: "I ".into() });
    }

    #[test]
    fn translations_attach_by_time_and_the_line_before_is_found() {
        let lines = with_translations(parse_lrc("[00:10.00]Oh I believe
[00:22.00]in yesterday"), &parse_lrc("[00:10.01]哦 我相信
"));
        assert_eq!(lines[0].translation.as_deref(), Some("哦 我相信"));
        assert_eq!(lines[1].translation, None);
        assert_eq!(line_index_at(&lines, 9_000), -1);
        assert_eq!(line_index_at(&lines, 10_000), 0);
        assert_eq!(line_index_at(&lines, 30_000), 1);
    }

    #[test]
    fn playback_position_moves_only_while_playing() {
        assert_eq!(playback_position(10_000, 30_000, 1_000, "playing", 3_000), 12_000);
        assert_eq!(playback_position(10_000, 30_000, 1_000, "paused", 3_000), 10_000);
        assert_eq!(playback_position(29_000, 30_000, 1_000, "playing", 5_000), 30_000);
    }
}