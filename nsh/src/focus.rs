//! Whether an island text field has the keyboard focus.

use std::sync::atomic::{AtomicBool, Ordering};

static INPUT_FOCUSED: AtomicBool = AtomicBool::new(false);

pub fn set_input_focused(focused: bool) {
    INPUT_FOCUSED.store(focused, Ordering::Relaxed);
}

pub fn input_focused() -> bool {
    INPUT_FOCUSED.load(Ordering::Relaxed)
}
