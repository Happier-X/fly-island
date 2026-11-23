//! Which media session the island follows, and the user's order for it.
//!
//! Tauri keeps focusd-island-music-sources in localStorage. NSH keeps the same idea in
//! settings.json: a list of source ids and labels. The list is kept in memory for the media
//! poller; the island updates it when settings change and when a new player appears.

use std::sync::{Mutex, OnceLock};

use serde::{Deserialize, Serialize};

use super::service::MediaSessionInfo;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KnownSource {
    pub id: String,
    pub label: String,
}

static ORDER: OnceLock<Mutex<Vec<KnownSource>>> = OnceLock::new();

fn order_lock() -> &'static Mutex<Vec<KnownSource>> {
    ORDER.get_or_init(|| Mutex::new(Vec::new()))
}

/// The current order, copied for one poll or command.
pub(crate) fn order() -> Vec<KnownSource> {
    order_lock().lock().map(|order| order.clone()).unwrap_or_default()
}

/// Replaces the order with the settings' copy.
pub fn set_order(next: Vec<KnownSource>) {
    if let Ok(mut order) = order_lock().lock() {
        *order = next;
    }
}

/// The first playing source in the order, else the first source that is open; None when the
/// order knows none of the sessions, so the caller may fall back to Windows' current session.
pub(super) fn pick(sessions: &[MediaSessionInfo], order: &[KnownSource]) -> Option<String> {
    let present = |id: &str| sessions.iter().find(|session| session.id == id);
    order
        .iter()
        .find(|source| present(&source.id).is_some_and(|session| session.playing))
        .or_else(|| order.iter().find(|source| present(&source.id).is_some()))
        .map(|source| source.id.clone())
}

/// Adds newly seen sessions to the end of the order and refreshes their labels. True when the
/// order changed and should be saved.
pub fn remember(order: &mut Vec<KnownSource>, sessions: &[MediaSessionInfo]) -> bool {
    let mut changed = false;
    for session in sessions {
        match order.iter_mut().find(|source| source.id == session.id) {
            Some(source) if source.label != session.label => {
                source.label = session.label.clone();
                changed = true;
            }
            Some(_) => {}
            None => {
                order.push(KnownSource { id: session.id.clone(), label: session.label.clone() });
                changed = true;
            }
        }
    }
    changed
}

#[cfg(test)]
mod tests {
    use super::*;

    fn session(id: &str, label: &str, playing: bool) -> MediaSessionInfo {
        MediaSessionInfo { id: id.into(), label: label.into(), playing }
    }

    #[test]
    fn a_playing_source_wins_over_an_open_one() {
        let sessions = [session("a", "Player A", false), session("b", "Player B", true)];
        let order = [KnownSource { id: "a".into(), label: "Player A".into() }, KnownSource { id: "b".into(), label: "Player B".into() }];
        assert_eq!(pick(&sessions, &order), Some("b".into()));
        assert_eq!(pick(&sessions, &order[..1]), Some("a".into()));
        assert_eq!(pick(&sessions, &[]), None);
    }

    #[test]
    fn new_players_are_appended_and_labels_refresh() {
        let mut order = vec![KnownSource { id: "a".into(), label: "Old A".into() }];
        let sessions = [session("a", "Player A", false), session("b", "Player B", true)];
        assert!(remember(&mut order, &sessions));
        assert_eq!(order, vec![KnownSource { id: "a".into(), label: "Player A".into() }, KnownSource { id: "b".into(), label: "Player B".into() }]);
        assert!(!remember(&mut order, &sessions));
    }
}