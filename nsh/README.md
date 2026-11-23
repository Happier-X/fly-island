# FocuSD Island on NSH (Phase 1 spike)

This directory is the Phase 1 feasibility spike for migrating FocuSD Island
from Tauri 2 + React to NSH. It is independent of `../src` and `../src-tauri`;
the Tauri app keeps building and shipping untouched.

## What it does

- A frameless, translucent, always-on-top capsule at the top center of the
  primary display. Click it to expand the window (320×40 → 620×340) and show
  the clipboard history; click ⌃ to collapse it.
- The history is real: it polls the system clipboard through NSH's
  `clipboard_string()`, dedupes, keeps the newest 20 texts, and clicking a row
  copies it back. Images, files and persistence are not in the spike.
- A tray icon (assets/tray.svg) with a menu, and one global shortcut:
  `Ctrl+Shift+I` toggles the island.
- English and Simplified Chinese from `locales/`, switched by the EN / 中文
  button; the switch calls NSH's `set_locale`.
- The window uses `level: "aboveDock"`, `frame: "none"`, `transparent: true`,
  `taskbar: false`, `activate: false`, and a normal window kind. Overlay is
  click-through and never focusable, so it is not the island base.

## Run it

```powershell
# from the repository root, with NSH installed and on PATH
nsh run nsh
```

`nsh run` builds the Rust runner from `nsh.json` (with the locales compiled
in) and opens the capsule. `cargo run --manifest-path nsh/Cargo.toml` also
works, but shows translation keys because it does not get the generated
runner's translations.

Headless test:

```powershell
nsh test nsh
```

## Logs

The app writes its logs to the terminal that runs `nsh run`. A shortcut that
another app already holds logs `mod+shift+i is taken`.

## Known spike limits

- The window is positioned on the primary display only. It re-centers when
  displays change, but moving it to another display is not handled.
- Window resizing is an instant `setBounds`; only macOS has animated resizes.
- The clipboard poll is 800 ms and text-only; there is no
  `WM_CLIPBOARDUPDATE` listener, no image/file capture and no history
  persistence yet.
- Clicking the capsule when the window is unfocused goes through NSH's normal
  activation rules; the window is created with `activate: false` and is
  focused only by the global shortcut or the tray.
