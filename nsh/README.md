# FocuSD Island on NSH (Phase 3)

This directory is the NSH migration of FocuSD Island. It is independent of
`../src` and `../src-tauri`; the Tauri app keeps building and shipping
untouched.

**NSH version:** 0.6.1, pinned. `nsh.json` requires exactly `=0.6.1`; an
upgrade needs its own verification run. The CLI is installed at
`%LOCALAPPDATA%\nsh` (PATH is set for new terminals).

## What runs

- `src/` is the app root: island capsule, clipboard history, settings page
  and i18n. It builds standalone with `nsh run nsh`.
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
- Tray (overflow area, "焦点岛"): show/collapse, click-through, tuck, next
  monitor, quit.

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
- Settings (`⚙`) switches dark/light and Chinese/English; both are saved to
  `%APPDATA%\com.focusd.island.nsh\settings.json`.

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
nsh test nsh
```

The standalone app still has NSH's tray and global shortcut; the host disables
those in favor of its own.

## Known limits

- The current session exposes one active display (seen at 150% and 125%).
  Bounds and DPI are computed per monitor and re-applied on display changes,
  but mixed 125%/150% needs a machine with two active monitors to verify.
- The settings page is in-island; the independent settings window is Phase 3
  late work.
- Drag works through the capsule grip; expanding or collapsing re-centers the
  window, so a dragged position is not preserved yet.
- This app does not implement todo, daily notes or the archive. Old Tauri data
  for them is not read, migrated or deleted.
