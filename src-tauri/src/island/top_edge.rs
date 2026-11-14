//! Top-edge snapping and the legacy flush-attachment event. The shell remains rounded;
//! snapping uses the same work-area margin as default placement and reset.

use tauri::{Emitter, PhysicalPosition, Position, WebviewWindow};

use super::geometry::{monitor_for_island_position, work_area_top_inset};
use super::state::{mutate_window_state, read_window_state, IslandPosition, IslandWindowState};

// Mirrored by TOP_ATTACHMENT_CHANGED_EVENT in src/windows/island/api.ts.
pub(crate) const TOP_ATTACHMENT_CHANGED_EVENT: &str = "island-top-attachment-changed";
// Dropping this close to the default top position (DIP) restores its margin.
const SNAP_DISTANCE: f64 = 16.0;

pub(crate) fn is_attached() -> bool {
    read_window_state().top_attached
}

/// Recomputes the attachment once the stage geometry has resolved the window position.
/// `top` is the physical y of the monitor top, below a top-docked taskbar.
pub(crate) fn sync_from_stage(window: &WebviewWindow, state: IslandWindowState, top: i32) {
    set_attached(
        window,
        is_top_attached(state.custom_position, state.margin_y, top),
    );
}

/// Pulls a dropped island to its normal top margin when it lands close to it.
pub(crate) fn snap_to_top(window: &WebviewWindow, position: IslandPosition) -> IslandPosition {
    let Ok(Some(monitor)) = monitor_for_island_position(window, position) else {
        set_attached(window, false);
        return position;
    };
    let monitor_top = monitor.position().y;
    let top = monitor_top + work_area_top_inset(monitor_top, monitor.work_area().position.y);
    let snapped = IslandPosition {
        x: position.x,
        y: snap_y_at_scale(position.y, top, read_window_state().margin_y, monitor.scale_factor()),
    };

    if snapped != position {
        let target = Position::Physical(PhysicalPosition::new(snapped.x, snapped.y));
        if let Err(error) = window.set_position(target) {
            eprintln!("Failed to snap island to the top edge: {error}");
            set_attached(window, position.y == top);
            return position;
        }
    }

    set_attached(window, snapped.y == top);
    snapped
}

pub(crate) fn set_attached(window: &WebviewWindow, attached: bool) {
    let mut changed = false;
    mutate_window_state(|state| {
        changed = state.top_attached != attached;
        state.top_attached = attached;
        *state
    });

    if changed {
        let _ = window.emit(TOP_ATTACHMENT_CHANGED_EVENT, attached);
    }
}

// A custom position ignores the vertical margin, so only its own y decides.
fn is_top_attached(custom_position: Option<IslandPosition>, margin_y: f64, top: i32) -> bool {
    match custom_position {
        Some(position) => position.y == top,
        None => margin_y <= 0.0,
    }
}

fn snap_y_at_scale(y: i32, top: i32, margin: f64, scale: f64) -> i32 {
    let target = top + (margin * scale).round() as i32;
    snapped_y(y, target, (SNAP_DISTANCE * scale).round() as i32)
}

fn snapped_y(y: i32, top: i32, snap_distance: i32) -> i32 {
    if (y - top).abs() <= snap_distance {
        top
    } else {
        y
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_position_is_attached_only_without_a_margin() {
        assert!(is_top_attached(None, 0.0, 0));
        assert!(is_top_attached(None, 0.0, 48));
        assert!(!is_top_attached(None, 12.0, 0));
    }

    #[test]
    fn custom_position_is_attached_only_when_flush_with_the_top() {
        let at = |y| Some(IslandPosition { x: 420, y });
        assert!(is_top_attached(at(0), 0.0, 0));
        assert!(is_top_attached(at(0), 40.0, 0));
        assert!(is_top_attached(at(-1080), 0.0, -1080));
        assert!(is_top_attached(at(48), 0.0, 48));
        assert!(!is_top_attached(at(1), 0.0, 0));
        assert!(!is_top_attached(at(180), 0.0, 0));
        assert!(!is_top_attached(at(0), 0.0, 48));
    }

    #[test]
    fn snapping_uses_the_same_dip_margin_on_every_monitor() {
        for scale in [1.0, 1.25, 1.5, 2.0] {
            for top in [0, 48, -1440, -1392] {
                let target = top + (10.0_f64 * scale).round() as i32;
                let threshold = (SNAP_DISTANCE * scale).round() as i32;
                assert_eq!(snap_y_at_scale(top, top, 10.0, scale), target);
                assert_eq!(snap_y_at_scale(target, top, 10.0, scale), target);
                assert_eq!(snap_y_at_scale(target + threshold, top, 10.0, scale), target);
                let outside = target + threshold + 1;
                assert_eq!(snap_y_at_scale(outside, top, 10.0, scale), outside);
            }
        }
    }

    #[test]
    fn drops_near_the_target_snap_and_others_stay() {
        assert_eq!(snapped_y(0, 0, 16), 0);
        assert_eq!(snapped_y(16, 0, 16), 0);
        assert_eq!(snapped_y(-16, 0, 16), 0);
        assert_eq!(snapped_y(17, 0, 16), 17);
        assert_eq!(snapped_y(-17, 0, 16), -17);
        assert_eq!(snapped_y(-1070, -1080, 24), -1080);
        assert_eq!(snapped_y(60, 48, 24), 48);
        assert_eq!(snapped_y(300, 48, 24), 300);
    }
}
