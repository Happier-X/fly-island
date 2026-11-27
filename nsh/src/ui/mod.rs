//! The island's shared visual language: tokens (`theme`), lucide icons and the
//! building blocks the screens compose. The mapping to the Tauri shell is in
//! `docs/architecture/nsh-ui-parity.md`.

// The icon set and the widget kit cover the whole shell; screens adopt them one
// page at a time, so parts stay unreferenced until their page is restyled.
#[allow(dead_code)]
pub mod icons;
#[allow(dead_code)]
pub mod theme;
#[allow(dead_code)]
pub mod widgets;
