# FocuSD Island on NSH (Phase 2)

This directory is the NSH migration of FocuSD Island. It is independent of
`../src` and `../src-tauri`; the Tauri app keeps building and shipping
untouched.

**NSH version:** 0.6.1, pinned. `nsh.json` requires exactly `=0.6.1`; an
upgrade needs its own verification run. The CLI is installed at
`%LOCALAPPDATA%\nsh` (PATH is set for new terminals).

## What runs

- `src/` is the app root: island capsule, clipboard history, settings page
  and i18n. It builds standalone with `nsh run nsh`.
- `host/` is the Phase 2 Win32 host: a message loop, hotkeys, tray, bounds
  and DPI handling around one NSH `AppView`.
- NSH's child-window `AppView` disables `transparent`, so the host uses a
  top-level `AppView` and owns its HWND after creation. Per-pixel alpha,
  topmost and click-through come from NSH; Win32 owns positioning, tray,
  hotkeys and probes.

## Run the host

```powershell
cargo run --manifest-path nsh/host/Cargo.toml
```

or run `nsh/host/target/debug/focusd-island-host.exe` after a build.
`RUST_LOG=info` prints the host probes.

- `Ctrl+Shift+I`: show/collapse the island.
- `Ctrl+Shift+O`: toggle click-through.
- `Ctrl+Shift+M`: move to the next monitor.
- Tray (overflow area, "焦点岛"): show/collapse, click-through, next monitor,
  quit.

## Clipboard pilot

The expanded island has a searchable history:

- Text and images are captured by polling the clipboard every 800 ms. Image
  entries live for the session; text entries are saved to
  `%APPDATA%\com.focusd.island.nsh\clipboard.json`.
- Click an entry to copy it back. `☆` / `★` toggles a favorite; clearing the
  history keeps favorites.
- Settings (`⚙`) switches dark/light and Chinese/English; both are saved to
  `%APPDATA%\com.focusd.island.nsh\settings.json`.

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
- Window resizes are immediate `SetWindowPos` calls; no animation yet.
- Clipboard capture is polling, text/image only; no `WM_CLIPBOARDUPDATE`
  listener, file clipboard or drag-out yet.
- Image history is session-only; the clipboard image URL is temporary. Images are normalised to PNG before copy-back, which writes the Windows PNG clipboard format (some legacy apps read only CF_DIB; a Win32 DIB writer is Phase 3).
- The settings page is in-island for Phase 2; the separate settings window
  migration is still open.
