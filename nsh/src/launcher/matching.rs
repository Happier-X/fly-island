//! Ranking for the quick launcher; ported from the frontend's matching.ts.

use super::{EntryKind, LauncherEntry, LauncherResult, MatchKind, ResultKind, SearchEngine};

const DEFAULT_LIMIT: usize = 30;
const MAX_LIMIT: usize = 100;

pub(super) fn search(
    entries: &[LauncherEntry],
    query: &str,
    limit: usize,
    search_web: bool,
    engine: SearchEngine,
) -> Vec<LauncherResult> {
    let query = query.trim();
    let normalized = query.to_lowercase();
    if normalized.is_empty() {
        return Vec::new();
    }
    let limit = limit.clamp(1, MAX_LIMIT).min(DEFAULT_LIMIT.max(1));
    let mut results = Vec::new();
    if is_url(query) {
        results.push(LauncherResult {
            id: "url".to_string(),
            name: query.to_string(),
            subtitle: "打开网址".to_string(),
            target: query.to_string(),
            kind: ResultKind::Web,
            match_kind: MatchKind::Web,
        });
    }
    let mut matched = Vec::new();
    for entry in entries {
        let matched_kind = name_match(&entry.name, &normalized).or_else(|| {
            (entry.kind == EntryKind::File && entry.subtitle.to_lowercase().contains(&normalized))
                .then_some(MatchKind::Path)
        });
        if let Some(match_kind) = matched_kind {
            matched.push(LauncherResult {
                id: entry.id.clone(),
                name: entry.name.clone(),
                subtitle: entry.subtitle.clone(),
                target: entry.target.clone(),
                kind: match entry.kind {
                    EntryKind::App => ResultKind::App,
                    EntryKind::File => ResultKind::File,
                },
                match_kind,
            });
        }
    }
    matched.sort_by(|a, b| {
        rank(a.match_kind)
            .cmp(&rank(b.match_kind))
            .then_with(|| (a.kind != ResultKind::App).cmp(&(b.kind != ResultKind::App)))
            .then_with(|| a.name.len().cmp(&b.name.len()))
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
            .then_with(|| a.id.cmp(&b.id))
    });
    results.extend(matched);
    let mut seen = std::collections::HashSet::new();
    results.retain(|result| seen.insert(format!("{:?}:{}", result.kind, result.target.to_lowercase())));
    results.truncate(limit);
    if results.is_empty() && search_web {
        results.push(LauncherResult {
            id: "web".to_string(),
            name: query.to_string(),
            subtitle: engine.host().to_string(),
            target: engine.url(query),
            kind: ResultKind::Web,
            match_kind: MatchKind::Web,
        });
    }
    results
}
fn rank(kind: MatchKind) -> u8 {
    match kind {
        MatchKind::Exact => 0,
        MatchKind::Prefix => 1,
        MatchKind::Word => 2,
        MatchKind::Contains => 3,
        MatchKind::Path => 4,
        MatchKind::Web => 5,
    }
}

fn name_match(name: &str, query: &str) -> Option<MatchKind> {
    let lower = name.to_lowercase();
    if lower == query {
        return Some(MatchKind::Exact);
    }
    if lower.starts_with(query) {
        return Some(MatchKind::Prefix);
    }
    let mut at = lower.find(query);
    while let Some(index) = at {
        if index == 0 || lower[..index].chars().next_back().is_some_and(is_separator) {
            return Some(MatchKind::Word);
        }
        at = lower[index + query.len()..].find(query).map(|next| index + query.len() + next);
    }
    lower.contains(query).then_some(MatchKind::Contains)
}

fn is_separator(ch: char) -> bool {
    ch.is_whitespace() || matches!(ch, '-' | '_' | '.' | '/' | '\\' | '(' | ')' | '{' | '}' | ',' | '、' | '，')
}

fn is_url(query: &str) -> bool {
    let lower = query.to_ascii_lowercase();
    lower.starts_with("http://") || lower.starts_with("https://")
}