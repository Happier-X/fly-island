//! A minimal text clipboard history: the same capture idea as
//! `src-tauri/src/clipboard_history` (poll, dedupe, cap the list), through
//! NSH's clipboard service instead of Win32 or `arboard`.

use std::{
    sync::atomic::{AtomicU64, Ordering},
    time::Duration,
};

use nsh_core::prelude::*;

const POLL_INTERVAL: Duration = Duration::from_millis(800);
const MAX_ITEMS: usize = 20;

static NEXT_ID: AtomicU64 = AtomicU64::new(1);

/// One captured clipboard text, newest first in the list.
#[derive(Debug, Clone, PartialEq)]
pub struct ClipItem {
    pub id: u64,
    pub text: String,
}

/// The history and the capture loop. `start` reads once and then polls.
pub struct ClipboardHistory {
    pub items: Signal<Vec<ClipItem>>,
}

impl ClipboardHistory {
    pub fn start() -> Self {
        let items = signal(Vec::<ClipItem>::new());
        let last = signal(String::new());
        let read = move || {
            spawn(async move {
                let text = clipboard_string().await.trim().to_string();
                if text.is_empty() || last.get() == text {
                    return;
                }
                last.set(text.clone());
                items.update(|list| {
                    list.retain(|item| item.text != text);
                    list.insert(
                        0,
                        ClipItem {
                            id: NEXT_ID.fetch_add(1, Ordering::Relaxed),
                            text: text.clone(),
                        },
                    );
                    list.truncate(MAX_ITEMS);
                });
            });
        };
        read();
        set_interval(POLL_INTERVAL, read);
        Self { items }
    }
}
