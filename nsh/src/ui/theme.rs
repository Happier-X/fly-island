//! The island's design tokens, ported from the Tauri shell.
//!
//! Every value here has one source in the Tauri app; the mapping (and what it
//! means for a screen) is written down in `docs/architecture/nsh-ui-parity.md`.
//! Restyle a screen by using these names, not by writing new literals.

use nsh_core::{prelude::*, Color};

/// The UI font stack (`--font-ui`, `src/styles/tokens.css`).
pub const FONT_UI: &str
    = "\"Segoe UI Variable Text\", \"Segoe UI\", \"Microsoft YaHei UI\", \"Microsoft YaHei\", system-ui, sans-serif";

// --- Geometry -----------------------------------------------------------------------

/// The collapsed bar's height (`.island__collapsed`, `IslandShell.css`).
pub const COLLAPSED_HEIGHT: f32 = 38.0;
/// The collapsed capsule's corner radius (`.island--collapsed`).
pub const COLLAPSED_RADIUS: f32 = 19.0;
/// The expanded card's corner radius (`--island-shape-radius`).
pub const EXPANDED_RADIUS: f32 = 30.0;
/// The page panel inside the expanded card (`--island-radius` / 1.4 with the
/// default 42 corner radius, then minus the card's 20px padding).
pub const PANEL_RADIUS: f32 = 10.0;
/// A box inside a panel, with the panel's usual 16px padding (`--island-card-radius`).
pub const CARD_RADIUS: f32 = 8.0;
/// The header strip's height (`.island__header` min-height).
pub const HEADER_HEIGHT: f32 = 34.0;
/// A round icon button (`.icon-button`).
pub const ICON_BUTTON: f32 = 34.0;
/// The page dot's hit box (`.dot-button`).
pub const DOT_WIDTH: f32 = 24.0;
/// The page dot's capsule: width, height and radius (`.dot-button::before`).
pub const DOT_CAPSULE_WIDTH: f32 = 18.0;
pub const DOT_CAPSULE_HEIGHT: f32 = 12.0;
/// The collapsed stage: 20px of padding each side, 8px between items.
pub const ISLAND_PADDING_X: f32 = 20.0;
pub const COLLAPSED_PADDING_X: f32 = 18.0;
/// The gap between the card's header, page panel and footer.
pub const ISLAND_GAP: f32 = 14.0;
/// The page panel's own padding.
pub const PANEL_PADDING: f32 = 16.0;

// Spacing scale (`--space-*`).
pub const SPACE_XS: f32 = 4.0;
pub const SPACE_SM: f32 = 6.0;
pub const SPACE_MD: f32 = 8.0;
pub const SPACE_LG: f32 = 10.0;
pub const SPACE_XL: f32 = 12.0;
pub const SPACE_2XL: f32 = 14.0;

// --- Colors -------------------------------------------------------------------------

/// One hue, four alphas: `--text-1` … `--text-placeholder`.
pub const TEXT_1: Color = Color::rgba(248, 246, 240, 245);
/// Secondary text (`--text-2`), for labels beside a value.
pub const TEXT_2: Color = Color::rgba(248, 246, 240, 173);
/// Tertiary text (`--text-3`), for hints and counters.
pub const TEXT_3: Color = Color::rgba(248, 246, 240, 138);
/// Input hints only (`--text-placeholder`).
pub const TEXT_PLACEHOLDER: Color = Color::rgba(248, 246, 240, 107);

/// The island's default fill (`islandBackgroundColor`).
pub const ISLAND_BG: Color = Color::rgba(0, 0, 0, 255);
/// `#2F2F37`, the stage's fallback background before settings load.
pub const ISLAND_BG_FALLBACK: Color = Color::rgba(47, 47, 55, 255);

/// The collapsed bar's shading, over the fill: a light top, a dark bottom.
pub const COLLAPSED_TOP: Color = Color::rgba(255, 255, 255, 15);
pub const COLLAPSED_BOTTOM: Color = Color::rgba(0, 0, 0, 56);
/// The expanded card's top light, `rgba(255, 255, 255, 0.04)` fading to nothing.
pub const CARD_TOP: Color = Color::rgba(255, 255, 255, 10);
/// The expanded card's 1px rim (`--island-rim`).
pub const RIM: Color = Color::rgba(255, 255, 255, 20);

/// `--color-accent`.
pub const ACCENT: Color = Color::rgba(116, 214, 255, 255);
/// `--color-sky`.
pub const SKY: Color = Color::rgba(168, 216, 234, 255);
/// `--color-success`.
pub const SUCCESS: Color = Color::rgba(47, 140, 97, 255);
/// `--color-danger`.
pub const DANGER: Color = Color::rgba(199, 80, 101, 255);
/// `--color-warning`: the attention colour.
pub const WARNING: Color = Color::rgba(255, 209, 102, 255);
/// `--color-agent-idle`.
pub const AGENT_IDLE: Color = Color::rgba(53, 233, 133, 255);
/// `--color-focus-ring-soft`.
pub const FOCUS_RING: Color = Color::rgba(143, 216, 255, 184);
/// The default pulse dot (`pulseColor`).
pub const PULSE: Color = Color::rgba(58, 228, 137, 255);
/// The pulse dot's glow, `pulseColor` at 72%.
pub const PULSE_GLOW: Color = Color::rgba(58, 228, 137, 184);

/// Translucent white overlays (`--color-white-aNN`).
pub const WHITE_A04: Color = Color::rgba(255, 255, 255, 10);
pub const WHITE_A06: Color = Color::rgba(255, 255, 255, 15);
pub const WHITE_A08: Color = Color::rgba(255, 255, 255, 20);
pub const WHITE_A09: Color = Color::rgba(255, 255, 255, 23);
pub const WHITE_A10: Color = Color::rgba(255, 255, 255, 26);
pub const WHITE_A12: Color = Color::rgba(255, 255, 255, 31);
pub const WHITE_A16: Color = Color::rgba(255, 255, 255, 41);
pub const WHITE_A18: Color = Color::rgba(255, 255, 255, 46);

/// The page dots' colors (`IslandHeader.css`, one capsule per page).
pub const DOT_TODO: Color = ACCENT;
pub const DOT_MUSIC: Color = Color::rgba(182, 156, 255, 255);
pub const DOT_CLIPBOARD: Color = Color::rgba(244, 167, 196, 255);
pub const DOT_SHELF: Color = Color::rgba(216, 214, 207, 255);
pub const DOT_TRANSLATION: Color = Color::rgba(255, 154, 118, 255);
pub const DOT_BATTERY: Color = Color::rgba(154, 226, 189, 255);

// --- Type ---------------------------------------------------------------------------

/// The collapsed brand (`fly-island`): 15px bold.
pub const BRAND_SIZE: f32 = 15.0;
/// The collapsed summary: 13px semibold.
pub const SUMMARY_SIZE: f32 = 13.0;
/// Body text inside the pages.
pub const BODY_SIZE: f32 = 13.0;
/// Secondary text inside the pages.
pub const LABEL_SIZE: f32 = 12.0;
/// Hints, counters and badges.
pub const HINT_SIZE: f32 = 11.0;

// --- Materials ----------------------------------------------------------------------

/// The expanded card's shadow: `0 18px 46px rgba(0, 0, 0, 0.24)` at the default
/// glass strength, plus the rim drawn as an inset ring.
pub fn card_shadow() -> Vec<BoxShadow> {
    vec![BoxShadow {
        offset_x: 0.0.into(),
        offset_y: 18.0.into(),
        blur: 46.0.into(),
        spread: 0.0.into(),
        color: Color::rgba(0, 0, 0, 61),
        inset: false,
    }]
}

/// The collapsed capsule's inset shading and its soft drop shadow.
pub fn collapsed_shadow(focused: bool) -> Vec<BoxShadow> {
    let mut layers = vec![BoxShadow {
        offset_x: 0.0.into(),
        offset_y: 5.0.into(),
        blur: 16.0.into(),
        spread: 0.0.into(),
        color: Color::rgba(0, 0, 0, 31),
        inset: false,
    }];
    // Lit only for keyboard focus, as `.island--collapsed:has(:focus-visible)`.
    layers.push(BoxShadow {
        offset_x: 0.0.into(),
        offset_y: px(-1.0),
        blur: 0.0.into(),
        spread: 0.0.into(),
        color: if focused { Color::rgba(255, 255, 255, 66) } else { Color::TRANSPARENT },
        inset: true,
    });
    layers.push(BoxShadow {
        offset_x: 0.0.into(),
        offset_y: 0.0.into(),
        blur: 18.0.into(),
        spread: 0.0.into(),
        color: WHITE_A08,
        inset: true,
    });
    layers
}

/// A colored glow behind a dot or a ring: `box-shadow: 0 0 <blur> <color>`.
pub fn soft_glow(color: Color, blur: f32) -> Vec<BoxShadow> {
    vec![BoxShadow {
        offset_x: 0.0.into(),
        offset_y: 0.0.into(),
        blur: blur.into(),
        spread: 0.0.into(),
        color,
        inset: false,
    }]
}

/// The glow behind a page dot, `color` at 46%.
pub fn dot_glow(color: Color) -> Vec<BoxShadow> {
    soft_glow(alpha(color, 118), 12.0)
}

/// `base` with its alpha replaced, for hover and pressed states.
pub fn alpha(base: Color, alpha: u8) -> Color {
    let (r, g, b, _) = base.to_f64();
    Color::rgba((r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8, alpha)
}
