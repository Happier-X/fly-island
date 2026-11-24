# FocuSD Island on NSH (Phase 3)

This directory is the NSH migration of FocuSD Island. It is independent of
`../src` and `../src-tauri`; the Tauri app keeps building and shipping
untouched.

**NSH version:** 0.6.1, pinned. `nsh.json` requires exactly `=0.6.1`; an
upgrade needs its own verification run. The CLI is installed at
`%LOCALAPPDATA%\nsh` (PATH is set for new terminals).

## What runs

- `src/` is the Rust app root: island capsule, clipboard history, the separate
  settings window and i18n. It starts from the Rust `App` component in
  `src/lib.rs`; there is no TSX/QuickJS entry. Tests are Rust tests
  (`cargo test --manifest-path nsh/Cargo.toml --lib`).
- `host/` is the Win32 host: a message loop, hotkeys, tray, bounds, DPI,
  tuck/fullscreen handling and focus policy around one NSH `AppView`.
- NSH's child-window `AppView` disables `transparent`, so the host uses a
  top-level `AppView` and owns its HWND. This is the accepted host shape;
  parent/child AppView is not retried.

## Run the host

```powershell
cargo run --manifest-path nsh/host/Cargo.toml
```

or run `nsh/host/target/debug/focusd-island-host.exe` after a build.
`RUST_LOG=info` prints the host probes.

- `Ctrl+Shift+I`: show/collapse the island.
- `Ctrl+Shift+O`: toggle click-through.
- `Ctrl+Shift+M`: move to the next monitor.
- `Ctrl+Shift+T`: tuck the collapsed island above the top edge; hover the
  visible strip to reveal it.
- Tray (overflow area, "焦点岛"): show/collapse, settings, click-through, tuck,
  next monitor, quit.

## Clipboard (Phase 3)

- Text and images are captured by polling plus the host's
  `WM_CLIPBOARDUPDATE` flag, so an embedded run reacts in about 100 ms.
- History and favorites are saved to
  `%APPDATA%\com.focusd.island.nsh\clipboard.json`; images are PNG files
  under `clipboard-images`.
- The first run imports the Tauri `clipboard-history.json` once (text,
  favorites and images). The old file is read-only and never deleted.
- Click an entry to copy it back. Images are written as PNG and CF_DIB through
  `arboard`, so `Clipboard.ContainsImage()` is true; Notepad's manual paste
  still needs a human check on the target build.
- `☆` / `★` toggles a favorite; clearing the history keeps favorites.
- Settings (`⚙` or the tray menu) opens a separate window: dark/light,
  Chinese/English, clipboard capture on/off, capture images, max items and
  clear history. They are saved to
  `%APPDATA%\com.focusd.island.nsh\settings.json`.

## AI status and Cindy notifications

- Read-only from the newest `%APPDATA%\Cindy\cindy-*.db`: active Codex and
  Claude Code sessions are shown as running, done or needs attention (waiting
  for an answer or an error).
- The four notification kinds mirror Tauri: done, question, scheduled/background
  and error, with a 24-hour window. A local watermark in
  `ai-watermark.json` keeps seen events out; it moves when Cindy is already in
  front or after a successful click.
- Clicking the AI strip, a session row or "Focus Cindy" brings Cindy's window to
  the front. If no window is found, the events stay unseen.
- Settings adds the master switch and the four event switches. The old
  `cindyNotificationsEnabled` / `cindyNotify*` fields are imported with the
  other legacy settings.
- Known limits: no notification sound yet and no per-session deep link (Cindy
  runs every session in one window).

## Media and peripherals

- When Windows has a media session, the expanded island shows one compact row:
  previous / play-pause / next, title and artist, and volume -/+ (10% steps).
  State is polled every 1.5 seconds and the row disappears with the session.
- Peripheral battery levels come from the same BLE Battery Service, HID Battery
  Strength and Logitech HID++ readers as Tauri. Up to four devices are listed in
  one row and refreshed every 60 seconds; the laptop battery is never read.
- Notification sounds: the three thumb-piano WAVs are copied to
  nsh/assets/sounds, scaled to 0.7 in memory and played with PlaySoundW; the
  settings page has a master switch.
- Translation runs in the expanded island's 翻译 page and in a separate
  480×640 always-on-top window; Ctrl+Shift+F or the page tab opens it. The
  clipboard fills an empty source, 500 ms auto-translate is optional, and seven
  OpenAI-compatible providers are supported. Shortcuts are recorded in Settings
  with conflict and reset handling; API keys stay in Windows Credential Manager.
  See docs/architecture/nsh-translation.md.
- The quick launcher opens with Alt+Space or two Ctrl taps, centered on the
  monitor under the pointer, and closes on Esc, blur or its shortcut. It
  searches Start Menu shortcuts, UWP apps (`Get-StartApps`), executables on
  PATH, files under Desktop/Documents/Downloads (or custom folders), and URLs;
  with no match it offers a Bing/Google/Baidu/DuckDuckGo search. Arrow keys,
  Enter and Esc work from the search field, and Settings records the
  combination or selects double-Ctrl. The file shelf is still pending.
- Lyrics come from a local .lrc folder, LRCLIB, Kugou (KRC word times and a
  translation when the source has one) and NetEase. Answers and recent misses
  are cached in APPDATA/com.focusd.island.nsh/lyrics-cache; the expanded page
  has a three-line view and per-song -0.5 s / +0.5 s offset. Settings adds the
  LRC folder, online lyrics and Chinese-source switches.
- The progress bar can be clicked or dragged: the position freezes while the
  pointer is down, the seek is sent on release, and both ends clamp.
- With no media session the expanded island keeps an idle row (record, text and
  a Play button) instead of disappearing.
- Settings -> Player order lists the sessions seen; the first playing one in
  the list is followed, then the first open one. The order is saved in
  settings.json. Media parity is complete; the equalizer is still the
  block-character level.

## WeChat reminder

- The host listens for SHELLHOOK/HSHELL_FLASH and matches only the Weixin.exe /
  WeChat.exe process. No chat content is read and no client setting is changed.
- The expanded island shows one row: disabled, listening, needs attention, or
  WeChat not running, plus "Open WeChat". The pending state clears only when the
  window was actually brought to the front.
- QQ uses the WinRT UserNotificationListener, filters AppUserModelIds to QQ and
  keeps an in-memory buffer (dedupe, 5 s merge, 20 items) with QQ.exe running
  detection, notification-settings entry and window activation. Mail uses Win32
  Credentials plus a rustls TLS IMAP client (EXAMINE, header fetch, BODY.PEEK on
  demand); valid-account mail and QQ toasts still need live integration.

## Focus policy

- Entering click-through releases keyboard focus to the last foreground
  window, or clears it if none exists.
- If an island text field has focus, entering click-through keeps that focus
  and logs `island input keeps focus`; it is never released by the periodic
  probe.
- Leaving click-through restores focusability only; it does not steal focus.

## Standalone run

```powershell
nsh run nsh
cargo test --manifest-path nsh/Cargo.toml --lib
```

The standalone app still has NSH's tray and global shortcut; the host disables
those in favor of its own.

## Rust-only checks

- No app TS/TSX/JS sources: `rg --files nsh -g '*.ts' -g '*.tsx' -g '*.js' -g '*.mjs' -g '!target/**' -g '!.nsh/**'` prints nothing.
- `cargo test --manifest-path nsh/Cargo.toml --lib` runs the Rust tests (102 passed / 9 ignored on 2026-11-24).
- `nsh build nsh -o .nsh/out` reports `no bundle: its runners mount the root`; the output holds only `app.json` and the app assets, with no JS or QuickJS bytecode.
- `nsh run nsh` reports `running FocuSD Island (NSH) (Rust, no JS runtime)`.

## Known limits

- Dual display verified 2026-11-23: 2560×1440 @125% plus 2240×1400 @150%.
  Moving, expand/collapse, topmost, click-through, tray/hotkeys, unplug/replug
  and edge straddling all re-center correctly. Evidence is in
  `docs/assets/nsh-dpi/`.
- First run imports the old Tauri settings read-only: appearance maps to the
  dark/light theme, `focusd-island-locale` maps to the language, and the old
  clipboard options (enabled/captureImages/maxItems) come from
  `clipboard-history.json`. The raw old settings are kept in
  `legacy-settings.json`; every save keeps the previous file as
  `settings.json.bak`. Field mapping: `docs/architecture/nsh-settings-map.md`.
- Settings from the old app that have no NSH equivalent yet (layout, presets,
  media, agent, shelf, translation, mail, battery) are kept in the old data but
  not applied.
- Drag works through the capsule grip; expanding or collapsing re-centers the
  window, so a dragged position is not preserved yet.
- This app does not implement todo, daily notes or the archive. Old Tauri data
  for them is not read, migrated or deleted.
